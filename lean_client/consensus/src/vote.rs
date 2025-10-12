use crate::checkpoint::Checkpoint;
use crate::slot::Slot;
use crate::Hash256;
use serde::{Deserialize, Serialize};
use ssz::{Decode, DecodeError, Encode};
use tree_hash::TreeHash;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vote {
    pub slot: Slot,
    pub head: Checkpoint,
    pub target: Checkpoint,
    pub source: Checkpoint,
    pub validator_index: u64,
    pub block_root: Hash256,
}

impl Vote {
    pub fn new(
        slot: Slot,
        head: Checkpoint,
        target: Checkpoint,
        source: Checkpoint,
        validator_index: u64,
        block_root: Hash256,
    ) -> Self {
        Self {
            slot,
            head,
            target,
            source,
            validator_index,
            block_root,
        }
    }
}

impl Encode for Vote {
    fn is_ssz_fixed_len() -> bool {
        true
    }

    fn ssz_fixed_len() -> usize {
        <Slot as Encode>::ssz_fixed_len() +
        <Checkpoint as Encode>::ssz_fixed_len() * 3 +
        <u64 as Encode>::ssz_fixed_len() +
        <Hash256 as Encode>::ssz_fixed_len()
    }

    fn ssz_bytes_len(&self) -> usize {
        self.slot.ssz_bytes_len() +
        self.head.ssz_bytes_len() +
        self.target.ssz_bytes_len() +
        self.source.ssz_bytes_len() +
        self.validator_index.ssz_bytes_len() +
        self.block_root.ssz_bytes_len()
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        self.slot.ssz_append(buf);
        self.head.ssz_append(buf);
        self.target.ssz_append(buf);
        self.source.ssz_append(buf);
        self.validator_index.ssz_append(buf);
        self.block_root.ssz_append(buf);
    }
}

impl Decode for Vote {
    fn is_ssz_fixed_len() -> bool {
        true
    }

    fn ssz_fixed_len() -> usize {
        <Slot as Decode>::ssz_fixed_len() +
        <Checkpoint as Decode>::ssz_fixed_len() * 3 +
        <u64 as Decode>::ssz_fixed_len() +
        <Hash256 as Decode>::ssz_fixed_len()
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut offset = 0;
        
        let slot = Slot::from_ssz_bytes(&bytes[offset..])?;
        offset += <Slot as Decode>::ssz_fixed_len();
        
        let head = Checkpoint::from_ssz_bytes(&bytes[offset..])?;
        offset += <Checkpoint as Decode>::ssz_fixed_len();
        
        let target = Checkpoint::from_ssz_bytes(&bytes[offset..])?;
        offset += <Checkpoint as Decode>::ssz_fixed_len();
        
        let source = Checkpoint::from_ssz_bytes(&bytes[offset..])?;
        offset += <Checkpoint as Decode>::ssz_fixed_len();
        
        let validator_index = u64::from_ssz_bytes(&bytes[offset..])?;
        offset += <u64 as Decode>::ssz_fixed_len();
        
        let block_root = Hash256::from_ssz_bytes(&bytes[offset..])?;
        
        Ok(Vote {
            slot,
            head,
            target,
            source,
            validator_index,
            block_root,
        })
    }
}

impl TreeHash for Vote {
    fn tree_hash_type() -> tree_hash::TreeHashType {
        tree_hash::TreeHashType::Container
    }

    fn tree_hash_packed_encoding(&self) -> tree_hash::PackedEncoding {
        unreachable!("Container types should not be packed")
    }

    fn tree_hash_packing_factor() -> usize {
        unreachable!("Container types should not be packed")
    }

    fn tree_hash_root(&self) -> tree_hash::Hash256 {
        let mut combined = Vec::new();
        combined.extend_from_slice(self.slot.tree_hash_root().as_slice());
        combined.extend_from_slice(self.head.tree_hash_root().as_slice());
        combined.extend_from_slice(self.target.tree_hash_root().as_slice());
        combined.extend_from_slice(self.source.tree_hash_root().as_slice());
        combined.extend_from_slice(self.validator_index.tree_hash_root().as_slice());
        combined.extend_from_slice(self.block_root.tree_hash_root().as_slice());
        tree_hash::merkle_root(&combined, 6)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignedVote {
    pub vote: Vote,
    pub signature: [u8; 32],
}

impl SignedVote {
    pub fn new(vote: Vote, signature: [u8; 32]) -> Self {
        Self { vote, signature }
    }
}

impl Encode for SignedVote {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn ssz_fixed_len() -> usize {
        0
    }

    fn ssz_bytes_len(&self) -> usize {
        self.vote.ssz_bytes_len() + self.signature.len()
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        self.vote.ssz_append(buf);
        buf.extend_from_slice(&self.signature);
    }
}

impl Decode for SignedVote {
    fn is_ssz_fixed_len() -> bool {
        false
    }

    fn ssz_fixed_len() -> usize {
        0
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, DecodeError> {
        let vote = Vote::from_ssz_bytes(bytes)?;
        let vote_len = vote.ssz_bytes_len();
        
        if bytes.len() < vote_len + 32 {
            return Err(DecodeError::InvalidByteLength {
                len: bytes.len(),
                expected: vote_len + 32,
            });
        }
        
        let mut signature = [0u8; 32];
        signature.copy_from_slice(&bytes[vote_len..vote_len + 32]);
        
        Ok(SignedVote { vote, signature })
    }
}

impl TreeHash for SignedVote {
    fn tree_hash_type() -> tree_hash::TreeHashType {
        tree_hash::TreeHashType::Container
    }

    fn tree_hash_packed_encoding(&self) -> tree_hash::PackedEncoding {
        unreachable!("Container types should not be packed")
    }

    fn tree_hash_packing_factor() -> usize {
        unreachable!("Container types should not be packed")
    }

    fn tree_hash_root(&self) -> tree_hash::Hash256 {
        let mut combined = Vec::new();
        combined.extend_from_slice(self.vote.tree_hash_root().as_slice());
        combined.extend_from_slice(&self.signature);
        tree_hash::merkle_root(&combined, 2)
    }
}
