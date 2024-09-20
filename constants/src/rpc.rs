//! RPC related.

use std::time::Duration;

use crate::macros::monero_definition_link;

/// Maximum requestable block header range.
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/rpc/core_rpc_server.cpp", 74)]
///
/// This is the maximum amount of blocks that can be requested
/// per invocation of `get_block_headers` if the RPC server is
/// in restricted mode.
///
/// Used at:
/// - <https://github.com/monero-project/monero/blob/a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623/src/rpc/core_rpc_server.cpp#L2593>
pub const RESTRICTED_BLOCK_HEADER_RANGE: u64 = 1000;

/// Maximum requestable transaction count.
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/rpc/core_rpc_server.cpp", 75)]
///
/// This is the maximum amount of transactions that can be requested
/// per invocation of `get_transactions` and `get_indexes` if the
/// RPC server is in restricted mode.
///
/// Used at:
/// - <https://github.com/monero-project/monero/blob/a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623/src/rpc/core_rpc_server.cpp#L660>
/// - <https://github.com/monero-project/monero/blob/a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623/src/rpc/core_rpc_server.cpp#L998>
pub const RESTRICTED_TRANSACTIONS_COUNT: usize = 100;

/// Maximum amount of requestable key image checks.
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/rpc/core_rpc_server.cpp", 76)]
///
/// This is the maximum amount of key images that can be requested
/// to be checked per `/is_key_image_spent` call if the RPC server
/// is in restricted mode.
///
/// Used at:
/// - <https://github.com/monero-project/monero/blob/a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623/src/rpc/core_rpc_server.cpp#L1248>
/// - <https://github.com/monero-project/monero/blob/a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623/src/rpc/core_rpc_server.cpp#L3570>
pub const RESTRICTED_SPENT_KEY_IMAGES_COUNT: usize = 5000;

/// Maximum amount of requestable blocks.
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/rpc/core_rpc_server.cpp", 77)]
///
/// This is the maximum amount of blocks that can be
/// requested if the RPC server is in restricted mode.
///
/// Used at:
/// - <https://github.com/monero-project/monero/blob/a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623/src/rpc/core_rpc_server.cpp#L834>
/// - <https://github.com/monero-project/monero/blob/a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623/src/rpc/core_rpc_server.cpp#L2519>
pub const RESTRICTED_BLOCK_COUNT: usize = 1000;

/// Maximum amount of fake outputs.
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/rpc/core_rpc_server.cpp", 67)]
///
/// This is the maximum amount of outputs that can be
/// requested if the RPC server is in restricted mode.
///
/// Used at:
/// - <https://github.com/monero-project/monero/blob/a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623/src/rpc/core_rpc_server.cpp#L905>
/// - <https://github.com/monero-project/monero/blob/a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623/src/rpc/core_rpc_server.cpp#L935>
pub const MAX_RESTRICTED_GLOBAL_FAKE_OUTS_COUNT: usize = 5000;

/// Maximum output histrogram cutoff.
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/rpc/core_rpc_server.cpp", 69)]
///
/// This is the maximum cutoff duration allowed in `get_output_histogram` (3 days).
///
/// ```rust
/// # use cuprate_constants::rpc::*;
/// assert_eq!(OUTPUT_HISTOGRAM_RECENT_CUTOFF_RESTRICTION.as_secs(), 86_400 * 3);
/// ```
///
/// Used at:
/// <https://github.com/monero-project/monero/blob/a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623/src/rpc/core_rpc_server.cpp#L2961>
pub const OUTPUT_HISTOGRAM_RECENT_CUTOFF_RESTRICTION: Duration = Duration::from_secs(86400 * 3);

/// Maximum amount of requestable blocks in `/get_blocks.bin`.
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/cryptonote_config.h", 128)]
pub const GET_BLOCKS_BIN_MAX_BLOCK_COUNT: u64 = 1000;

/// Maximum amount of requestable transactions in `/get_blocks.bin`.
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/cryptonote_config.h", 129)]
pub const GET_BLOCKS_BIN_MAX_TX_COUNT: u64 = 20_000;

/// Max message content length in the RPC server.
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/cryptonote_config.h", 130)]
///
/// This is the maximum amount of bytes an HTTP request
/// body can be before the RPC server rejects it (1 megabyte).
pub const MAX_RPC_CONTENT_LENGTH: u64 = 1_048_576;

/// Amount of fails before blocking a remote RPC server.
#[doc = monero_definition_link!(a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623, "/src/cryptonote_config.h", 159)]
///
/// This is the amount of times an RPC will attempt to
/// connect to another remote IP before blocking it.
///
/// RPC servers connect to nodes when they themselves
/// lack the data to fulfill the response.
pub const RPC_IP_FAILS_BEFORE_BLOCK: u64 = 3;
