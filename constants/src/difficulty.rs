//! TODO

use std::time::Duration;

use crate::macros::monero_definition_link;

/// TODO
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/cryptonote_config.h", 81)]
///
/// ```rust
/// # use cuprate_constants::difficulty::*;
/// assert_eq!(DIFFICULTY_TARGET_V1.as_secs(), 60);
/// ```
pub const DIFFICULTY_TARGET_V1: Duration = Duration::from_secs(60);

/// TODO
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/cryptonote_config.h", 80)]
///
/// ```rust
/// # use cuprate_constants::difficulty::*;
/// assert_eq!(DIFFICULTY_TARGET_V2.as_secs(), 120);
/// ```
pub const DIFFICULTY_TARGET_V2: Duration = Duration::from_secs(120);

/// TODO
pub const DIFFICULTY_WINDOW: u64 = 720; // blocks;

/// TODO
pub const DIFFICULTY_LAG: u64 = 15; // !!!;

/// TODO
pub const DIFFICULTY_CUT: u64 = 60; // timestamps to cut after sorting;

/// TODO
pub const DIFFICULTY_BLOCKS_COUNT: u64 = DIFFICULTY_WINDOW + DIFFICULTY_LAG;
