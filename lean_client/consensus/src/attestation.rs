use std::marker::PhantomData;
use types::BitList;
use types::EthSpec;
pub struct Attestation<E: EthSpec> {
    validator_id: u64,
    attestation_data: AttestationData,
    _phantom_data: PhantomData<E>,
}

pub struct AttestationData {
    slot: Slot,
    head: Checkpoint,
    target: Checkpoint,
    source: Checkpoint,
}
pub struct Slot {}

pub struct Checkpoint {}

pub struct SignedAttestation<E: EthSpec> {
    message: Attestation<E>,
    signature: Signature,
}

pub struct Signature(u64);

pub struct AggregatedAttestations<E: EthSpec> {
    aggregation_bits: BitList<E::MaxValidatorsPerCommittee>,
    data: AttestationData,
}
pub struct SignedAggregatedAttestations<E: EthSpec> {
    aggregate_attestation: AggregatedAttestations<E>,
    signature: AggregatedSignatures,
}

pub struct AggregatedSignatures([Signature]);
