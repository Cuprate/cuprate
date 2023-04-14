pub mod hardforks;
pub mod network;
pub mod pruning;

pub use pruning::{PruningSeed, PruningError};
pub use network::Network;
pub use hardforks::HardForks;

pub const CRYPTONOTE_MAX_BLOCK_NUMBER: u64 = 500000000;

// pruning
pub const CRYPTONOTE_PRUNING_LOG_STRIPES: u32 = 3;
pub const CRYPTONOTE_PRUNING_STRIPE_SIZE: u64 = 4096;
pub const CRYPTONOTE_PRUNING_TIP_BLOCKS: u64 = 5500;
