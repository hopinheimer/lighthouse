use crate::Hash256;
use crate::checkpoint::Checkpoint;
use crate::config::LeanConfig;
use crate::lean_block::{LeanBlockHeader, SignedBlock};
use crate::slot::Slot;
use crate::vote::Vote;
use tree_hash::TreeHash;

pub struct LeanState {
    config: LeanConfig,
    slot: Slot,
    latest_block_header: LeanBlockHeader,
    latest_justified: Checkpoint,
    latest_finalized: Checkpoint,
    historical_block_hashes: Hash256,
    justified_slots: Hash256,
    justifications_roots: Hash256,
    justifications_validators: Vec<bool>,
}
