//pub mod hardforks;
pub mod network;
pub mod pruning;

use std::fmt::Formatter;
//pub use hardforks::HardForks;
pub use network::Network;
pub use pruning::{PruningError, PruningSeed};

pub const CRYPTONOTE_MAX_BLOCK_NUMBER: u64 = 500000000;

// pruning
pub const CRYPTONOTE_PRUNING_LOG_STRIPES: u32 = 3;
pub const CRYPTONOTE_PRUNING_STRIPE_SIZE: u64 = 4096;
pub const CRYPTONOTE_PRUNING_TIP_BLOCKS: u64 = 5500;

#[derive(Debug, Clone)]
pub enum BlockID {
    Hash([u8; 32]),
    Height(u64),
}

impl std::fmt::Display for BlockID {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockID::Hash(hash) => f.write_str(&format!("Hash: {}", hex::encode(hash))),
            BlockID::Height(height) => f.write_str(&format!("Height: {}", height)),
        }
    }
}

impl From<u64> for BlockID {
    fn from(value: u64) -> Self {
        BlockID::Height(value)
    }
}
