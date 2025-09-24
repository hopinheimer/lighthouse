use crate::slot::Slot;

//TODO: SignedVote
#[derive(Clone, Debug)]
pub struct BlockBody {}

#[derive(Clone, Debug, Hash)]
pub struct LeanBlockHeader {
    pub slot: Slot,
    pub proposer_index: u64,
    pub parent_root: [u8; 32],
    pub state_root: [u8; 32],
    pub body_root: [u8; 32],
}

#[derive(Clone, Debug)]
pub struct LeanBlock {
    pub slot: Slot,
    pub proposer_index: u64,
    pub parent_root: [u8; 32],
    pub state_root: [u8; 32],
    pub body_root: [u8; 32],
    pub block_body: BlockBody,
}

#[derive(Clone, Debug)]
pub struct SignedBlock {
    pub message: LeanBlock,
    pub signature: [u8; 32],
}
