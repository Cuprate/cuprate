//! TODO

use std::time::Duration;

use crate::{difficulty, macros::monero_definition_link};

/// TODO
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/cryptonote_config.h", 41)]
pub const MAX_TX_SIZE: u64 = 1_000_000;

/// TODO
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/cryptonote_config.h", 42)]
pub const MAX_TX_PER_BLOCK: u64 = 0x10000000;

/// TODO
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/cryptonote_config.h", 44)]
pub const MINED_MONEY_UNLOCK_WINDOW: u64 = 60;

/// TODO
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/cryptonote_config.h", 45)]
pub const CURRENT_TRANSACTION_VERSION: u64 = 2;

/// TODO
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/cryptonote_config.h", 49)]
pub const DEFAULT_TX_SPENDABLE_AGE: u64 = 10;

/// TODO
pub const LOCKED_TX_ALLOWED_DELTA_SECONDS_V1: u64 =
    difficulty::DIFFICULTY_TARGET_V1.as_secs() * LOCKED_TX_ALLOWED_DELTA_BLOCKS;

/// TODO
pub const LOCKED_TX_ALLOWED_DELTA_SECONDS_V2: u64 =
    difficulty::DIFFICULTY_TARGET_V2.as_secs() * LOCKED_TX_ALLOWED_DELTA_BLOCKS;

/// TODO
pub const LOCKED_TX_ALLOWED_DELTA_BLOCKS: u64 = 1;

/// TODO
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/cryptonote_config.h", 213)]
pub const MAX_TX_EXTRA_SIZE: u64 = 1060;

/// Three days.
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/cryptonote_config.h", 102)]
///
/// ```rust
/// # use cuprate_constants::tx::*;
/// assert_eq!(MEMPOOL_TX_LIFETIME.as_secs(), 86_400 * 3);
/// ```
pub const MEMPOOL_TX_LIFETIME: Duration = Duration::from_secs(86_400 * 3);

/// One week.
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/cryptonote_config.h", 103)]
///
/// ```rust
/// # use cuprate_constants::tx::*;
/// assert_eq!(MEMPOOL_TX_FROM_ALT_BLOCK_LIVETIME.as_secs(), 86_400 * 7);
/// ```
pub const MEMPOOL_TX_FROM_ALT_BLOCK_LIVETIME: u64 = 86_400 * 7;

/// TODO
pub const PER_KB_FEE_QUANTIZATION_DECIMALS: u64 = 8;

/// TODO
pub const SCALING_2021_FEE_ROUNDING_PLACES: u64 = 2;

/// TODO
pub const DEFAULT_TXPOOL_MAX_WEIGHT: u64 = 648_000_000; // 3 days at 300000, in bytes
