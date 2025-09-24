use crate::slot::Slot;
pub struct Checkpoint {
    root: [u8; 32],
    slot: Slot,
}
