use crate::*;

#[derive(Default, Clone, Debug, PartialEq)]
pub struct BeaconCommittee<'a> {
    pub slot: Slot,
    pub index: CommitteeIndex,
    pub committee: &'a [usize],
}

impl BeaconCommittee<'_> {
    pub fn into_owned(self) -> OwnedBeaconCommittee {
        OwnedBeaconCommittee {
            slot: self.slot,
            index: self.index,
            committee: self.committee.to_vec(),
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq)]
// #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct OwnedBeaconCommittee {
    pub slot: Slot,
    pub index: CommitteeIndex,
    pub committee: Vec<usize>,
}
