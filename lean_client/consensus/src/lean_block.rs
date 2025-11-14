use crate::attestation::{Attestation, Slot};
use tree_hash_derive::TreeHash;
use crate::validator::ValidatorIndex;
use lean_crypto::Signature;

use crate::lean_state::LeanState;
use milhouse::List;
use types::{EthSpec, VariableList};

use types::Hash256;
pub struct LeanBlock<E: EthSpec> {
    pub slot: Slot,
    pub proposer_index: u64,
    pub parent_root: Hash256,
    pub state_root: Hash256,
    pub body: LeanBlockBody<E>,
}

#[derive(TreeHash)]
pub struct LeanBlockBody<E: EthSpec> {
    pub attestations: VariableList<Attestation, E::MaxAttestations>,
}

pub struct LeanBlockHeader {
    pub slot: Slot,
    pub proposer_index: ValidatorIndex,
    pub parent_root: Hash256,
    pub state_root: Hash256,
    pub body_root: Hash256,
}
pub struct LeanBlockWithAttestation<E: EthSpec> {
    pub block: Box<LeanBlock<E>>,
    pub proposer_attestation: Attestation,
}
pub struct SignedLeanBlockWithAttestation<E: EthSpec> {
    pub message: LeanBlockWithAttestation<E>,
    pub signature: List<Signature, E::ValidatorRegistryLimit>,
}

impl<E: EthSpec> SignedLeanBlockWithAttestation<E> {
    pub fn verify_signatures(self, _parent_state: LeanState<E>) -> Result<(), String> {
        Ok(())
    }
}
