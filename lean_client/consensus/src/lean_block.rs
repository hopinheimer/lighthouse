use crate::attestation::{Attestation, Slot};
use types::EthSpec;

use types::Hash256;
pub struct LeanBlock<E: EthSpec> {
    slot: Slot,
    proposer_index: u64,
    parent_root: Hash256,
    state_root: Hash256,
    body: LeanBlockBody<E>,
}

pub struct LeanBlockBody<E: EthSpec> {
    attestations: Attestations<E>,
}

pub struct LeanBlockHeader {
    slot: Slot,
    proposer_index: u64,
    parent_root: Hash256,
    state_root: Hash256,
    body_root: Hash256,
}
pub struct LeanBlockWithAttestation<E: EthSpec> {
    block: Box<LeanBlock<E>>,
    proposer_attestation: Attestation<E>,
}
pub struct SignedLeanBlockWithAttestation<E: EthSpec> {
    message: LeanBlockWithAttestation<E>,
    signature: BlockSignature,
}
pub struct Attestations<E: EthSpec>([Attestation<E>]);
pub struct BlockSignature([u64]);
