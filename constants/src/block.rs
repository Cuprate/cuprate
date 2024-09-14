//! TODO

use crate::{difficulty, macros::monero_definition_link};

/// The maximum block height possible.
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/cryptonote_config.h", 40)]
pub const MAX_BLOCK_HEIGHT: usize = 500_000_000;

/// TODO
pub const CURRENT_BLOCK_MAJOR_VERSION: u64 = 1;

/// TODO
pub const CURRENT_BLOCK_MINOR_VERSION: u64 = 0;

/// TODO
pub const CRYPTONOTE_BLOCK_FUTURE_TIME_LIMIT: u64 = 60 * 60 * 2;

/// TODO
pub const CRYPTONOTE_REWARD_BLOCKS_WINDOW: u64 = 100;

/// TODO
pub const CRYPTONOTE_BLOCK_GRANTED_FULL_REWARD_ZONE_V2: u64 = 60000; //size of block (bytes) after which reward for block calculated ;using block size

/// TODO
pub const CRYPTONOTE_BLOCK_GRANTED_FULL_REWARD_ZONE_V1: u64 = 20000; //size of block (bytes) after which reward for block calculated ;using block size - before first fork

/// TODO
pub const CRYPTONOTE_BLOCK_GRANTED_FULL_REWARD_ZONE_V5: u64 = 300000; //size of block (bytes) after which reward for block calculated ;using block size - second change, from v5

/// TODO
pub const CRYPTONOTE_LONG_TERM_BLOCK_WEIGHT_WINDOW_SIZE: u64 = 100000; // size in blocks of the long term block weight median window;

/// TODO
pub const CRYPTONOTE_SHORT_TERM_BLOCK_WEIGHT_SURGE_FACTOR: u64 = 50;

/// TODO
pub const ORPHANED_BLOCKS_MAX_COUNT: u64 = 100;

/// TODO
pub const CRYPTONOTE_COINBASE_BLOB_RESERVED_SIZE: u64 = 600;

/// TODO
pub const DIFFICULTY_BLOCKS_ESTIMATE_TIMESPAN: u64 = difficulty::DIFFICULTY_TARGET_V1.as_secs(); //just alias; used by tests;

/// TODO
pub const BLOCKS_IDS_SYNCHRONIZING_DEFAULT_COUNT: u64 = 10000; //by default, blocks ids count in synchronizing;

/// TODO
pub const BLOCKS_IDS_SYNCHRONIZING_MAX_COUNT: u64 = 25000; //max blocks ids count in synchronizing;

/// TODO
pub const BLOCKS_SYNCHRONIZING_DEFAULT_COUNT_PRE_V4: u64 = 100; //by default, blocks count in blocks downloading;

/// TODO
pub const BLOCKS_SYNCHRONIZING_DEFAULT_COUNT: u64 = 20; //by default, blocks count in blocks downloading;

/// TODO
pub const BLOCKS_SYNCHRONIZING_MAX_COUNT: u64 = 2048; //must be a power of 2, greater than 128, equal to ;SEEDHASH_EPOCH_BLOCKS

/// TODO
pub const BULLETPROOF_MAX_OUTPUTS: u64 = 16;

/// TODO
pub const BULLETPROOF_PLUS_MAX_OUTPUTS: u64 = 16;
