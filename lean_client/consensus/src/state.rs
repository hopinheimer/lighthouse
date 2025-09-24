use crate::checkpoint::Checkpoint;
use crate::config::LeanConfig;
use crate::lean_block::{LeanBlockHeader, SignedBlock};
use crate::slot::Slot;
use crate::vote::Vote;
use std::hash::{Hash, Hasher};
pub struct LeanState {
    config: LeanConfig,
    slot: Slot,
    latest_block_header: LeanBlockHeader,
    latest_justified: Checkpoint,
    latest_finalized: Checkpoint,
    historical_block_hashes: Vec<[u8; 32]>,
    justified_slots: Vec<bool>,
    justifications_roots: Vec<[u8; 32]>,
    justifications_validators: Vec<bool>,
}

impl LeanState {
    pub fn generate_genesis(config: LeanConfig) -> Self {
        Self {
            config,
            slot: Slot(0),
            latest_block_header: LeanBlockHeader {
                slot: Slot(0),
                proposer_index: 0,
                parent_root: [0u8; 32],
                state_root: [0u8; 32],
                body_root: [0u8; 32],
            },
            latest_justified: Checkpoint::new([0u8; 32], Slot(0)),
            latest_finalized: Checkpoint::new([0u8; 32], Slot(0)),
            historical_block_hashes: Vec::new(),
            justified_slots: Vec::new(),
            justifications_roots: Vec::new(),
            justifications_validators: Vec::new(),
        }
    }

    pub fn process_slot(&mut self, slot: Slot) {
        self.slot = slot;
    }

    pub fn process_block_header(
        &mut self,
        block_header: LeanBlockHeader,
    ) -> Result<(), &'static str> {
        if block_header.slot != self.slot {
            return Err("Block slot does not match state slot");
        }

        if block_header.parent_root != self.get_latest_block_root() {
            return Err("Block parent root does not match latest block root");
        }

        self.latest_block_header = block_header;
        Ok(())
    }

    pub fn process_attestations(&mut self, attestations: &[Vote]) {
        for attestation in attestations {
            self.apply_attestation(attestation);
        }
        self.update_justification();
    }

    pub fn state_transition(&mut self, signed_block: &SignedBlock) -> Result<(), &'static str> {
        let block_header = LeanBlockHeader {
            slot: signed_block.message.slot,
            proposer_index: signed_block.message.proposer_index,
            parent_root: signed_block.message.parent_root,
            state_root: signed_block.message.state_root,
            body_root: signed_block.message.body_root,
        };

        self.process_slot(signed_block.message.slot);
        self.process_block_header(block_header)?;

        self.historical_block_hashes
            .push(self.get_latest_block_root());

        Ok(())
    }

    fn apply_attestation(&mut self, attestation: &Vote) {
        let slot_index = attestation.slot.0 as usize;

        while self.justified_slots.len() <= slot_index {
            self.justified_slots.push(false);
        }

        while self.justifications_validators.len() <= attestation.validator_index as usize {
            self.justifications_validators.push(false);
        }

        self.justified_slots[slot_index] = true;
        self.justifications_validators[attestation.validator_index as usize] = true;

        if self.justifications_roots.len() <= slot_index {
            self.justifications_roots.resize(slot_index + 1, [0u8; 32]);
        }
        self.justifications_roots[slot_index] = attestation.block_root;
    }

    fn update_justification(&mut self) {
        let current_slot = self.slot.0;

        if current_slot >= 2 {
            let justified_count = self
                .justified_slots
                .iter()
                .skip((current_slot.saturating_sub(2)) as usize)
                .take(2)
                .filter(|&&justified| justified)
                .count();

            if justified_count >= 2 {
                self.latest_justified = Checkpoint::new(self.get_latest_block_root(), self.slot);
            }
        }

        if current_slot >= 3 {
            let finalized_count = self
                .justified_slots
                .iter()
                .skip((current_slot.saturating_sub(3)) as usize)
                .take(3)
                .filter(|&&justified| justified)
                .count();

            if finalized_count >= 3 {
                self.latest_finalized = Checkpoint::new(
                    self.get_latest_block_root(),
                    Slot(current_slot.saturating_sub(1)),
                );
            }
        }
    }

    fn get_latest_block_root(&self) -> [u8; 32] {
        let header = &self.latest_block_header;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();

        header.hash(&mut hasher);

        let hash = hasher.finish();
        let mut result = [0u8; 32];
        result[..8].copy_from_slice(&hash.to_le_bytes());
        result
    }

    pub fn slot(&self) -> Slot {
        self.slot
    }

    pub fn latest_justified(&self) -> &Checkpoint {
        &self.latest_justified
    }

    pub fn latest_finalized(&self) -> &Checkpoint {
        &self.latest_finalized
    }

    pub fn latest_block_header(&self) -> &LeanBlockHeader {
        &self.latest_block_header
    }

    pub fn config(&self) -> &LeanConfig {
        &self.config
    }
}
