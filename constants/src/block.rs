//! Block related.

use core::time::Duration;

use crate::macros::monero_definition_link;

/// The maximum block height possible.
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/cryptonote_config.h", 40)]
pub const MAX_BLOCK_HEIGHT: u64 = 500_000_000;

/// [`MAX_BLOCK_HEIGHT`] as a [`usize`].
#[expect(clippy::cast_possible_truncation, reason = "will not be truncated")]
pub const MAX_BLOCK_HEIGHT_USIZE: usize = MAX_BLOCK_HEIGHT as usize;

/// Target block time for hardfork 1.
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/cryptonote_config.h", 81)]
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/difficulty.html#target-seconds>
pub const BLOCK_TIME_V1: Duration = Duration::from_secs(60);

/// Target block time from hardfork v2.
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/cryptonote_config.h", 80)]
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/difficulty.html#target-seconds>
pub const BLOCK_TIME_V2: Duration = Duration::from_secs(120);
