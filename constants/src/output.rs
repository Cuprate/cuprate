//! Output related.

use crate::macros::monero_definition_link;

/// The minimum amount of blocks a coinbase output is locked for.
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/cryptonote_config.h", 40)]
pub const COINBASE_LOCK_WINDOW: usize = 60;

/// The minimum amount of blocks an output is locked for.
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/cryptonote_config.h", 49)]
pub const DEFAULT_LOCK_WINDOW: usize = 10;
