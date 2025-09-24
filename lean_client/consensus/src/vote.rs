use crate::checkpoint::Checkpoint;
use crate::slot::Slot;
pub struct Vote {
    slot: Slot,
    head: Checkpoint,
    target: Checkpoint,
    source: Checkpoint,
}

pub struct SignedVote {
    pub vote: Vote,
    signature: [u8; 32],
}
