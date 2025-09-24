use crate::slot::Slot;

//TODO: SignedVote
pub struct BlockBody {}

pub struct LeanBlockHeader {
    slot: Slot,
    proposer_index: u64,
    parent_root: [u8; 32],
    state_root: [u8; 32],
    body_root: [u8; 32],
}

pub struct LeanBlock {
    slot: Slot,
    proposer_index: u64,
    parent_root: [u8; 32],
    state_root: [u8; 32],
    body_root: [u8; 32],
    block_body: BlockBody,
}

pub struct SignedBlock {
    message: LeanBlock,
    signature: [u8; 32],
}
