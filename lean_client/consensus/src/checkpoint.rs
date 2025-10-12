use crate::Hash256;
use crate::slot::Slot;
use serde::{Deserialize, Serialize};
use ssz_derive::{Decode, Encode};
use tree_hash_derive::TreeHash;

#[derive(
    Clone,
    Debug,
    Copy,
    Eq,
    PartialEq,
    Default,
    Hash,
    Serialize,
    Deserialize,
    Encode,
    Decode,
    TreeHash,
)]
pub struct Checkpoint {
    pub root: Hash256,
    pub slot: Slot,
}

impl Checkpoint {
    pub fn new(root: Hash256, slot: Slot) -> Self {
        Self { root, slot }
    }
}
