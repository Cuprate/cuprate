use std::sync::atomic::{AtomicU64, AtomicUsize};

/// Height at which RandomX activated.
pub const RANDOMX_START_HEIGHT: u64 = 1978433;

/// Total amount of blocks tested, used as a global counter.
pub static TESTED_BLOCK_COUNT: AtomicU64 = AtomicU64::new(0);

/// Total amount of transactions tested, used as a global counter.
pub static TESTED_TX_COUNT: AtomicUsize = AtomicUsize::new(0);
