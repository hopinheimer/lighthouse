//! Property-based fuzzer for optimistic sync in fork choice.
//!
//! A scenario is a list of `OptSyncOperation`s: blocks are imported, envelopes revealed, and the
//! execution layer (EL) validates or invalidates payloads in some order. `run` executes the steps
//! and checks the optimistic sync properties after every step.
//!
//! Every scenario runs in one `ForkMode`: pre-Gloas blocks only, Gloas blocks only, or a chain
//! that starts pre-Gloas and crosses the Gloas fork. A pre-Gloas block carries its payload, so it
//! is executed on import and has a single node. A Gloas block's payload arrives in an envelope
//! and is executed only by its `FULL` node.
//!
//! The EL is simulated from ground truth: every block carries a hidden `payload_valid` bit, and a
//! payload is truly valid iff its own bit is set and every payload in its executed ancestry is
//! truly valid. The EL never contradicts that truth, so any property violation is a fork choice
//! bug rather than a confused EL.
//!
//! Operation fields that name a block are indices into the blocks eligible for that operation
//! (taken modulo their count). Every input is therefore a valid scenario, which keeps shrinking
//! effective. An operation with no eligible block is a no-op.

use crate::proto_array::{ParentPayloadStatus, ProtoNode};
use crate::proto_array_fork_choice::{
    Block, ExecutionStatus, ExecutionVerdict, PayloadStatus, ProtoArrayForkChoice,
};
use crate::{InvalidationOperation, JustifiedBalances};
use fixed_bytes::FixedBytesExtended;
use proptest::prelude::*;
use std::collections::BTreeSet;
use std::time::Duration;
use types::{
    AttestationShufflingId, ChainSpec, Checkpoint, Epoch, EthSpec, ExecutionBlockHash, Hash256,
    MainnetEthSpec, Slot,
};

type E = MainnetEthSpec;

const NUM_VALIDATORS: usize = 8;
const MAX_BLOCKS: usize = 64;
const ANCHOR: usize = 0;

/// Which blocks a scenario builds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ForkMode {
    PreGloas,
    Gloas,
    /// Gloas from epoch 1, with the anchor a few slots before the fork.
    Boundary,
}

impl ForkMode {
    fn spec(self) -> ChainSpec {
        let mut spec = E::default_spec();
        spec.proposer_score_boost = 50;
        spec.gloas_fork_epoch = match self {
            Self::PreGloas => None,
            Self::Gloas => Some(Epoch::new(0)),
            Self::Boundary => Some(Epoch::new(1)),
        };
        spec
    }

    fn anchor_slot(self) -> Slot {
        match self {
            Self::PreGloas | Self::Gloas => Slot::new(0),
            Self::Boundary => Slot::new(E::slots_per_epoch() - 4),
        }
    }
}

/// One step of an optimistic sync scenario.
#[derive(Clone, Debug)]
pub enum OptSyncOperation {
    /// Import a block on the block `parent_offset` places back from the newest one.
    ///
    /// The block extends its parent's `FULL` node when `on_full` is set and the parent's payload
    /// has been revealed, otherwise its `EMPTY` node. `payload_valid` is the ground truth the EL
    /// will report for this block's own payload.
    AddBlock {
        parent_offset: u8,
        on_full: bool,
        payload_valid: bool,
        slot_skip: u8,
    },
    /// The envelope of a block whose envelope has not arrived yet.
    ///
    /// With `el_judges` the EL answers `newPayload` from ground truth: `VALID` is recorded with
    /// the envelope, `INVALID` rejects the envelope and reports the latest valid ancestor.
    /// Otherwise the EL is `SYNCING` and the payload is recorded optimistically.
    RevealPayload {
        block: u8,
        el_judges: bool,
    },
    /// The EL reports `VALID` for an executed, truly valid payload (e.g. via forkchoiceUpdated).
    ValidatePayload {
        block: u8,
    },
    /// The EL reports `INVALID` for an executed, truly invalid payload.
    InvalidatePayload {
        block: u8,
        kind: InvalidationKind,
    },
    /// A validator votes for a block.
    Vote {
        validator: u8,
        block: u8,
        payload_present: bool,
    },
    AdvanceSlot,
    FindHead,
    /// Replace fork choice with a copy decoded from its own SSZ bytes, as a restart would.
    ReloadFromBytes,
}

#[derive(Clone, Copy, Debug)]
pub enum InvalidationKind {
    /// `InvalidateOne` on the payload.
    One,
    /// `InvalidateMany` with the true latest valid ancestor.
    ManyExactLatestValidAncestor,
    /// `InvalidateMany` with a latest valid ancestor unknown to fork choice.
    ManyUnknownLatestValidAncestor,
}

/// How often each kind of event happened, so tests can check the fuzzer is not vacuous.
#[derive(Debug, Default, Clone, Copy)]
pub struct Stats {
    pub blocks_imported: usize,
    pub blocks_refused: usize,
    pub payloads_revealed: usize,
    pub payloads_rejected_on_import: usize,
    pub validations: usize,
    pub invalidations: usize,
    pub pre_gloas_blocks: usize,
    pub gloas_blocks: usize,
    /// Gloas blocks imported on a pre-Gloas parent.
    pub fork_crossings: usize,
    pub head_checks: usize,
    pub full_heads: usize,
    /// Invalidations whose deepest condemned node lies below the reported head, i.e. the walk
    /// down to the latest valid ancestor did real work.
    pub deep_invalidations: usize,
    pub max_invalid_nodes: usize,
}

/// A block as the oracle sees it.
struct ModelBlock {
    root: Hash256,
    block_hash: ExecutionBlockHash,
    slot: Slot,
    /// A pre-Gloas block, whose payload is executed on import and which has a single node.
    pre_gloas: bool,
    /// The beacon block this block extends. `None` for the anchor.
    parent: Option<usize>,
    /// The block whose payload this block builds on: a pre-Gloas block's beacon parent, a Gloas
    /// block's bid parent. `None` is the pre-anchor payload, which is valid by assumption.
    execution_parent: Option<usize>,
    execution_parent_hash: ExecutionBlockHash,
    /// Ground truth for this block's own payload, ignoring its ancestry.
    payload_valid: bool,
    /// The envelope was handed to fork choice or rejected by the EL. Never delivered twice.
    envelope_processed: bool,
    /// Fork choice recorded the envelope (`payload_received`).
    revealed: bool,
}

impl ModelBlock {
    /// Fork choice has run this block's own payload.
    fn executed(&self) -> bool {
        self.pre_gloas || self.revealed
    }
}

/// `get_head`'s ordering of sibling nodes: weight, then root, then payload status tiebreaker.
type HeadKey = (u64, Hash256, u8);

/// What fork choice reports about one block, captured after each step.
#[derive(Clone, Copy, Debug)]
struct Snapshot {
    stored: ExecutionStatus,
    empty: ExecutionVerdict,
    full: ExecutionVerdict,
}

struct Runner {
    fork_choice: ProtoArrayForkChoice,
    spec: ChainSpec,
    blocks: Vec<ModelBlock>,
    snapshots: Vec<Snapshot>,
    current_slot: Slot,
    /// Each validator's latest message as fork choice should have accepted it:
    /// `(block, attestation slot, payload_present)`.
    votes: Vec<Option<(usize, Slot, bool)>>,
    stats: Stats,
}

fn root(index: usize) -> Hash256 {
    Hash256::from_low_u64_be(index as u64 + 1)
}

fn block_hash(index: usize) -> ExecutionBlockHash {
    ExecutionBlockHash::from_root(Hash256::from_low_u64_be(index as u64 + 1_000_000))
}

/// The payload the anchor's state was built on. Not known to fork choice.
fn pre_anchor_hash() -> ExecutionBlockHash {
    ExecutionBlockHash::from_root(Hash256::repeat_byte(0xaa))
}

/// A hash no block commits to.
fn junk_hash() -> ExecutionBlockHash {
    ExecutionBlockHash::from_root(Hash256::repeat_byte(0xbb))
}

fn anchor_checkpoint() -> Checkpoint {
    Checkpoint {
        epoch: Epoch::new(0),
        root: root(ANCHOR),
    }
}

/// Pick the `choice`-th element of `eligible`, wrapping around.
fn pick(eligible: &[usize], choice: u8) -> Option<usize> {
    if eligible.is_empty() {
        None
    } else {
        eligible.get(choice as usize % eligible.len()).copied()
    }
}

macro_rules! ensure {
    ($cond:expr, $($arg:tt)*) => {
        if !$cond {
            return Err(format!($($arg)*));
        }
    };
}

/// Run a scenario, checking the optimistic sync properties after every step.
pub fn run(fork_mode: ForkMode, operations: &[OptSyncOperation]) -> Result<Stats, String> {
    run_scenario(fork_mode, operations).map(|runner| runner.stats)
}

fn run_scenario(fork_mode: ForkMode, operations: &[OptSyncOperation]) -> Result<Runner, String> {
    let mut runner = Runner::new(fork_mode)?;
    for (index, operation) in operations.iter().enumerate() {
        runner
            .apply(operation)
            .and_then(|()| runner.check_properties())
            .map_err(|e| format!("step {index} ({operation:?}): {e}"))?;
    }
    runner
        .find_head()
        .and_then(|()| runner.check_properties())
        .map_err(|e| format!("final head check: {e}"))?;
    Ok(runner)
}

impl Runner {
    fn new(fork_mode: ForkMode) -> Result<Self, String> {
        let spec = fork_mode.spec();
        let anchor_slot = fork_mode.anchor_slot();
        let shuffling_id = AttestationShufflingId::from_components(Epoch::new(0), Hash256::zero());
        let fork_choice = ProtoArrayForkChoice::new::<E>(
            anchor_slot,
            anchor_slot,
            Hash256::zero(),
            anchor_checkpoint(),
            anchor_checkpoint(),
            shuffling_id.clone(),
            shuffling_id,
            ExecutionStatus::Optimistic(block_hash(ANCHOR)),
            Some(pre_anchor_hash()),
            Some(block_hash(ANCHOR)),
            0,
            &spec,
        )?;
        let anchor = ModelBlock {
            root: root(ANCHOR),
            block_hash: block_hash(ANCHOR),
            slot: anchor_slot,
            pre_gloas: !spec.fork_name_at_slot::<E>(anchor_slot).gloas_enabled(),
            parent: None,
            execution_parent: None,
            execution_parent_hash: pre_anchor_hash(),
            payload_valid: true,
            // A Gloas anchor's envelope is never received.
            envelope_processed: true,
            revealed: false,
        };
        let mut runner = Self {
            fork_choice,
            spec,
            blocks: vec![anchor],
            snapshots: vec![],
            current_slot: anchor_slot,
            votes: vec![None; NUM_VALIDATORS],
            stats: Stats::default(),
        };
        runner.check_properties()?;
        Ok(runner)
    }

    /// Ground truth: is the payload of `index` and its whole executed ancestry valid?
    fn truly_valid(&self, index: usize) -> bool {
        let mut current = Some(index);
        while let Some(index) = current {
            let Some(block) = self.blocks.get(index) else {
                return false;
            };
            if !block.payload_valid {
                return false;
            }
            current = block.execution_parent;
        }
        true
    }

    /// Ground truth: is the payload that `execution_parent` names valid?
    fn execution_parent_truly_valid(&self, execution_parent: Option<usize>) -> bool {
        execution_parent.is_none_or(|index| self.truly_valid(index))
    }

    /// The hash the EL reports as the latest valid ancestor of payload `start`, inclusive.
    fn latest_valid_hash(&self, start: Option<usize>) -> ExecutionBlockHash {
        let mut current = start;
        while let Some(index) = current {
            let Some(block) = self.blocks.get(index) else {
                break;
            };
            if self.truly_valid(index) {
                return block.block_hash;
            }
            current = block.execution_parent;
        }
        pre_anchor_hash()
    }

    /// The payload a fork choice node ends on: a pre-Gloas block's own, the `FULL` node of a
    /// revealed Gloas block its own, and every other Gloas node its execution parent's.
    fn executed_payload(&self, index: usize, payload_status: PayloadStatus) -> Option<usize> {
        let block = self.blocks.get(index)?;
        if block.pre_gloas || (payload_status == PayloadStatus::Full && block.revealed) {
            Some(index)
        } else {
            block.execution_parent
        }
    }

    /// Ground truth for the verdict of a fork choice node.
    fn node_truly_valid(&self, index: usize, payload_status: PayloadStatus) -> bool {
        self.execution_parent_truly_valid(self.executed_payload(index, payload_status))
    }

    fn node(&self, index: usize) -> Result<&ProtoNode, String> {
        let root = self.blocks.get(index).ok_or("unknown model block")?.root;
        self.fork_choice
            .proto_array
            .get_block(root)
            .ok_or_else(|| format!("block {index} missing from fork choice"))
    }

    fn verdict(
        &self,
        index: usize,
        payload_status: PayloadStatus,
    ) -> Result<ExecutionVerdict, String> {
        let root = self.blocks.get(index).ok_or("unknown model block")?.root;
        self.fork_choice
            .proto_array
            .node_execution_status(root, payload_status)
            .map_err(|e| format!("verdict of ({index}, {payload_status:?}) failed: {e:?}"))
    }

    fn apply(&mut self, operation: &OptSyncOperation) -> Result<(), String> {
        match *operation {
            OptSyncOperation::AddBlock {
                parent_offset,
                on_full,
                payload_valid,
                slot_skip,
            } => self.add_block(parent_offset, on_full, payload_valid, slot_skip),
            OptSyncOperation::RevealPayload { block, el_judges } => {
                self.reveal_payload(block, el_judges)
            }
            OptSyncOperation::ValidatePayload { block } => self.validate_payload(block),
            OptSyncOperation::InvalidatePayload { block, kind } => {
                self.invalidate_payload(block, kind)
            }
            OptSyncOperation::Vote {
                validator,
                block,
                payload_present,
            } => {
                let eligible = (0..self.blocks.len()).collect::<Vec<_>>();
                let Some(index) = pick(&eligible, block) else {
                    return Ok(());
                };
                let block_root = self.blocks.get(index).ok_or("unknown model block")?.root;
                let validator = validator as usize % NUM_VALIDATORS;
                // Fork choice keeps the first message of a slot and replaces it only with a
                // message from a later slot.
                let vote = self.votes.get_mut(validator).ok_or("unknown validator")?;
                if vote.is_none_or(|(_, slot, _)| self.current_slot > slot) {
                    *vote = Some((index, self.current_slot, payload_present));
                }
                self.fork_choice.process_attestation(
                    validator,
                    block_root,
                    self.current_slot,
                    payload_present,
                )
            }
            OptSyncOperation::AdvanceSlot => {
                self.current_slot += 1;
                Ok(())
            }
            OptSyncOperation::FindHead => self.find_head(),
            OptSyncOperation::ReloadFromBytes => {
                let reloaded = ProtoArrayForkChoice::from_bytes(
                    &self.fork_choice.as_bytes(),
                    self.fork_choice.balances.clone(),
                )?;
                ensure!(
                    reloaded == self.fork_choice,
                    "fork choice changed across an SSZ round trip"
                );
                self.fork_choice = reloaded;
                Ok(())
            }
        }
    }

    fn add_block(
        &mut self,
        parent_offset: u8,
        on_full: bool,
        payload_valid: bool,
        slot_skip: u8,
    ) -> Result<(), String> {
        if self.blocks.len() >= MAX_BLOCKS {
            return Ok(());
        }
        let parent = self
            .blocks
            .len()
            .saturating_sub(1)
            .saturating_sub(parent_offset as usize % self.blocks.len());
        let parent_block = self.blocks.get(parent).ok_or("unknown parent")?;
        let index = self.blocks.len();
        let slot = parent_block.slot + 1 + u64::from(slot_skip % 3);
        let pre_gloas = !self.spec.fork_name_at_slot::<E>(slot).gloas_enabled();

        // A block builds on its parent's payload when that payload is in the parent's state:
        // always for a pre-Gloas parent, and for a Gloas parent only once revealed. Otherwise a
        // Gloas block builds on its parent's `EMPTY` node.
        let (execution_parent, execution_parent_hash) =
            if parent_block.pre_gloas || (on_full && parent_block.revealed) {
                (Some(parent), parent_block.block_hash)
            } else {
                (
                    parent_block.execution_parent,
                    parent_block.execution_parent_hash,
                )
            };
        let crosses_fork = parent_block.pre_gloas && !pre_gloas;

        let parent_root = parent_block.root;
        self.current_slot = self.current_slot.max(slot);

        let execution_parent_known_invalid = execution_parent
            .map(|execution_parent| self.node(execution_parent))
            .transpose()?
            .is_some_and(|node| node.execution_status().is_invalid());

        let block = Block {
            slot,
            root: root(index),
            parent_root: Some(parent_root),
            state_root: Hash256::zero(),
            target_root: Hash256::zero(),
            current_epoch_shuffling_id: AttestationShufflingId::from_components(
                Epoch::new(0),
                Hash256::zero(),
            ),
            next_epoch_shuffling_id: AttestationShufflingId::from_components(
                Epoch::new(0),
                Hash256::zero(),
            ),
            justified_checkpoint: anchor_checkpoint(),
            finalized_checkpoint: anchor_checkpoint(),
            execution_status: ExecutionStatus::Optimistic(block_hash(index)),
            unrealized_justified_checkpoint: None,
            unrealized_finalized_checkpoint: None,
            // A pre-Gloas block has no bid.
            execution_payload_parent_hash: (!pre_gloas).then_some(execution_parent_hash),
            execution_payload_block_hash: (!pre_gloas).then_some(block_hash(index)),
            proposer_index: Some(0),
            payload_received: false,
        };
        let result = self.fork_choice.process_block::<E>(
            block,
            self.current_slot,
            &self.spec,
            Duration::ZERO,
        );

        match result {
            Ok(()) => {
                ensure!(
                    !execution_parent_known_invalid,
                    "block {index} built on a payload fork choice knows is invalid was accepted"
                );
                self.blocks.push(ModelBlock {
                    root: root(index),
                    block_hash: block_hash(index),
                    slot,
                    pre_gloas,
                    parent: Some(parent),
                    execution_parent,
                    execution_parent_hash,
                    payload_valid,
                    // A pre-Gloas block has no envelope.
                    envelope_processed: pre_gloas,
                    revealed: false,
                });
                self.stats.blocks_imported += 1;
                if pre_gloas {
                    self.stats.pre_gloas_blocks += 1;
                } else {
                    self.stats.gloas_blocks += 1;
                }
                if crosses_fork {
                    self.stats.fork_crossings += 1;
                }
                Ok(())
            }
            Err(e) => {
                ensure!(
                    execution_parent_known_invalid && e.contains("ParentExecutionStatusIsInvalid"),
                    "block {index} refused unexpectedly: {e}"
                );
                ensure!(
                    !self.execution_parent_truly_valid(execution_parent),
                    "block {index} refused although its execution parent is truly valid"
                );
                self.stats.blocks_refused += 1;
                Ok(())
            }
        }
    }

    fn reveal_payload(&mut self, choice: u8, el_judges: bool) -> Result<(), String> {
        let eligible = self
            .blocks
            .iter()
            .enumerate()
            .filter(|(_, block)| !block.envelope_processed)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let Some(index) = pick(&eligible, choice) else {
            return Ok(());
        };
        let truly_valid = self.truly_valid(index);
        let block = self.blocks.get_mut(index).ok_or("unknown model block")?;
        block.envelope_processed = true;
        let (block_root, hash) = (block.root, block.block_hash);
        let (execution_parent, execution_parent_hash) =
            (block.execution_parent, block.execution_parent_hash);

        if el_judges && !truly_valid {
            // `newPayload` INVALID: the envelope is rejected, and the EL condemns everything
            // between the payload it builds on and the latest valid ancestor.
            let latest_valid_ancestor = self.latest_valid_hash(execution_parent);
            self.stats.payloads_rejected_on_import += 1;
            let expected = match execution_parent {
                Some(head) => {
                    self.expected_deepest_condemned(head, latest_valid_ancestor, false)?
                }
                // The payload it builds on predates fork choice: nothing to condemn.
                None => None,
            };
            self.fork_choice
                .process_execution_payload_invalidation::<E>(
                    &InvalidationOperation::InvalidateMany {
                        head_hash: execution_parent_hash,
                        always_invalidate_head: false,
                        latest_valid_ancestor,
                    },
                    anchor_checkpoint(),
                )?;
            return self.check_condemned(execution_parent, expected);
        }

        let status = if el_judges {
            ExecutionStatus::Valid(hash)
        } else {
            ExecutionStatus::Optimistic(hash)
        };
        self.fork_choice
            .on_payload_envelope_received(block_root, status)?;
        if let Some(block) = self.blocks.get_mut(index) {
            block.revealed = true;
        }
        self.stats.payloads_revealed += 1;
        if el_judges {
            self.check_validated(index)?;
        }
        Ok(())
    }

    /// The executed payloads from `start` down to the anchor's execution parent, `start` first.
    fn execution_chain(&self, start: Option<usize>) -> Vec<usize> {
        let mut chain = vec![];
        let mut current = start;
        while let Some(index) = current {
            chain.push(index);
            current = self
                .blocks
                .get(index)
                .and_then(|block| block.execution_parent);
        }
        chain
    }

    fn index_of_hash(&self, hash: ExecutionBlockHash) -> Option<usize> {
        self.blocks
            .iter()
            .position(|block| block.block_hash == hash)
    }

    /// Completeness after a VALID verdict for the payload of `index`: it and every payload its
    /// branch executed are stored `Valid`.
    fn check_validated(&self, index: usize) -> Result<(), String> {
        for executed in self.execution_chain(Some(index)) {
            let stored = self.node(executed)?.execution_status();
            ensure!(
                matches!(stored, ExecutionStatus::Valid(_)),
                "VALID for block {index} left executed ancestor {executed} as {stored:?}"
            );
        }
        Ok(())
    }

    /// `Pn`, the deepest node an invalidation of the payload of `head` should condemn, derived
    /// from the model's execution chain alone.
    ///
    /// A latest valid ancestor known to fork choice condemns the executed payloads above it, so
    /// `Pn` is the one built directly on it. An unknown one condemns the head alone when the head
    /// itself was judged, and nothing otherwise.
    fn expected_deepest_condemned(
        &self,
        head: usize,
        latest_valid_ancestor: ExecutionBlockHash,
        invalidate_head: bool,
    ) -> Result<Option<usize>, String> {
        match self.index_of_hash(latest_valid_ancestor) {
            Some(latest_valid) if latest_valid == head => Ok(None),
            Some(latest_valid) => self
                .execution_chain(Some(head))
                .into_iter()
                .find(|&index| {
                    self.blocks
                        .get(index)
                        .is_some_and(|block| block.execution_parent == Some(latest_valid))
                })
                .map(Some)
                .ok_or_else(|| {
                    format!(
                        "latest valid ancestor {latest_valid} is not in the execution chain of {head}"
                    )
                }),
            None => Ok(invalidate_head.then_some(head)),
        }
    }

    /// Completeness and precision after an invalidation: exactly `Pn` and every block whose
    /// execution chain runs through `Pn` become invalid; nothing else changes its invalidity.
    fn check_condemned(
        &mut self,
        head: Option<usize>,
        deepest: Option<usize>,
    ) -> Result<(), String> {
        if deepest.is_some_and(|deepest| Some(deepest) != head) {
            self.stats.deep_invalidations += 1;
        }
        for index in 0..self.blocks.len() {
            let stored = self.node(index)?.execution_status();
            let condemned =
                deepest.is_some_and(|deepest| self.execution_chain(Some(index)).contains(&deepest));
            let was_invalid = self
                .snapshots
                .get(index)
                .ok_or("missing snapshot before invalidation")?
                .stored
                .is_invalid();
            ensure!(
                stored.is_invalid() == (condemned || was_invalid),
                "invalidation with deepest condemned node {deepest:?}: block {index} is \
                 {stored:?} (condemned: {condemned}, previously invalid: {was_invalid})"
            );
        }
        Ok(())
    }

    fn validate_payload(&mut self, choice: u8) -> Result<(), String> {
        let eligible = (0..self.blocks.len())
            .filter(|&index| {
                self.blocks.get(index).is_some_and(ModelBlock::executed) && self.truly_valid(index)
            })
            .collect::<Vec<_>>();
        let Some(index) = pick(&eligible, choice) else {
            return Ok(());
        };
        let hash = self
            .blocks
            .get(index)
            .ok_or("unknown model block")?
            .block_hash;
        self.stats.validations += 1;
        self.fork_choice
            .process_execution_payload_validation(hash)?;
        self.check_validated(index)
    }

    fn invalidate_payload(&mut self, choice: u8, kind: InvalidationKind) -> Result<(), String> {
        let eligible = (0..self.blocks.len())
            .filter(|&index| {
                self.blocks.get(index).is_some_and(ModelBlock::executed) && !self.truly_valid(index)
            })
            .collect::<Vec<_>>();
        let Some(index) = pick(&eligible, choice) else {
            return Ok(());
        };
        let block = self.blocks.get(index).ok_or("unknown model block")?;
        let head_hash = block.block_hash;
        let (operation, deepest) = match kind {
            // No latest valid ancestor: the head alone.
            InvalidationKind::One => (
                InvalidationOperation::InvalidateOne { head_hash },
                Some(index),
            ),
            InvalidationKind::ManyExactLatestValidAncestor => {
                let latest_valid_ancestor = self.latest_valid_hash(block.execution_parent);
                (
                    InvalidationOperation::InvalidateMany {
                        head_hash,
                        always_invalidate_head: true,
                        latest_valid_ancestor,
                    },
                    self.expected_deepest_condemned(index, latest_valid_ancestor, true)?,
                )
            }
            // Adversarial: with an unpruned, fully known chain a real EL cannot name a latest
            // valid hash fork choice has never seen. It exercises the head-only fallback.
            InvalidationKind::ManyUnknownLatestValidAncestor => (
                InvalidationOperation::InvalidateMany {
                    head_hash,
                    always_invalidate_head: true,
                    latest_valid_ancestor: junk_hash(),
                },
                Some(index),
            ),
        };
        self.stats.invalidations += 1;
        self.fork_choice
            .process_execution_payload_invalidation::<E>(&operation, anchor_checkpoint())?;
        self.check_condemned(Some(index), deepest)
    }

    /// Property: the head is never invalid, and only names a `FULL` node whose payload arrived.
    fn find_head(&mut self) -> Result<(), String> {
        let balances = JustifiedBalances::from_effective_balances(vec![1; NUM_VALIDATORS])
            .map_err(|e| format!("{e:?}"))?;
        let head = self.fork_choice.find_head::<E>(
            anchor_checkpoint(),
            anchor_checkpoint(),
            &balances,
            Hash256::zero(),
            &BTreeSet::new(),
            self.current_slot,
            &self.spec,
        )?;
        let verdict = self
            .fork_choice
            .get_node_execution_status(head)
            .map_err(|e| format!("head verdict failed: {e:?}"))?;
        ensure!(!verdict.is_invalid(), "head {head:?} is invalid");
        if head.payload_status() == PayloadStatus::Full {
            ensure!(
                self.fork_choice.is_payload_received(&head.root()),
                "head {head:?} is a FULL node without a received payload"
            );
            self.stats.full_heads += 1;
        }
        let (expected_index, expected_status) = self.model_head()?;
        let expected_root = self
            .blocks
            .get(expected_index)
            .ok_or("unknown model block")?
            .root;
        ensure!(
            head.root() == expected_root && head.payload_status() == expected_status,
            "head {head:?} differs from the model head ({expected_index}, {expected_status:?})"
        );
        self.stats.head_checks += 1;
        Ok(())
    }

    fn stored_is_invalid(&self, index: usize) -> Result<bool, String> {
        Ok(self.node(index)?.execution_status().is_invalid())
    }

    /// The edge from `index` to its parent as fork choice records it: `PreGloas` when either
    /// end is pre-Gloas, otherwise the parent's `FULL` or `EMPTY` node.
    fn parent_edge(&self, index: usize) -> Result<ParentPayloadStatus, String> {
        let block = self.blocks.get(index).ok_or("unknown model block")?;
        let parent = block.parent.ok_or("the anchor has no parent")?;
        let parent_block = self.blocks.get(parent).ok_or("unknown model block")?;
        Ok(if block.pre_gloas || parent_block.pre_gloas {
            ParentPayloadStatus::PreGloas
        } else if block.execution_parent == Some(parent) {
            ParentPayloadStatus::Full
        } else {
            ParentPayloadStatus::Empty
        })
    }

    /// The node of the parent a child hangs under in the head walk: a pre-Gloas parent's single
    /// node is conventionally `EMPTY`.
    fn parent_node(&self, index: usize) -> Result<PayloadStatus, String> {
        Ok(match self.parent_edge(index)? {
            ParentPayloadStatus::Full => PayloadStatus::Full,
            ParentPayloadStatus::Empty | ParentPayloadStatus::PreGloas => PayloadStatus::Empty,
        })
    }

    /// Independent `get_head` for this harness: justified and finalized at the genesis anchor
    /// (every leaf viable), no proposer boost, unit balances.
    ///
    /// Execution validity enters in three places:
    /// - A block whose state ends on a payload stored invalid is removed with its subtree: a
    ///   pre-Gloas block's own payload, a Gloas block's bid parent.
    /// - A Gloas `FULL` node exists only for a received payload not stored invalid. A pre-Gloas
    ///   block has no `FULL` node.
    /// - A vote that reaches an invalid pre-Gloas block, or reaches an invalid Gloas block through
    ///   its `FULL` node, is dead weight: it counts for nothing at or above that block.
    fn model_head(&self) -> Result<(usize, PayloadStatus), String> {
        let mut excluded = vec![false; self.blocks.len()];
        for index in 0..self.blocks.len() {
            let block = self.blocks.get(index).ok_or("unknown model block")?;
            let parent_excluded = block
                .parent
                .is_some_and(|parent| excluded.get(parent).copied().unwrap_or(true));
            let builds_on_invalid = if block.pre_gloas {
                self.stored_is_invalid(index)?
            } else {
                match block.execution_parent {
                    Some(execution_parent) => self.stored_is_invalid(execution_parent)?,
                    None => false,
                }
            };
            if let Some(slot) = excluded.get_mut(index) {
                *slot = parent_excluded || builds_on_invalid;
            }
        }

        let mut head = (ANCHOR, PayloadStatus::Pending);
        loop {
            let (index, status) = head;
            let mut children = vec![];
            if status == PayloadStatus::Pending {
                children.push((index, PayloadStatus::Empty));
                let block = self.blocks.get(index).ok_or("unknown model block")?;
                if block.revealed && !self.stored_is_invalid(index)? {
                    children.push((index, PayloadStatus::Full));
                }
            } else {
                for child in 0..self.blocks.len() {
                    let block = self.blocks.get(child).ok_or("unknown model block")?;
                    if block.parent == Some(index)
                        && self.parent_node(child)? == status
                        && !excluded.get(child).copied().unwrap_or(true)
                    {
                        children.push((child, PayloadStatus::Pending));
                    }
                }
            }

            let mut best: Option<(HeadKey, (usize, PayloadStatus))> = None;
            for child in children {
                let key = (
                    self.model_weight(child.0, child.1)?,
                    self.blocks.get(child.0).ok_or("unknown model block")?.root,
                    self.model_tiebreaker(child.0, child.1)?,
                );
                if best.as_ref().is_none_or(|(best_key, _)| key > *best_key) {
                    best = Some((key, child));
                }
            }
            match best {
                Some((_, child)) => head = child,
                None => return Ok(head),
            }
        }
    }

    /// Spec `get_weight` with unit balances, minus dead weight (see `model_head`).
    fn model_weight(&self, index: usize, status: PayloadStatus) -> Result<u64, String> {
        let block = self.blocks.get(index).ok_or("unknown model block")?;
        if status != PayloadStatus::Pending && block.slot + 1 == self.current_slot {
            return Ok(0);
        }
        let mut weight = 0;
        for &(target, vote_slot, payload_present) in self.votes.iter().flatten() {
            // Walk from the vote's block up to `index`, tracking which node of each block the
            // vote passes through. `None` is the block's `PENDING` node only.
            let target_block = self.blocks.get(target).ok_or("unknown model block")?;
            let mut current = target;
            let mut entry = (vote_slot > target_block.slot).then_some(if payload_present {
                PayloadStatus::Full
            } else {
                PayloadStatus::Empty
            });
            let mut dead = false;
            loop {
                let current_block = self.blocks.get(current).ok_or("unknown model block")?;
                if (current_block.pre_gloas || entry == Some(PayloadStatus::Full))
                    && self.stored_is_invalid(current)?
                {
                    dead = true;
                }
                if current == index {
                    // A pre-Gloas block keeps every vote in its single node.
                    let supports = status == PayloadStatus::Pending
                        || current_block.pre_gloas
                        || entry == Some(status);
                    if supports && !dead {
                        weight += 1;
                    }
                    break;
                }
                let Some(parent) = self.blocks.get(current).and_then(|block| block.parent) else {
                    break;
                };
                entry = Some(self.parent_node(current)?);
                current = parent;
            }
        }
        Ok(weight)
    }

    /// Spec `get_payload_status_tiebreaker` with no proposer boost: a block from the previous
    /// slot prefers its received `FULL` node, otherwise `FULL` wins ties against `EMPTY`.
    fn model_tiebreaker(&self, index: usize, status: PayloadStatus) -> Result<u8, String> {
        let block = self.blocks.get(index).ok_or("unknown model block")?;
        Ok(match status {
            PayloadStatus::Pending => 2,
            _ if block.slot + 1 != self.current_slot => status as u8,
            PayloadStatus::Empty => 1,
            PayloadStatus::Full => 2,
        })
    }

    /// The exact verdict of a fork choice node: that of the payload it ends on.
    fn model_verdict(
        &self,
        snapshots: &[Snapshot],
        index: usize,
        status: PayloadStatus,
    ) -> Result<ExecutionVerdict, String> {
        let Some(executed) = self.executed_payload(index, status) else {
            // The pre-anchor payload is valid by assumption.
            return Ok(ExecutionVerdict::Valid);
        };
        match snapshots.get(executed).ok_or("missing snapshot")?.stored {
            ExecutionStatus::Valid(_) | ExecutionStatus::Irrelevant(_) => {
                Ok(ExecutionVerdict::Valid)
            }
            ExecutionStatus::Invalid(_) => Ok(ExecutionVerdict::Invalid),
            ExecutionStatus::Optimistic(_) => Ok(ExecutionVerdict::Optimistic),
            ExecutionStatus::NotYetRevealed(_) => Err(format!(
                "model executed block {executed} has no revealed payload"
            )),
        }
    }

    fn check_properties(&mut self) -> Result<(), String> {
        let mut snapshots = Vec::with_capacity(self.blocks.len());
        for index in 0..self.blocks.len() {
            snapshots.push(Snapshot {
                stored: self.node(index)?.execution_status(),
                empty: self.verdict(index, PayloadStatus::Empty)?,
                full: self.verdict(index, PayloadStatus::Full)?,
            });
        }

        for (index, now) in snapshots.iter().enumerate() {
            let block = self.blocks.get(index).ok_or("unknown model block")?;
            let node = self.node(index)?;
            let prefix = format!("block {index} ({now:?})");

            // The stored status names the block's own payload.
            let stored_hash = match now.stored {
                ExecutionStatus::Valid(hash)
                | ExecutionStatus::Invalid(hash)
                | ExecutionStatus::Optimistic(hash)
                | ExecutionStatus::NotYetRevealed(hash) => Some(hash),
                ExecutionStatus::Irrelevant(_) => None,
            };
            ensure!(
                stored_hash == Some(block.block_hash),
                "{prefix}: stored status does not name the block's payload"
            );
            ensure!(
                node.payload_received().unwrap_or(false) == block.revealed,
                "{prefix}: payload_received disagrees with the envelopes delivered"
            );

            // Soundness against ground truth: fork choice never calls a valid payload invalid
            // or an invalid payload valid.
            let truly_valid = self.truly_valid(index);
            ensure!(
                !(matches!(now.stored, ExecutionStatus::Valid(_)) && !truly_valid),
                "{prefix}: truly invalid payload stored as valid"
            );
            ensure!(
                !(now.stored.is_invalid() && truly_valid),
                "{prefix}: truly valid payload stored as invalid"
            );
            for (payload_status, verdict) in [
                (PayloadStatus::Empty, now.empty),
                (PayloadStatus::Full, now.full),
            ] {
                let node_truly_valid = self.node_truly_valid(index, payload_status);
                ensure!(
                    !(verdict.is_valid() && !node_truly_valid),
                    "{prefix}: truly invalid {payload_status:?} node judged valid"
                );
                ensure!(
                    !(verdict.is_invalid() && node_truly_valid),
                    "{prefix}: truly valid {payload_status:?} node judged invalid"
                );
            }

            // Completeness: each node reports exactly the verdict of the payload it executed.
            // A Gloas anchor's `FULL` node is skipped: the anchor starts `Optimistic` and its
            // envelope never arrives, so that node never exists.
            for (payload_status, verdict) in [
                (PayloadStatus::Empty, now.empty),
                (PayloadStatus::Full, now.full),
            ] {
                if index == ANCHOR && !block.pre_gloas && payload_status == PayloadStatus::Full {
                    continue;
                }
                let expected = self.model_verdict(&snapshots, index, payload_status)?;
                ensure!(
                    verdict == expected,
                    "{prefix}: {payload_status:?} verdict {verdict:?}, expected {expected:?}"
                );
            }

            // Within a block, the `FULL` node ran everything the `EMPTY` node ran.
            ensure!(
                !(now.empty.is_invalid() && !now.full.is_invalid()),
                "{prefix}: EMPTY node invalid but FULL node not"
            );
            ensure!(
                !(now.full.is_valid() && !now.empty.is_valid()),
                "{prefix}: FULL node valid but EMPTY node not"
            );

            // Across the edge to the parent: descendants of an invalid node are invalid, and
            // ancestors of a valid node are valid. The parent node is the one this block's bid
            // extends.
            if index != ANCHOR {
                let parent_index = self
                    .fork_choice
                    .proto_array
                    .get_parent(node)
                    .and_then(|parent| self.fork_choice.proto_array.indices.get(&parent.root()))
                    .copied()
                    .ok_or_else(|| format!("{prefix}: parent missing"))?;
                let parent_full = block.execution_parent == Some(parent_index);
                let edge = self.parent_edge(index)?;
                ensure!(
                    node.get_parent_payload_status() == edge,
                    "{prefix}: parent payload status {:?} disagrees with the model edge {edge:?}",
                    node.get_parent_payload_status()
                );
                let parent_status = if parent_full {
                    PayloadStatus::Full
                } else {
                    PayloadStatus::Empty
                };
                let parent = snapshots
                    .get(parent_index)
                    .ok_or_else(|| format!("{prefix}: parent snapshot missing"))?;
                let parent_verdict = if parent_full {
                    parent.full
                } else {
                    parent.empty
                };
                for (payload_status, verdict) in [
                    (PayloadStatus::Empty, now.empty),
                    (PayloadStatus::Full, now.full),
                ] {
                    ensure!(
                        !(parent_verdict.is_invalid() && !verdict.is_invalid()),
                        "{prefix}: {payload_status:?} node not invalid but its parent's \
                         {parent_status:?} node is"
                    );
                    ensure!(
                        !(verdict.is_valid() && !parent_verdict.is_valid()),
                        "{prefix}: {payload_status:?} node valid but its parent's \
                         {parent_status:?} node is {parent_verdict:?}"
                    );
                }
            }

            // Settled statuses never change. Each envelope is delivered once, so only the
            // `NotYetRevealed -> *` and `Optimistic -> {Valid, Invalid}` moves are legal.
            if let Some(previous) = self.snapshots.get(index) {
                let legal = match (previous.stored, now.stored) {
                    (ExecutionStatus::Valid(_), ExecutionStatus::Valid(_))
                    | (ExecutionStatus::Invalid(_), ExecutionStatus::Invalid(_))
                    | (ExecutionStatus::NotYetRevealed(_), _) => true,
                    (ExecutionStatus::Optimistic(_), next) => {
                        !matches!(next, ExecutionStatus::NotYetRevealed(_))
                    }
                    _ => false,
                };
                ensure!(
                    legal,
                    "{prefix}: stored status moved from {:?}",
                    previous.stored
                );

                ensure!(
                    !(previous.empty.is_valid() && !now.empty.is_valid())
                        && !(previous.empty.is_invalid() && !now.empty.is_invalid()),
                    "{prefix}: settled EMPTY verdict moved from {:?}",
                    previous.empty
                );
                // Before the reveal the `FULL` verdict is borrowed from the execution parent, so
                // it is only settled once the block's own payload has been judged.
                if !matches!(previous.stored, ExecutionStatus::NotYetRevealed(_)) {
                    ensure!(
                        !(previous.full.is_valid() && !now.full.is_valid())
                            && !(previous.full.is_invalid() && !now.full.is_invalid()),
                        "{prefix}: settled FULL verdict moved from {:?}",
                        previous.full
                    );
                }
            }
        }

        let invalid_nodes = snapshots
            .iter()
            .filter(|snapshot| snapshot.stored.is_invalid())
            .count();
        self.stats.max_invalid_nodes = self.stats.max_invalid_nodes.max(invalid_nodes);
        self.snapshots = snapshots;
        Ok(())
    }
}

/// Indices biased towards recent blocks, so scenarios grow deep chains as well as wide trees.
fn parent_offset_strategy() -> impl Strategy<Value = u8> {
    prop_oneof![3 => 0u8..2, 1 => any::<u8>()]
}

fn invalidation_kind_strategy() -> impl Strategy<Value = InvalidationKind> {
    prop_oneof![
        Just(InvalidationKind::One),
        Just(InvalidationKind::ManyExactLatestValidAncestor),
        Just(InvalidationKind::ManyUnknownLatestValidAncestor),
    ]
}

pub fn operation_strategy() -> impl Strategy<Value = OptSyncOperation> {
    prop_oneof![
        5 => (
            parent_offset_strategy(),
            any::<bool>(),
            proptest::bool::weighted(0.8),
            any::<u8>(),
        )
            .prop_map(|(parent_offset, on_full, payload_valid, slot_skip)| {
                OptSyncOperation::AddBlock {
                    parent_offset,
                    on_full,
                    payload_valid,
                    slot_skip,
                }
            }),
        4 => (any::<u8>(), any::<bool>())
            .prop_map(|(block, el_judges)| OptSyncOperation::RevealPayload { block, el_judges }),
        2 => any::<u8>().prop_map(|block| OptSyncOperation::ValidatePayload { block }),
        2 => (any::<u8>(), invalidation_kind_strategy())
            .prop_map(|(block, kind)| OptSyncOperation::InvalidatePayload { block, kind }),
        2 => (any::<u8>(), any::<u8>(), any::<bool>()).prop_map(
            |(validator, block, payload_present)| OptSyncOperation::Vote {
                validator,
                block,
                payload_present,
            }
        ),
        1 => Just(OptSyncOperation::AdvanceSlot),
        2 => Just(OptSyncOperation::FindHead),
        1 => Just(OptSyncOperation::ReloadFromBytes),
    ]
}

pub fn fork_mode_strategy() -> impl Strategy<Value = ForkMode> {
    prop_oneof![
        Just(ForkMode::PreGloas),
        Just(ForkMode::Gloas),
        Just(ForkMode::Boundary),
    ]
}

pub fn scenario_strategy() -> impl Strategy<Value = Vec<OptSyncOperation>> {
    proptest::collection::vec(operation_strategy(), 1..150)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;

    proptest! {
        /// Grind random scenarios. Raise `PROPTEST_CASES` for a longer run.
        #[test]
        fn optimistic_sync_properties(
            fork_mode in fork_mode_strategy(),
            operations in scenario_strategy(),
        ) {
            run(fork_mode, &operations).map_err(TestCaseError::fail)?;
        }
    }

    /// Guard against a vacuous fuzzer: the generator must actually reach the interesting paths,
    /// and each fork mode must build the blocks it names.
    #[test]
    fn scenarios_exercise_invalidation() {
        let mut runner = TestRunner::deterministic();
        let strategy = scenario_strategy();
        let mut total = Stats::default();
        for fork_mode in [ForkMode::PreGloas, ForkMode::Gloas, ForkMode::Boundary] {
            let mut mode_total = Stats::default();
            for _ in 0..100 {
                let operations = strategy
                    .new_tree(&mut runner)
                    .expect("strategy should generate a scenario")
                    .current();
                let stats =
                    run(fork_mode, &operations).expect("scenario should uphold the properties");
                mode_total.pre_gloas_blocks += stats.pre_gloas_blocks;
                mode_total.gloas_blocks += stats.gloas_blocks;
                mode_total.fork_crossings += stats.fork_crossings;
                mode_total.deep_invalidations += stats.deep_invalidations;
                mode_total.head_checks += stats.head_checks;

                total.blocks_imported += stats.blocks_imported;
                total.blocks_refused += stats.blocks_refused;
                total.payloads_revealed += stats.payloads_revealed;
                total.payloads_rejected_on_import += stats.payloads_rejected_on_import;
                total.validations += stats.validations;
                total.invalidations += stats.invalidations;
                total.full_heads += stats.full_heads;
                total.max_invalid_nodes = total.max_invalid_nodes.max(stats.max_invalid_nodes);
            }
            let built_both = (mode_total.pre_gloas_blocks > 0, mode_total.gloas_blocks > 0);
            let expected = match fork_mode {
                ForkMode::PreGloas => (true, false),
                ForkMode::Gloas => (false, true),
                ForkMode::Boundary => (true, true),
            };
            assert_eq!(built_both, expected, "{fork_mode:?}: {mode_total:?}");
            assert_eq!(
                mode_total.fork_crossings > 0,
                fork_mode == ForkMode::Boundary,
                "{fork_mode:?}: {mode_total:?}"
            );
            assert!(
                mode_total.deep_invalidations > 0,
                "{fork_mode:?}: {mode_total:?}"
            );
            assert!(mode_total.head_checks > 0, "{fork_mode:?}: {mode_total:?}");
        }
        assert!(total.blocks_imported > 0, "{total:?}");
        assert!(total.blocks_refused > 0, "{total:?}");
        assert!(total.payloads_revealed > 0, "{total:?}");
        assert!(total.payloads_rejected_on_import > 0, "{total:?}");
        assert!(total.validations > 0, "{total:?}");
        assert!(total.invalidations > 0, "{total:?}");
        assert!(total.max_invalid_nodes > 1, "{total:?}");
        assert!(total.full_heads > 0, "{total:?}");
    }

    /// A hand-written scenario: an invalid payload deep in a chain, reported with its exact
    /// latest valid ancestor, condemns the chain above it but not its `EMPTY` sibling branch.
    #[test]
    fn exact_latest_valid_ancestor_condemns_chain() {
        use OptSyncOperation::*;
        let add = |parent_offset, on_full, payload_valid| AddBlock {
            parent_offset,
            on_full,
            payload_valid,
            slot_skip: 0,
        };
        let operations = vec![
            add(0, false, true), // 1 on anchor
            RevealPayload {
                block: 0,
                el_judges: false,
            },
            add(0, true, false), // 2 on 1's FULL node, invalid payload
            RevealPayload {
                block: 0,
                el_judges: false,
            },
            add(0, true, true), // 3 on 2's FULL node
            RevealPayload {
                block: 0,
                el_judges: false,
            },
            add(1, false, true), // 4 on 2's EMPTY node
            FindHead,
            InvalidatePayload {
                // Eligible invalid payloads are blocks 2 and 3: report block 3, so the walk must
                // reach down through block 2 to find the deepest condemned node.
                block: 1,
                kind: InvalidationKind::ManyExactLatestValidAncestor,
            },
            FindHead,
            add(1, true, true), // on 3's FULL node: refused
        ];
        let runner = run_scenario(ForkMode::Gloas, &operations)
            .expect("scenario should uphold the properties");
        let stats = runner.stats;
        assert_eq!(stats.blocks_refused, 1, "{stats:?}");
        assert_eq!(stats.max_invalid_nodes, 2, "{stats:?}");
        assert_eq!(stats.deep_invalidations, 1, "{stats:?}");
        let invalid = (0..runner.blocks.len())
            .filter(|&index| runner.stored_is_invalid(index).expect("block is known"))
            .collect::<Vec<_>>();
        assert_eq!(invalid, vec![2, 3]);
    }
}
