//! Optimistic sync tests for Gloas payload validation and invalidation.
//!
//! See https://github.com/sigp/lighthouse/issues/10152.

use super::*;

/// Gloas from `gloas_fork_epoch`, with every earlier fork at genesis.
fn fork_boundary_spec(gloas_fork_epoch: u64) -> ChainSpec {
    let mut spec = gloas_spec();
    spec.gloas_fork_epoch = Some(Epoch::new(gloas_fork_epoch));
    spec
}

/// A latest valid hash whose block is separated from the first condemned payload by blocks on
/// `EMPTY` edges (https://github.com/ethereum/consensus-specs/issues/5635).
///
/// ```text
/// genesis <-EMPTY- A <-FULL- X1 <-EMPTY- X2 <-EMPTY- B <-FULL- C
///                                                   ^
///                                                   +-EMPTY- D
/// ```
///
/// X1 and X2 never reveal their payloads, so the execution chain is `A.payload -> B.payload ->
/// C.payload`. The EL rejects C's payload with `latest_valid_hash = A.payload`. B is the deepest
/// block whose payload this branch executed, so only B's `FULL` side and C are condemned:
///
/// - X1 and X2 build on A's valid payload and keep their statuses.
/// - D, on B's `EMPTY` edge, builds on A's valid payload and stays viable.
/// - B survives on its `EMPTY` node and the head moves to D.
pub fn get_gloas_latest_valid_hash_gap_test_definition() -> ForkChoiceTestDefinition {
    let a = get_root(1);
    let x1 = get_root(2);
    let x2 = get_root(3);
    let b = get_root(4);
    let c = get_root(5);
    let d = get_root(6);

    let ops = vec![
        // A builds on genesis' EMPTY node (genesis is the anchor and never FULL).
        Operation::ProcessBlock {
            slot: Slot::new(1),
            root: a,
            parent_root: get_root(0),
            justified_checkpoint: get_checkpoint(0),
            finalized_checkpoint: get_checkpoint(0),
            execution_payload_parent_hash: Some(get_hash(42)),
            execution_payload_block_hash: Some(get_hash(1)),
        },
        Operation::ProcessOptimisticExecutionPayloadEnvelope { block_root: a },
        // X1 builds on A's payload. Its own payload is never revealed.
        Operation::ProcessBlock {
            slot: Slot::new(2),
            root: x1,
            parent_root: a,
            justified_checkpoint: get_checkpoint(0),
            finalized_checkpoint: get_checkpoint(0),
            execution_payload_parent_hash: Some(get_hash(1)),
            execution_payload_block_hash: Some(get_hash(2)),
        },
        // X2 and B skip X1's and X2's missing payloads, so they still build on A's.
        Operation::ProcessBlock {
            slot: Slot::new(3),
            root: x2,
            parent_root: x1,
            justified_checkpoint: get_checkpoint(0),
            finalized_checkpoint: get_checkpoint(0),
            execution_payload_parent_hash: Some(get_hash(1)),
            execution_payload_block_hash: Some(get_hash(3)),
        },
        Operation::ProcessBlock {
            slot: Slot::new(4),
            root: b,
            parent_root: x2,
            justified_checkpoint: get_checkpoint(0),
            finalized_checkpoint: get_checkpoint(0),
            execution_payload_parent_hash: Some(get_hash(1)),
            execution_payload_block_hash: Some(get_hash(4)),
        },
        Operation::ProcessOptimisticExecutionPayloadEnvelope { block_root: b },
        // C builds on B's payload.
        Operation::ProcessBlock {
            slot: Slot::new(5),
            root: c,
            parent_root: b,
            justified_checkpoint: get_checkpoint(0),
            finalized_checkpoint: get_checkpoint(0),
            execution_payload_parent_hash: Some(get_hash(4)),
            execution_payload_block_hash: Some(get_hash(5)),
        },
        Operation::ProcessOptimisticExecutionPayloadEnvelope { block_root: c },
        // D skips B's payload, so it builds on A's.
        Operation::ProcessBlock {
            slot: Slot::new(6),
            root: d,
            parent_root: b,
            justified_checkpoint: get_checkpoint(0),
            finalized_checkpoint: get_checkpoint(0),
            execution_payload_parent_hash: Some(get_hash(1)),
            execution_payload_block_hash: Some(get_hash(6)),
        },
        Operation::AssertParentPayloadStatus {
            block_root: x1,
            expected_status: ParentPayloadStatus::Full,
        },
        Operation::AssertParentPayloadStatus {
            block_root: x2,
            expected_status: ParentPayloadStatus::Empty,
        },
        Operation::AssertParentPayloadStatus {
            block_root: b,
            expected_status: ParentPayloadStatus::Empty,
        },
        Operation::AssertParentPayloadStatus {
            block_root: c,
            expected_status: ParentPayloadStatus::Full,
        },
        Operation::AssertParentPayloadStatus {
            block_root: d,
            expected_status: ParentPayloadStatus::Empty,
        },
        // A vote for C's FULL node pulls the head down the whole chain.
        Operation::ProcessGloasAttestation {
            validator_index: 0,
            block_root: c,
            attestation_slot: Slot::new(6),
            payload_present: true,
        },
        Operation::FindHead {
            justified_checkpoint: get_checkpoint(0),
            finalized_checkpoint: get_checkpoint(0),
            justified_state_balances: vec![1],
            expected_head: c,
            current_slot: Slot::new(10),
            expected_payload_status: Some(PayloadStatus::Full),
        },
        // The EL rejects C's payload and names A's as the latest valid one.
        Operation::InvalidatePayload {
            head_hash: get_hash(5),
            latest_valid_ancestor: Some(get_hash(1)),
        },
        // A's payload is the latest valid one and the gap blocks executed no payload of their
        // own, so none of them is touched.
        Operation::AssertExecutionStatus {
            block_root: a,
            expected: ExecutionStatus::Optimistic(get_hash(1)),
        },
        Operation::AssertExecutionStatus {
            block_root: x1,
            expected: ExecutionStatus::NotYetRevealed(get_hash(2)),
        },
        Operation::AssertExecutionStatus {
            block_root: x2,
            expected: ExecutionStatus::NotYetRevealed(get_hash(3)),
        },
        // B is the deepest executed block: its payload and everything built on it are condemned.
        Operation::AssertExecutionStatus {
            block_root: b,
            expected: ExecutionStatus::Invalid(get_hash(4)),
        },
        Operation::AssertExecutionStatus {
            block_root: c,
            expected: ExecutionStatus::Invalid(get_hash(5)),
        },
        // D is on B's EMPTY edge, so the sweep spares it.
        Operation::AssertExecutionStatus {
            block_root: d,
            expected: ExecutionStatus::NotYetRevealed(get_hash(6)),
        },
        // C is excluded. B falls back to its EMPTY node, which leads to D.
        Operation::FindHead {
            justified_checkpoint: get_checkpoint(0),
            finalized_checkpoint: get_checkpoint(0),
            justified_state_balances: vec![1],
            expected_head: d,
            current_slot: Slot::new(10),
            expected_payload_status: Some(PayloadStatus::Empty),
        },
        // The vote for C's invalid FULL node no longer counts anywhere.
        Operation::AssertPayloadWeights {
            block_root: b,
            expected_full_weight: 0,
            expected_empty_weight: 0,
        },
        Operation::AssertPayloadWeights {
            block_root: a,
            expected_full_weight: 0,
            expected_empty_weight: 0,
        },
    ];

    ForkChoiceTestDefinition {
        finalized_block_slot: Slot::new(0),
        justified_checkpoint: get_checkpoint(0),
        finalized_checkpoint: get_checkpoint(0),
        operations: ops,
        execution_payload_parent_hash: Some(get_hash(42)),
        execution_payload_block_hash: Some(get_hash(0)),
        spec: Some(gloas_spec()),
    }
}

/// The companion of `get_gloas_latest_valid_hash_gap_test_definition` where the first condemned
/// payload is the head's own (https://github.com/ethereum/consensus-specs/issues/5635).
///
/// ```text
/// genesis <-EMPTY- A <-FULL- X1 <-EMPTY- X2 <-EMPTY- B
///                                                   ^
///                                                   +-EMPTY- D (imported after invalidation)
/// ```
///
/// The head is `(B, FULL)`. The EL rejects B's payload with `latest_valid_hash = A.payload`. B is
/// the deepest block whose payload this branch executed, so only B's `FULL` side is condemned:
///
/// - X1 and X2 build on A's valid payload and keep their statuses.
/// - B stays head on its `EMPTY` node, which builds on A's valid payload.
/// - A child on B's `EMPTY` edge imports and extends the chain.
pub fn get_gloas_latest_valid_hash_gap_invalid_head_test_definition() -> ForkChoiceTestDefinition {
    let a = get_root(1);
    let x1 = get_root(2);
    let x2 = get_root(3);
    let b = get_root(4);
    let d = get_root(5);

    let ops = vec![
        // A builds on genesis' EMPTY node (genesis is the anchor and never FULL).
        Operation::ProcessBlock {
            slot: Slot::new(1),
            root: a,
            parent_root: get_root(0),
            justified_checkpoint: get_checkpoint(0),
            finalized_checkpoint: get_checkpoint(0),
            execution_payload_parent_hash: Some(get_hash(42)),
            execution_payload_block_hash: Some(get_hash(1)),
        },
        Operation::ProcessOptimisticExecutionPayloadEnvelope { block_root: a },
        // X1 builds on A's payload. Its own payload is never revealed.
        Operation::ProcessBlock {
            slot: Slot::new(2),
            root: x1,
            parent_root: a,
            justified_checkpoint: get_checkpoint(0),
            finalized_checkpoint: get_checkpoint(0),
            execution_payload_parent_hash: Some(get_hash(1)),
            execution_payload_block_hash: Some(get_hash(2)),
        },
        // X2 and B skip X1's and X2's missing payloads, so they still build on A's.
        Operation::ProcessBlock {
            slot: Slot::new(3),
            root: x2,
            parent_root: x1,
            justified_checkpoint: get_checkpoint(0),
            finalized_checkpoint: get_checkpoint(0),
            execution_payload_parent_hash: Some(get_hash(1)),
            execution_payload_block_hash: Some(get_hash(3)),
        },
        Operation::ProcessBlock {
            slot: Slot::new(4),
            root: b,
            parent_root: x2,
            justified_checkpoint: get_checkpoint(0),
            finalized_checkpoint: get_checkpoint(0),
            execution_payload_parent_hash: Some(get_hash(1)),
            execution_payload_block_hash: Some(get_hash(4)),
        },
        Operation::ProcessOptimisticExecutionPayloadEnvelope { block_root: b },
        Operation::AssertParentPayloadStatus {
            block_root: x1,
            expected_status: ParentPayloadStatus::Full,
        },
        Operation::AssertParentPayloadStatus {
            block_root: x2,
            expected_status: ParentPayloadStatus::Empty,
        },
        Operation::AssertParentPayloadStatus {
            block_root: b,
            expected_status: ParentPayloadStatus::Empty,
        },
        // A vote for B's FULL node makes it the head.
        Operation::ProcessGloasAttestation {
            validator_index: 0,
            block_root: b,
            attestation_slot: Slot::new(6),
            payload_present: true,
        },
        Operation::FindHead {
            justified_checkpoint: get_checkpoint(0),
            finalized_checkpoint: get_checkpoint(0),
            justified_state_balances: vec![1],
            expected_head: b,
            current_slot: Slot::new(10),
            expected_payload_status: Some(PayloadStatus::Full),
        },
        // The EL rejects the head's own payload and names A's as the latest valid one.
        Operation::InvalidatePayload {
            head_hash: get_hash(4),
            latest_valid_ancestor: Some(get_hash(1)),
        },
        // A's payload is the latest valid one and the gap blocks executed no payload of their
        // own, so none of them is touched.
        Operation::AssertExecutionStatus {
            block_root: a,
            expected: ExecutionStatus::Optimistic(get_hash(1)),
        },
        Operation::AssertExecutionStatus {
            block_root: x1,
            expected: ExecutionStatus::NotYetRevealed(get_hash(2)),
        },
        Operation::AssertExecutionStatus {
            block_root: x2,
            expected: ExecutionStatus::NotYetRevealed(get_hash(3)),
        },
        Operation::AssertExecutionStatus {
            block_root: b,
            expected: ExecutionStatus::Invalid(get_hash(4)),
        },
        // B stays head, falling back to its EMPTY node.
        Operation::FindHead {
            justified_checkpoint: get_checkpoint(0),
            finalized_checkpoint: get_checkpoint(0),
            justified_state_balances: vec![1],
            expected_head: b,
            current_slot: Slot::new(10),
            expected_payload_status: Some(PayloadStatus::Empty),
        },
        // The vote for B's invalid FULL node no longer counts anywhere.
        Operation::AssertPayloadWeights {
            block_root: b,
            expected_full_weight: 0,
            expected_empty_weight: 0,
        },
        Operation::AssertPayloadWeights {
            block_root: a,
            expected_full_weight: 0,
            expected_empty_weight: 0,
        },
        // A child on B's EMPTY edge builds on A's valid payload, so it imports and becomes head.
        Operation::ProcessBlock {
            slot: Slot::new(11),
            root: d,
            parent_root: b,
            justified_checkpoint: get_checkpoint(0),
            finalized_checkpoint: get_checkpoint(0),
            execution_payload_parent_hash: Some(get_hash(1)),
            execution_payload_block_hash: Some(get_hash(5)),
        },
        Operation::AssertParentPayloadStatus {
            block_root: d,
            expected_status: ParentPayloadStatus::Empty,
        },
        Operation::FindHead {
            justified_checkpoint: get_checkpoint(0),
            finalized_checkpoint: get_checkpoint(0),
            justified_state_balances: vec![1],
            expected_head: d,
            current_slot: Slot::new(12),
            expected_payload_status: Some(PayloadStatus::Empty),
        },
    ];

    ForkChoiceTestDefinition {
        finalized_block_slot: Slot::new(0),
        justified_checkpoint: get_checkpoint(0),
        finalized_checkpoint: get_checkpoint(0),
        operations: ops,
        execution_payload_parent_hash: Some(get_hash(42)),
        execution_payload_block_hash: Some(get_hash(0)),
        spec: Some(gloas_spec()),
    }
}

/// A Gloas block `get_root(root)` committing to payload `get_hash(root)` and building on payload
/// `get_hash(parent_hash)`.
fn gloas_block(slot: u64, root: u64, parent: u64, parent_hash: u64) -> Operation {
    Operation::ProcessBlock {
        slot: Slot::new(slot),
        root: get_root(root),
        parent_root: get_root(parent),
        justified_checkpoint: get_checkpoint(0),
        finalized_checkpoint: get_checkpoint(0),
        execution_payload_parent_hash: Some(get_hash(parent_hash)),
        execution_payload_block_hash: Some(get_hash(root)),
    }
}

/// A pre-Gloas block `get_root(root)` carrying payload `get_hash(root)`.
fn pre_gloas_block(slot: u64, root: u64, parent: u64) -> Operation {
    Operation::ProcessBlock {
        slot: Slot::new(slot),
        root: get_root(root),
        parent_root: get_root(parent),
        justified_checkpoint: get_checkpoint(0),
        finalized_checkpoint: get_checkpoint(0),
        execution_payload_parent_hash: None,
        execution_payload_block_hash: None,
    }
}

fn vote(
    validator_index: usize,
    root: u64,
    attestation_slot: u64,
    payload_present: bool,
) -> Operation {
    Operation::ProcessGloasAttestation {
        validator_index,
        block_root: get_root(root),
        attestation_slot: Slot::new(attestation_slot),
        payload_present,
    }
}

fn find_head(
    balances: &[u64],
    expected_head: u64,
    current_slot: u64,
    expected_payload_status: PayloadStatus,
) -> Operation {
    Operation::FindHead {
        justified_checkpoint: get_checkpoint(0),
        finalized_checkpoint: get_checkpoint(0),
        justified_state_balances: balances.to_vec(),
        expected_head: get_root(expected_head),
        current_slot: Slot::new(current_slot),
        expected_payload_status: Some(expected_payload_status),
    }
}

fn assert_status(root: u64, expected: ExecutionStatus) -> Operation {
    Operation::AssertExecutionStatus {
        block_root: get_root(root),
        expected,
    }
}

fn assert_weights(root: u64, weight: u64, full_weight: u64, empty_weight: u64) -> [Operation; 2] {
    [
        Operation::AssertWeight {
            block_root: get_root(root),
            weight,
        },
        Operation::AssertPayloadWeights {
            block_root: get_root(root),
            expected_full_weight: full_weight,
            expected_empty_weight: empty_weight,
        },
    ]
}

fn optimistic_envelope(root: u64) -> Operation {
    Operation::ProcessOptimisticExecutionPayloadEnvelope {
        block_root: get_root(root),
    }
}

fn valid_envelope(root: u64) -> Operation {
    Operation::ProcessExecutionPayloadEnvelope {
        block_root: get_root(root),
    }
}

fn definition(operations: Vec<Operation>, spec: ChainSpec) -> ForkChoiceTestDefinition {
    let gloas_genesis = spec.gloas_fork_epoch == Some(Epoch::new(0));
    ForkChoiceTestDefinition {
        finalized_block_slot: Slot::new(0),
        justified_checkpoint: get_checkpoint(0),
        finalized_checkpoint: get_checkpoint(0),
        operations,
        execution_payload_parent_hash: gloas_genesis.then(|| get_hash(42)),
        execution_payload_block_hash: gloas_genesis.then(|| get_hash(0)),
        spec: Some(spec),
    }
}

/// The tree shared by the weight accounting tests:
///
/// ```text
/// genesis <-EMPTY- P <-FULL- B <-FULL-- CF
///                            ^
///                            +-EMPTY- CE
/// ```
///
/// Votes, with balances that make each one identifiable in a sum:
///
/// - validator 0 (1): `(B, FULL)`
/// - validator 1 (2): `(B, EMPTY)`
/// - validator 2 (4): CF
/// - validator 3 (8): CE
fn weight_accounting_setup() -> Vec<Operation> {
    let mut ops = weight_accounting_tree();
    ops.extend(invalidate_weight_accounting_tree());
    ops
}

/// The tree of `weight_accounting_setup`, before the invalidation.
fn weight_accounting_tree() -> Vec<Operation> {
    let balances = [1, 2, 4, 8];
    let mut ops = vec![
        gloas_block(1, 1, 0, 42),
        optimistic_envelope(1),
        gloas_block(2, 2, 1, 1),
        optimistic_envelope(2),
        // CF builds on B's payload, CE skips it.
        gloas_block(3, 3, 2, 2),
        gloas_block(3, 4, 2, 1),
        vote(0, 2, 4, true),
        vote(1, 2, 4, false),
        vote(2, 3, 4, false),
        vote(3, 4, 4, false),
        // B's EMPTY side (2 + 8) outweighs its FULL side (1 + 4).
        find_head(&balances, 4, 10, PayloadStatus::Empty),
    ];
    ops.extend(TreeWeights::BEFORE_INVALIDATION.assert());
    ops
}

/// The EL rejects B's payload with P's as the latest valid one, condemning B's FULL node and CF.
fn invalidate_weight_accounting_tree() -> Vec<Operation> {
    vec![
        Operation::InvalidatePayload {
            head_hash: get_hash(2),
            latest_valid_ancestor: Some(get_hash(1)),
        },
        assert_status(2, ExecutionStatus::Invalid(get_hash(2))),
        assert_status(3, ExecutionStatus::Invalid(get_hash(3))),
        assert_status(4, ExecutionStatus::NotYetRevealed(get_hash(4))),
    ]
}

/// Weights of the `weight_accounting_setup` tree: `(weight, full, empty)` for B, and the weights
/// of CF and CE. P carries B's weight on its FULL side.
pub struct TreeWeights {
    b: (u64, u64, u64),
    cf: u64,
    ce: u64,
}

impl TreeWeights {
    const BEFORE_INVALIDATION: Self = Self {
        b: (15, 5, 10),
        cf: 4,
        ce: 8,
    };

    fn assert(&self) -> Vec<Operation> {
        let (weight, full, empty) = self.b;
        let mut ops = vec![];
        ops.extend(assert_weights(2, weight, full, empty));
        ops.extend(assert_weights(1, weight, weight, 0));
        ops.push(Operation::AssertWeight {
            block_root: get_root(3),
            weight: self.cf,
        });
        ops.push(Operation::AssertWeight {
            block_root: get_root(4),
            weight: self.ce,
        });
        ops
    }
}

/// Weight accounting for a Gloas node whose payload is invalid
/// (https://github.com/sigp/lighthouse/pull/9934#discussion_r4127650147).
///
/// See `weight_accounting_setup` for the tree.
pub fn get_gloas_invalid_payload_weight_accounting_test_definition() -> ForkChoiceTestDefinition {
    let mut ops = weight_accounting_setup();

    // Settle the invalidation.
    ops.push(find_head(&[1, 2, 4, 8], 4, 10, PayloadStatus::Empty));
    // B's FULL side is dead: its own FULL vote (1) and CF's weight (4) are gone, and its EMPTY
    // side keeps its own vote (2) and CE's weight (8).
    ops.extend(assert_weights(2, 10, 0, 10));
    // A FULL child does not carry its weight past the invalid parent, an EMPTY child does.
    ops.extend(assert_weights(1, 10, 10, 0));
    ops.push(Operation::AssertWeight {
        block_root: get_root(4),
        weight: 8,
    });

    // A new vote for B's invalid FULL node counts nowhere.
    ops.push(vote(4, 2, 5, true));
    ops.push(find_head(&[1, 2, 4, 8, 16], 4, 10, PayloadStatus::Empty));
    ops.extend(assert_weights(2, 10, 0, 10));
    ops.extend(assert_weights(1, 10, 10, 0));

    // Validator 0 moves its vote off B's FULL node, whose weight was already removed on
    // invalidation. Only the new vote for CE is added; B and P are not reduced a second time.
    ops.push(vote(0, 4, 6, false));
    ops.push(find_head(&[1, 2, 4, 8, 16], 4, 10, PayloadStatus::Empty));
    ops.push(Operation::AssertWeight {
        block_root: get_root(4),
        weight: 9,
    });
    ops.extend(assert_weights(2, 11, 0, 11));
    ops.extend(assert_weights(1, 11, 11, 0));

    definition(ops, gloas_spec())
}

/// Resetting to optimistic after an invalidation restores the weights from before it
/// (https://github.com/sigp/lighthouse/pull/9934#discussion_r4127650147).
///
/// `settle_invalidation` runs fork choice between the invalidation and the reset, so the weights
/// being restored have already been removed.
pub fn get_gloas_optimistic_reset_restores_weights_test_definition(
    settle_invalidation: bool,
) -> ForkChoiceTestDefinition {
    let mut ops = weight_accounting_setup();

    if settle_invalidation {
        ops.push(find_head(&[1, 2, 4, 8], 4, 10, PayloadStatus::Empty));
        ops.extend(assert_weights(2, 10, 0, 10));
        ops.extend(assert_weights(1, 10, 10, 0));
    }

    ops.push(Operation::ResetToOptimistic);
    ops.push(assert_status(2, ExecutionStatus::Optimistic(get_hash(2))));
    ops.push(assert_status(3, ExecutionStatus::Optimistic(get_hash(3))));
    ops.push(assert_status(
        4,
        ExecutionStatus::NotYetRevealed(get_hash(4)),
    ));
    ops.extend(TreeWeights::BEFORE_INVALIDATION.assert());

    // Running fork choice again leaves the restored weights alone.
    ops.push(find_head(&[1, 2, 4, 8], 4, 10, PayloadStatus::Empty));
    ops.extend(TreeWeights::BEFORE_INVALIDATION.assert());

    definition(ops, gloas_spec())
}

/// Gloas versions of the pre-Gloas optimistic reset tests on
/// https://github.com/michaelsproul/lighthouse/tree/test-optimistic-reset-pre-gloas, on the
/// `weight_accounting_setup` tree.
///
/// Votes that moved but were not yet applied when the invalidation landed are applied by the
/// reset. Each move is `(validator, block)`, made at slot 5 with `payload_present = false`.
pub fn get_gloas_optimistic_reset_applies_pending_votes_test_definition(
    moves: &[(usize, u64)],
    expected: TreeWeights,
    expected_head: u64,
) -> ForkChoiceTestDefinition {
    let mut ops = weight_accounting_tree();
    for &(validator, block) in moves {
        ops.push(vote(validator, block, 5, false));
    }
    ops.extend(invalidate_weight_accounting_tree());
    ops.push(Operation::ResetToOptimistic);
    ops.extend(expected.assert());
    ops.push(find_head(
        &[1, 2, 4, 8],
        expected_head,
        10,
        PayloadStatus::Empty,
    ));
    ops.extend(expected.assert());

    definition(ops, gloas_spec())
}

/// A vote that leaves the invalid subtree after the invalidation was applied is restored at its
/// new target by the reset, and not at its old one.
pub fn get_gloas_optimistic_reset_moves_vote_out_of_invalid_subtree_test_definition()
-> ForkChoiceTestDefinition {
    let mut ops = weight_accounting_setup();
    ops.push(find_head(&[1, 2, 4, 8], 4, 10, PayloadStatus::Empty));
    // Validator 2 leaves CF, which was condemned, for CE.
    ops.push(vote(2, 4, 5, false));
    ops.push(Operation::ResetToOptimistic);
    let expected = TreeWeights {
        b: (15, 1, 14),
        cf: 0,
        ce: 12,
    };
    ops.extend(expected.assert());
    ops.push(find_head(&[1, 2, 4, 8], 4, 10, PayloadStatus::Empty));
    ops.extend(expected.assert());

    definition(ops, gloas_spec())
}

/// A slashing that has not yet been applied when the reset runs removes the validator's vote
/// from the rebuilt weights, and credits its balance to the equivocation score of the block it
/// voted for, `voted_block`.
pub fn get_gloas_optimistic_reset_excludes_equivocators_test_definition(
    equivocator: u64,
    voted_block: u64,
    expected: TreeWeights,
    expected_head: u64,
) -> ForkChoiceTestDefinition {
    let balances = [1, 2, 4, 8];
    let mut ops = weight_accounting_setup();
    ops.push(Operation::SetEquivocatingIndices {
        indices: vec![equivocator],
    });
    ops.push(Operation::ResetToOptimistic);
    ops.extend(expected.assert());
    ops.push(Operation::AssertEquivocatingScore {
        block_root: get_root(voted_block),
        expected: balances[equivocator as usize],
    });
    ops.push(find_head(
        &[1, 2, 4, 8],
        expected_head,
        10,
        PayloadStatus::Empty,
    ));
    ops.extend(expected.assert());

    definition(ops, gloas_spec())
}

/// A pending vote and an invalidation survive a restart, and the reset applies both.
pub fn get_gloas_optimistic_reset_after_reload_test_definition() -> ForkChoiceTestDefinition {
    let mut ops = weight_accounting_tree();
    ops.push(vote(2, 4, 5, false));
    ops.extend(invalidate_weight_accounting_tree());
    ops.push(Operation::Reload);
    ops.push(assert_status(2, ExecutionStatus::Invalid(get_hash(2))));
    ops.push(Operation::ResetToOptimistic);
    let expected = TreeWeights {
        b: (15, 1, 14),
        cf: 0,
        ce: 12,
    };
    ops.extend(expected.assert());
    ops.push(find_head(&[1, 2, 4, 8], 4, 10, PayloadStatus::Empty));
    ops.extend(expected.assert());

    definition(ops, gloas_spec())
}

/// Resetting twice gives the same weights as resetting once.
pub fn get_gloas_optimistic_reset_is_idempotent_test_definition() -> ForkChoiceTestDefinition {
    let mut ops = weight_accounting_setup();
    ops.push(Operation::ResetToOptimistic);
    ops.extend(TreeWeights::BEFORE_INVALIDATION.assert());
    ops.push(Operation::ResetToOptimistic);
    ops.extend(TreeWeights::BEFORE_INVALIDATION.assert());

    definition(ops, gloas_spec())
}

/// Resetting with no invalid payload leaves the weights alone.
pub fn get_gloas_optimistic_reset_without_invalid_payload_test_definition()
-> ForkChoiceTestDefinition {
    let mut ops = weight_accounting_tree();
    ops.push(Operation::ResetToOptimistic);
    ops.extend(TreeWeights::BEFORE_INVALIDATION.assert());
    ops.push(find_head(&[1, 2, 4, 8], 4, 10, PayloadStatus::Empty));
    ops.extend(TreeWeights::BEFORE_INVALIDATION.assert());

    definition(ops, gloas_spec())
}

/// A VALID verdict for the first Gloas payload validates the last pre-Gloas block it builds on
/// (https://github.com/sigp/lighthouse/pull/9934#discussion_r4120241064).
///
/// ```text
/// genesis <- F1 <- F2 <-PRE_GLOAS- G
/// ```
///
/// `skip_first_gloas_slot` puts G one slot after the fork, leaving the first Gloas slot empty.
pub fn get_gloas_first_payload_validates_last_pre_gloas_block_test_definition(
    skip_first_gloas_slot: bool,
) -> ForkChoiceTestDefinition {
    let first_gloas_slot = MainnetEthSpec::slots_per_epoch();
    let g_slot = first_gloas_slot + u64::from(skip_first_gloas_slot);
    let ops = vec![
        pre_gloas_block(first_gloas_slot - 2, 1, 0),
        pre_gloas_block(first_gloas_slot - 1, 2, 1),
        gloas_block(g_slot, 3, 2, 2),
        Operation::AssertParentPayloadStatus {
            block_root: get_root(3),
            expected_status: ParentPayloadStatus::PreGloas,
        },
        assert_status(1, ExecutionStatus::Optimistic(get_hash(1))),
        assert_status(2, ExecutionStatus::Optimistic(get_hash(2))),
        valid_envelope(3),
        assert_status(3, ExecutionStatus::Valid(get_hash(3))),
        assert_status(2, ExecutionStatus::Valid(get_hash(2))),
        assert_status(1, ExecutionStatus::Valid(get_hash(1))),
    ];

    definition(ops, fork_boundary_spec(1))
}

/// A VALID verdict reaches the last pre-Gloas block across a gap at the fork: the first Gloas
/// block's own payload is skipped by its child.
///
/// ```text
/// genesis <- F1 <- F2 <-PRE_GLOAS- G <-EMPTY- H
/// ```
///
/// G's payload is optimistic and H builds on F2's payload, so H's VALID verdict validates F2
/// and F1 but not G.
pub fn get_gloas_valid_payload_crosses_fork_gap_test_definition() -> ForkChoiceTestDefinition {
    let first_gloas_slot = MainnetEthSpec::slots_per_epoch();
    let ops = vec![
        pre_gloas_block(first_gloas_slot - 2, 1, 0),
        pre_gloas_block(first_gloas_slot - 1, 2, 1),
        gloas_block(first_gloas_slot, 3, 2, 2),
        optimistic_envelope(3),
        // An optimistic payload validates nothing.
        assert_status(2, ExecutionStatus::Optimistic(get_hash(2))),
        gloas_block(first_gloas_slot + 1, 4, 3, 2),
        Operation::AssertParentPayloadStatus {
            block_root: get_root(4),
            expected_status: ParentPayloadStatus::Empty,
        },
        valid_envelope(4),
        assert_status(4, ExecutionStatus::Valid(get_hash(4))),
        assert_status(3, ExecutionStatus::Optimistic(get_hash(3))),
        assert_status(2, ExecutionStatus::Valid(get_hash(2))),
        assert_status(1, ExecutionStatus::Valid(get_hash(1))),
    ];

    definition(ops, fork_boundary_spec(1))
}

/// A payload that was never revealed is condemned through its ancestry, then reset to optimistic
/// (https://github.com/sigp/lighthouse/pull/9934#discussion_r4126920810).
///
/// ```text
/// genesis <-EMPTY- P <-FULL- B <-FULL- C (payload never revealed)
/// ```
///
/// The reset turns C's `Invalid` into `Optimistic` although its payload was never received. This
/// pins the current behaviour; the review suggested resetting it to `NotYetRevealed` instead.
/// Either way C must keep behaving as unrevealed: it has no `FULL` node.
pub fn get_gloas_optimistic_reset_of_unrevealed_payload_test_definition() -> ForkChoiceTestDefinition
{
    let ops = vec![
        gloas_block(1, 1, 0, 42),
        optimistic_envelope(1),
        gloas_block(2, 2, 1, 1),
        optimistic_envelope(2),
        gloas_block(3, 3, 2, 2),
        vote(0, 3, 4, true),
        find_head(&[1], 3, 10, PayloadStatus::Empty),
        Operation::InvalidatePayload {
            head_hash: get_hash(2),
            latest_valid_ancestor: Some(get_hash(1)),
        },
        assert_status(2, ExecutionStatus::Invalid(get_hash(2))),
        // C's payload never arrived, but C committed to building on B's invalid payload.
        assert_status(3, ExecutionStatus::Invalid(get_hash(3))),
        Operation::AssertPayloadReceived {
            block_root: get_root(3),
            expected: false,
        },
        find_head(&[1], 2, 10, PayloadStatus::Empty),
        Operation::ResetToOptimistic,
        assert_status(2, ExecutionStatus::Optimistic(get_hash(2))),
        assert_status(3, ExecutionStatus::Optimistic(get_hash(3))),
        Operation::AssertPayloadReceived {
            block_root: get_root(3),
            expected: false,
        },
        // C is head again, on EMPTY: an unreceived payload still has no FULL node.
        find_head(&[1], 3, 10, PayloadStatus::Empty),
    ];

    definition(ops, gloas_spec())
}

/// A payload that was never revealed is validated through a descendant, then reset to optimistic
/// (https://github.com/sigp/lighthouse/pull/9934#discussion_r4126920810).
///
/// ```text
/// genesis <-EMPTY- P <-FULL- B <-FULL- C (payload not yet revealed) <-FULL- D
/// ```
///
/// D's VALID verdict proves C's payload valid before C's envelope arrives. C still has no `FULL`
/// node until it does, so D cannot be head.
pub fn get_gloas_unrevealed_payload_validated_by_descendant_test_definition()
-> ForkChoiceTestDefinition {
    let ops = vec![
        gloas_block(1, 1, 0, 42),
        optimistic_envelope(1),
        gloas_block(2, 2, 1, 1),
        optimistic_envelope(2),
        gloas_block(3, 3, 2, 2),
        gloas_block(4, 4, 3, 3),
        valid_envelope(4),
        assert_status(4, ExecutionStatus::Valid(get_hash(4))),
        assert_status(3, ExecutionStatus::Valid(get_hash(3))),
        assert_status(2, ExecutionStatus::Valid(get_hash(2))),
        assert_status(1, ExecutionStatus::Valid(get_hash(1))),
        Operation::AssertPayloadReceived {
            block_root: get_root(3),
            expected: false,
        },
        vote(0, 4, 5, true),
        // D extends C's FULL node, which does not exist yet.
        find_head(&[1], 3, 10, PayloadStatus::Empty),
        // The reset forgets the transitive verdict. As above, the payload is still unrevealed.
        Operation::ResetToOptimistic,
        assert_status(3, ExecutionStatus::Optimistic(get_hash(3))),
        Operation::AssertPayloadReceived {
            block_root: get_root(3),
            expected: false,
        },
        find_head(&[1], 3, 10, PayloadStatus::Empty),
        // C's envelope arrives and its FULL node leads to D.
        optimistic_envelope(3),
        assert_status(3, ExecutionStatus::Optimistic(get_hash(3))),
        find_head(&[1], 4, 10, PayloadStatus::Full),
    ];

    definition(ops, gloas_spec())
}

/// An envelope whose VALID verdict fork choice refuses still marks the payload received
/// (https://github.com/sigp/lighthouse/pull/9934#discussion_r4127757213).
///
/// ```text
/// genesis <-EMPTY- P <-FULL- B
/// ```
///
/// The refusal needs an executed ancestor that is invalid while B is not. Invalidation sweeps
/// every descendant that executed the invalid payload, so fork choice only gets here after the EL
/// contradicts itself; the test injects P's verdict to reach it.
///
/// This pins the current behaviour flagged in review: `payload_received` is set and B is
/// promoted to `Valid` before the walk reaches P and fails, so the import errors after fork
/// choice has changed.
pub fn get_gloas_refused_envelope_marks_payload_received_test_definition()
-> ForkChoiceTestDefinition {
    let ops = vec![
        gloas_block(1, 1, 0, 42),
        optimistic_envelope(1),
        gloas_block(2, 2, 1, 1),
        Operation::InjectExecutionStatus {
            block_root: get_root(1),
            status: ExecutionStatus::Invalid(get_hash(1)),
        },
        Operation::AssertPayloadReceived {
            block_root: get_root(2),
            expected: false,
        },
        Operation::ProcessExecutionPayloadEnvelopeExpectError {
            block_root: get_root(2),
        },
        Operation::AssertPayloadReceived {
            block_root: get_root(2),
            expected: true,
        },
        assert_status(2, ExecutionStatus::Valid(get_hash(2))),
        assert_status(1, ExecutionStatus::Invalid(get_hash(1))),
    ];

    definition(ops, gloas_spec())
}

/// An invalid pre-Gloas ancestor beneath a Gloas justified checkpoint
/// (https://github.com/sigp/lighthouse/issues/10152#issuecomment-5901342477).
///
/// ```text
/// genesis <- L <- A <-PRE_GLOAS- B (justified) <-FULL or EMPTY- C
/// ```
///
/// The EL rejects A's payload with L's as the latest valid one. Invalidity crosses the fork
/// boundary: B's state ends on A's payload, so B is invalid on both nodes, and C goes with it
/// whichever of B's nodes it extends. Head selection starting from B must not pick C.
pub fn get_gloas_invalid_pre_gloas_ancestor_of_justified_test_definition(
    c_extends_full: bool,
) -> ForkChoiceTestDefinition {
    let first_gloas_slot = MainnetEthSpec::slots_per_epoch();
    let justified_checkpoint = Checkpoint {
        epoch: Epoch::new(1),
        root: get_root(3),
    };
    let find_head_from_justified =
        |expected_head: u64, expected_payload_status| Operation::FindHead {
            justified_checkpoint,
            finalized_checkpoint: get_checkpoint(0),
            justified_state_balances: vec![1],
            expected_head: get_root(expected_head),
            current_slot: Slot::new(first_gloas_slot + 8),
            expected_payload_status: Some(expected_payload_status),
        };
    let c_parent_hash = if c_extends_full { 3 } else { 2 };
    let c_payload_status = if c_extends_full {
        PayloadStatus::Full
    } else {
        PayloadStatus::Empty
    };

    let ops = vec![
        pre_gloas_block(first_gloas_slot - 2, 1, 0),
        pre_gloas_block(first_gloas_slot - 1, 2, 1),
        gloas_block(first_gloas_slot, 3, 2, 2),
        optimistic_envelope(3),
        gloas_block(first_gloas_slot + 1, 4, 3, c_parent_hash),
        optimistic_envelope(4),
        vote(0, 4, first_gloas_slot + 2, c_extends_full),
        find_head_from_justified(4, c_payload_status),
        Operation::InvalidatePayload {
            head_hash: get_hash(2),
            latest_valid_ancestor: Some(get_hash(1)),
        },
        assert_status(1, ExecutionStatus::Optimistic(get_hash(1))),
        assert_status(2, ExecutionStatus::Invalid(get_hash(2))),
        assert_status(3, ExecutionStatus::Invalid(get_hash(3))),
        assert_status(4, ExecutionStatus::Invalid(get_hash(4))),
        Operation::AssertInheritedExecutionStatus {
            block_root: get_root(3),
            expected: ExpectedVerdict::Invalid,
        },
        Operation::AssertInheritedExecutionStatus {
            block_root: get_root(4),
            expected: ExpectedVerdict::Invalid,
        },
        // C is excluded. The justified block is the only candidate left, and it is invalid too,
        // which the beacon chain treats as fatal.
        find_head_from_justified(3, PayloadStatus::Empty),
    ];

    definition(ops, fork_boundary_spec(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_valid_hash_gap() {
        get_gloas_latest_valid_hash_gap_test_definition().run();
    }

    #[test]
    fn latest_valid_hash_gap_invalid_head() {
        get_gloas_latest_valid_hash_gap_invalid_head_test_definition().run();
    }

    #[test]
    fn invalid_payload_weight_accounting() {
        get_gloas_invalid_payload_weight_accounting_test_definition().run();
    }

    #[test]
    fn optimistic_reset_restores_weights() {
        get_gloas_optimistic_reset_restores_weights_test_definition(false).run();
    }

    #[test]
    fn optimistic_reset_restores_settled_weights() {
        get_gloas_optimistic_reset_restores_weights_test_definition(true).run();
    }

    #[test]
    fn first_payload_validates_last_pre_gloas_block() {
        get_gloas_first_payload_validates_last_pre_gloas_block_test_definition(false).run();
    }

    #[test]
    fn first_payload_validates_last_pre_gloas_block_after_skipped_slot() {
        get_gloas_first_payload_validates_last_pre_gloas_block_test_definition(true).run();
    }

    #[test]
    fn valid_payload_crosses_fork_gap() {
        get_gloas_valid_payload_crosses_fork_gap_test_definition().run();
    }

    #[test]
    fn optimistic_reset_of_unrevealed_payload() {
        get_gloas_optimistic_reset_of_unrevealed_payload_test_definition().run();
    }

    #[test]
    fn unrevealed_payload_validated_by_descendant() {
        get_gloas_unrevealed_payload_validated_by_descendant_test_definition().run();
    }

    #[test]
    fn refused_envelope_marks_payload_received() {
        get_gloas_refused_envelope_marks_payload_received_test_definition().run();
    }

    #[test]
    fn invalid_pre_gloas_ancestor_of_justified_full() {
        get_gloas_invalid_pre_gloas_ancestor_of_justified_test_definition(true).run();
    }

    #[test]
    fn invalid_pre_gloas_ancestor_of_justified_empty() {
        get_gloas_invalid_pre_gloas_ancestor_of_justified_test_definition(false).run();
    }

    #[test]
    fn optimistic_reset_applies_pending_votes() {
        // Validator 2 (4) moves from CF to CE.
        get_gloas_optimistic_reset_applies_pending_votes_test_definition(
            &[(2, 4)],
            TreeWeights {
                b: (15, 1, 14),
                cf: 0,
                ce: 12,
            },
            4,
        )
        .run();
        // Validator 3 (8) moves from CE to CF, tipping B to FULL.
        get_gloas_optimistic_reset_applies_pending_votes_test_definition(
            &[(3, 3)],
            TreeWeights {
                b: (15, 13, 2),
                cf: 12,
                ce: 0,
            },
            3,
        )
        .run();
        // Both swap.
        get_gloas_optimistic_reset_applies_pending_votes_test_definition(
            &[(2, 4), (3, 3)],
            TreeWeights {
                b: (15, 9, 6),
                cf: 8,
                ce: 4,
            },
            3,
        )
        .run();
    }

    #[test]
    fn optimistic_reset_moves_vote_out_of_invalid_subtree() {
        get_gloas_optimistic_reset_moves_vote_out_of_invalid_subtree_test_definition().run();
    }

    #[test]
    fn optimistic_reset_excludes_equivocators() {
        // Validator 0 (1) voted for B's FULL node.
        get_gloas_optimistic_reset_excludes_equivocators_test_definition(
            0,
            2,
            TreeWeights {
                b: (14, 4, 10),
                cf: 4,
                ce: 8,
            },
            4,
        )
        .run();
        // Validator 3 (8) voted for CE, tipping B to FULL.
        get_gloas_optimistic_reset_excludes_equivocators_test_definition(
            3,
            4,
            TreeWeights {
                b: (7, 5, 2),
                cf: 4,
                ce: 0,
            },
            3,
        )
        .run();
    }

    #[test]
    fn optimistic_reset_after_reload() {
        get_gloas_optimistic_reset_after_reload_test_definition().run();
    }

    #[test]
    fn optimistic_reset_is_idempotent() {
        get_gloas_optimistic_reset_is_idempotent_test_definition().run();
    }

    #[test]
    fn optimistic_reset_without_invalid_payload() {
        get_gloas_optimistic_reset_without_invalid_payload_test_definition().run();
    }
}
