//! TODO
//!
//! <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L63-L77>

pub const MAX_RESTRICTED_FAKE_OUTS_COUNT: usize = 40;
pub const MAX_RESTRICTED_GLOBAL_FAKE_OUTS_COUNT: usize = 5000;

/// 3 days max, the wallet requests 1.8: usize = days.
pub const OUTPUT_HISTOGRAM_RECENT_CUTOFF_RESTRICTION: usize = 3 * 86400;

pub const DEFAULT_PAYMENT_DIFFICULTY: usize = 1000;
pub const DEFAULT_PAYMENT_CREDITS_PER_HASH: usize = 100;

pub const RESTRICTED_BLOCK_HEADER_RANGE: u64 = 1000;
pub const RESTRICTED_TRANSACTIONS_COUNT: usize = 100;
pub const RESTRICTED_SPENT_KEY_IMAGES_COUNT: usize = 5000;
pub const RESTRICTED_BLOCK_COUNT: usize = 1000;
