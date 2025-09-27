use std::collections::HashMap;
use types::{
    Attestation, AttestationData, BeaconBlock, BeaconState, ChainSpec, Checkpoint, Epoch, EthSpec,
    Hash256, IndexedAttestation, SignedBeaconBlock, Slot,
};

use crate::constants::ZERO_HASH;

/// Attestation vote for the fork choice algorithm
#[derive(Clone, Debug, PartialEq)]
pub struct Vote {
    pub current_root: Hash256,
    pub next_root: Hash256,
    pub next_epoch: Epoch,
}

/// Fork choice store tracking chain state and validator votes
#[derive(Clone, Debug)]
pub struct Store<E: EthSpec> {
    /// Current time tracked by the store
    pub time: Slot,
    /// Genesis time
    pub genesis_time: u64,
    /// Current justified checkpoint
    pub justified_checkpoint: Checkpoint,
    /// Current finalized checkpoint
    pub finalized_checkpoint: Checkpoint,
    /// Best justified checkpoint (may not be current justified)
    pub best_justified_checkpoint: Checkpoint,
    /// Blocks tracked by the store
    pub blocks: HashMap<Hash256, BeaconBlock<E>>,
    /// Block states
    pub block_states: HashMap<Hash256, BeaconState<E>>,
    /// Checkpoint states
    pub checkpoint_states: HashMap<Checkpoint, BeaconState<E>>,
    /// Latest messages from validators (votes)
    pub latest_messages: HashMap<u64, Vote>,
}

impl<E: EthSpec> Store<E> {
    /// Initialize fork choice store from an anchor state and block
    pub fn get_forkchoice_store(
        anchor_state: &BeaconState<E>,
        anchor_block: &BeaconBlock<E>,
    ) -> Result<Self, String> {
        let anchor_root = anchor_block.canonical_root();
        let anchor_epoch = anchor_state.current_epoch();
        let justified_checkpoint = Checkpoint {
            epoch: anchor_epoch,
            root: anchor_root,
        };
        let finalized_checkpoint = Checkpoint {
            epoch: anchor_epoch,
            root: anchor_root,
        };

        let mut blocks = HashMap::new();
        blocks.insert(anchor_root, anchor_block.clone());

        let mut block_states = HashMap::new();
        block_states.insert(anchor_root, anchor_state.clone());

        let mut checkpoint_states = HashMap::new();
        checkpoint_states.insert(justified_checkpoint, anchor_state.clone());
        checkpoint_states.insert(finalized_checkpoint, anchor_state.clone());

        Ok(Store {
            time: anchor_state.slot(),
            genesis_time: anchor_state.genesis_time(),
            justified_checkpoint,
            finalized_checkpoint,
            best_justified_checkpoint: justified_checkpoint,
            blocks,
            block_states,
            checkpoint_states,
            latest_messages: HashMap::new(),
        })
    }

    /// Validate an attestation against the fork choice rules
    pub fn validate_attestation(&self, attestation: &Attestation<E>) -> Result<(), String> {
        let target = &attestation.data().target;

        // Check that the attestation is for a known block
        if !self.blocks.contains_key(&target.root) {
            return Err("Unknown target block".to_string());
        }

        // Additional validation logic would go here
        Ok(())
    }

    /// Process an attestation and update validator votes
    pub fn process_attestation(&mut self, attestation: &Attestation<E>) -> Result<(), String> {
        self.validate_attestation(attestation)?;

        let target = &attestation.data().target;
        let indexed_attestation = self.get_indexed_attestation(attestation)?;

        // Update latest messages for each attesting validator
        for validator_index in indexed_attestation.attesting_indices_to_vec() {
            let vote = Vote {
                current_root: target.root,
                next_root: target.root,
                next_epoch: target.epoch,
            };
            self.latest_messages.insert(validator_index, vote);
        }

        Ok(())
    }

    /// Process a new block and add it to the store
    pub fn process_block(&mut self, signed_block: &SignedBeaconBlock<E>) -> Result<(), String> {
        let block_root = signed_block.canonical_root();

        // For now, we'll store a placeholder since we have type issues with refs
        // This would need proper conversion from SignedBeaconBlock to BeaconBlock
        // self.blocks.insert(block_root, ...);

        // Placeholder implementation
        let _ = block_root;

        Ok(())
    }

    /// Update the head of the chain based on fork choice rule
    pub fn update_head(&mut self) -> Result<Hash256, String> {
        // Start from justified checkpoint
        let justified_root = self.justified_checkpoint.root;

        // Apply LMD GHOST algorithm
        let head = self.get_head(justified_root)?;
        Ok(head)
    }

    /// Advance store time and perform interval-based operations
    pub fn advance_time(&mut self, time: Slot) -> Result<(), String> {
        if time < self.time {
            return Err("Cannot advance time backwards".to_string());
        }

        self.time = time;

        // Update justified checkpoint if a better one is available
        if self.best_justified_checkpoint.epoch > self.justified_checkpoint.epoch {
            self.justified_checkpoint = self.best_justified_checkpoint;
        }

        Ok(())
    }

    /// Produce a new block for the given slot
    pub fn produce_block(
        &self,
        slot: Slot,
        validator_index: u64,
    ) -> Result<BeaconBlock<E>, String> {
        // Get current head
        let head_root = self.get_head(self.justified_checkpoint.root)?;
        let _parent_state = self
            .block_states
            .get(&head_root)
            .ok_or("Head state not found")?;

        // Create new block with proper parent
        let spec = ChainSpec::minimal();
        let mut block = BeaconBlock::empty(&spec);
        *block.slot_mut() = slot;
        *block.parent_root_mut() = head_root;
        *block.proposer_index_mut() = validator_index;

        Ok(block)
    }

    /// Generate an attestation vote for a validator
    pub fn produce_attestation_vote(
        &self,
        slot: Slot,
        committee_index: u64,
        _validator_index: u64,
    ) -> Result<AttestationData, String> {
        let head_root = self.get_head(self.justified_checkpoint.root)?;
        let epoch = slot.epoch(E::slots_per_epoch());

        let attestation_data = AttestationData {
            slot,
            index: committee_index,
            beacon_block_root: head_root,
            source: self.justified_checkpoint,
            target: Checkpoint {
                epoch,
                root: head_root,
            },
        };

        Ok(attestation_data)
    }

    /// Get the head block using LMD GHOST algorithm
    fn get_head(&self, justified_root: Hash256) -> Result<Hash256, String> {
        let mut current = justified_root;

        // Traverse down the tree following the heaviest branch
        loop {
            let children = self.get_children(current);
            if children.is_empty() {
                return Ok(current);
            }

            // Find child with most weight
            let mut best_child = children[0];
            let mut best_weight = self.get_weight(children[0]);

            for &child in &children[1..] {
                let weight = self.get_weight(child);
                if weight > best_weight || (weight == best_weight && child > best_child) {
                    best_child = child;
                    best_weight = weight;
                }
            }

            current = best_child;
        }
    }

    /// Get children blocks of a given block
    fn get_children(&self, block_root: Hash256) -> Vec<Hash256> {
        self.blocks
            .iter()
            .filter_map(|(&root, block)| {
                if block.parent_root() == block_root {
                    Some(root)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get the vote weight for a block
    fn get_weight(&self, block_root: Hash256) -> u64 {
        self.latest_messages
            .values()
            .filter(|vote| self.is_ancestor_or_self(vote.current_root, block_root))
            .count() as u64
    }

    /// Check if ancestor is an ancestor of descendant (or the same block)
    fn is_ancestor_or_self(&self, ancestor: Hash256, descendant: Hash256) -> bool {
        if ancestor == descendant {
            return true;
        }

        let mut current = descendant;
        while let Some(block) = self.blocks.get(&current) {
            let parent = block.parent_root();
            if parent == ancestor {
                return true;
            }
            if parent == ZERO_HASH {
                return false;
            }
            current = parent;
        }
        false
    }

    /// Convert attestation to indexed attestation
    fn get_indexed_attestation(
        &self,
        _attestation: &Attestation<E>,
    ) -> Result<IndexedAttestation<E>, String> {
        // This would need to resolve the committee and create indexed attestation
        // For now, return a placeholder
        Err("Not implemented".to_string())
    }
}
