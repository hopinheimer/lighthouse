pub mod checkpoint;
pub mod config;
pub mod lean_block;
pub mod slot;
pub mod state;
pub mod vote;

// Re-export main types for easier usage
pub use checkpoint::Checkpoint;
pub use config::LeanConfig;
pub use lean_block::{BlockBody, LeanBlock, LeanBlockHeader, SignedBlock};
pub use slot::Slot;
pub use state::LeanState;
pub use vote::{SignedVote, Vote};

pub type Hash256 = fixed_bytes::Hash256;
