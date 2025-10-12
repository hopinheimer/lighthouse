pub mod checkpoint;
pub mod config;
pub mod lean_block;
pub mod slot;
pub mod state;
pub mod vote;

// Re-export main types for easier usage
pub use state::LeanState;
pub use config::LeanConfig;
pub use lean_block::{LeanBlock, BlockBody, LeanBlockHeader, SignedBlock};
pub use vote::{Vote, SignedVote};
pub use checkpoint::Checkpoint;
pub use slot::Slot;

pub type Hash256 = fixed_bytes::Hash256;
