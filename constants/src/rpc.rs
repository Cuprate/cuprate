//! TODO
//!
//! - <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L63-L77>
//! - <https://github.com/monero-project/monero/blob/a1dc85c5373a30f14aaf7dcfdd95f5a7375d3623/src/cryptonote_core/cryptonote_core.cpp#L63-L72>

/// TODO
pub const MAX_RESTRICTED_FAKE_OUTS_COUNT: usize = 40;

/// TODO
pub const MAX_RESTRICTED_GLOBAL_FAKE_OUTS_COUNT: usize = 5000;

/// 3 days max, the wallet requests 1.8: usize = days.
pub const OUTPUT_HISTOGRAM_RECENT_CUTOFF_RESTRICTION: usize = 3 * 86400;

/// TODO
pub const RESTRICTED_BLOCK_HEADER_RANGE: u64 = 1000;

/// TODO
pub const RESTRICTED_TRANSACTIONS_COUNT: usize = 100;

/// TODO
pub const RESTRICTED_SPENT_KEY_IMAGES_COUNT: usize = 5000;

/// TODO
pub const RESTRICTED_BLOCK_COUNT: usize = 1000;

/// TODO
pub const BLOCK_SIZE_SANITY_LEEWAY: usize = 100;

/// TODO
pub const COMMAND_RPC_GET_BLOCKS_FAST_MAX_BLOCK_COUNT: u64 = 1000;

/// TODO
pub const COMMAND_RPC_GET_BLOCKS_FAST_MAX_TX_COUNT: u64 = 20_000;

/// TODO
pub const MAX_RPC_CONTENT_LENGTH: u64 = 1_048_576; // 1 MB

/// TODO
pub const RPC_IP_FAILS_BEFORE_BLOCK: u64 = 3;
