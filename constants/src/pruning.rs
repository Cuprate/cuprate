//! TODO

/// The default log stripes for Monero pruning.
pub const CRYPTONOTE_PRUNING_LOG_STRIPES: u32 = 3;

/// The amount of blocks that peers keep before another stripe starts storing blocks.
pub const CRYPTONOTE_PRUNING_STRIPE_SIZE: usize = 4096;

/// The amount of blocks from the top of the chain that should not be pruned.
pub const CRYPTONOTE_PRUNING_TIP_BLOCKS: usize = 5500;
