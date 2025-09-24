use crate::slot::Slot;

#[derive(Clone, Debug)]
pub struct Checkpoint {
    pub root: [u8; 32],
    pub slot: Slot,
}

impl Checkpoint {
    pub fn new(root: [u8; 32], slot: Slot) -> Self {
        Self { root, slot }
    }
}
