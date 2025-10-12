use serde::{Deserialize, Serialize};
use ssz::{Decode, DecodeError, Encode};
use tree_hash::TreeHash;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Slot(pub u64);

impl Slot {
    pub const fn new(slot: u64) -> Slot {
        Slot(slot)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl From<u64> for Slot {
    fn from(n: u64) -> Slot {
        Slot(n)
    }
}

impl From<Slot> for u64 {
    fn from(from: Slot) -> u64 {
        from.0
    }
}

impl Encode for Slot {
    fn is_ssz_fixed_len() -> bool {
        <u64 as Encode>::is_ssz_fixed_len()
    }

    fn ssz_fixed_len() -> usize {
        <u64 as Encode>::ssz_fixed_len()
    }

    fn ssz_bytes_len(&self) -> usize {
        0_u64.ssz_bytes_len()
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        self.0.ssz_append(buf)
    }
}

impl Decode for Slot {
    fn is_ssz_fixed_len() -> bool {
        <u64 as Decode>::is_ssz_fixed_len()
    }

    fn ssz_fixed_len() -> usize {
        <u64 as Decode>::ssz_fixed_len()
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, DecodeError> {
        Ok(Slot(u64::from_ssz_bytes(bytes)?))
    }
}

impl TreeHash for Slot {
    fn tree_hash_type() -> tree_hash::TreeHashType {
        tree_hash::TreeHashType::Basic
    }

    fn tree_hash_packed_encoding(&self) -> tree_hash::PackedEncoding {
        self.0.tree_hash_packed_encoding()
    }

    fn tree_hash_packing_factor() -> usize {
        32usize.wrapping_div(8)
    }

    fn tree_hash_root(&self) -> tree_hash::Hash256 {
        tree_hash::Hash256::from_slice(&int_to_bytes::int_to_fixed_bytes32(self.0))
    }
}
