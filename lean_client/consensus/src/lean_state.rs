use crate::attestation::{Checkpoint, Slot};
use tree_hash::TreeHash;

use crate::lean_block::LeanBlockBody;
use crate::validator::ValidatorIndex;
use crate::validator::Validator;

use types::VariableList;
use crate::lean_block::LeanBlockHeader;
use milhouse::List;
use types::{BitVector, EthSpec, Hash256};

pub struct LeanState<E: EthSpec> {
    pub config: Config,
    pub slot: Slot,

    pub latest_block_header: LeanBlockHeader,
    pub latest_justified: Checkpoint,
    pub latest_finalized: Checkpoint,
    //TODO: deal with this E: EthSpec
    pub historical_block_hashes: List<Hash256, E::HistoricalRootsLimit>,
    //TODO: the Justification needs to be different
    pub justified_slots: BitVector<E::JustificationBitsLength>,
    pub validators: List<Validator, E::ValidatorRegistryLimit>,
    pub justifications_roots: List<Hash256, E::JustificationBitsLength>,
    pub justifications_validators: BitVector<E::ValidatorRegistryLimit>,
}

impl<E: EthSpec> LeanState<E> {
    pub fn generate_genesis(&self, validators: List<Validator, E::ValidatorRegistryLimit>) -> Self {
        let genesis_config = Config {


        };
        let genesis_header = LeanBlockHeader{
            slot: Slot(0),
            proposer_index: ValidatorIndex(0),
            parent_root: Hash256::ZERO,
            state_root: Hash256::ZERO,
            body_root: LeanBlockBody::<E> {

                attestations: VariableList::empty()
            }.tree_hash_root()
        };

        Self{
            config: genesis_config,
            slot:Slot(0),
            latest_justified: Checkpoint::default(),
            latest_finalized: Checkpoint::default(),
            latest_block_header: genesis_header,
            historical_block_hashes: List::empty(),
            justified_slots: BitVector::default(),
            validators,
            justifications_roots: List::empty(),
            justifications_validators: BitVector::default(),


        }



    }

    pub fn is_proposer(&self) {

    }
    pub fn get_justifications(&self) {}
    pub fn with_justification(&self) {}
    pub fn process_slot(&self) {}
    pub fn process_slots(&self) {}
    pub fn process_block_header(&self) {}
    pub fn process_block(&self) {}
    pub fn process_attestations(&self) {}
    pub fn state_transistion(&self) {}
}

pub struct Config {}
