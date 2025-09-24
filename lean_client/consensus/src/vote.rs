use crate::checkpoint::Checkpoint;
use crate::slot::Slot;

#[derive(Clone, Debug)]
pub struct Vote {
    pub slot: Slot,
    pub head: Checkpoint,
    pub target: Checkpoint,
    pub source: Checkpoint,
    pub validator_index: u64,
    pub block_root: [u8; 32],
}

pub struct SignedVote {
    pub vote: Vote,
    signature: [u8; 32],
}
