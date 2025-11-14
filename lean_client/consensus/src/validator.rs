use ssz_derive::{Decode, Encode};
use crate::attestation::Slot;
use types::{FixedVector, typenum::U52};

use tree_hash_derive::TreeHash;
#[derive(Clone, PartialEq, Decode, Encode, TreeHash)]
pub struct Validator {
    pub pubkey: FixedVector<u8, U52>,
}

pub struct ValidatorIndex(pub u64);
impl ValidatorIndex {
    pub fn is_proposer(&self, slot: Slot, num_validators: u64) -> bool {
        slot.0 % num_validators == self.0
    }
}

