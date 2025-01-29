#![forbid(
    clippy::missing_assert_message,
    clippy::should_panic_without_expect,
    clippy::single_char_lifetime_names,
    unsafe_code,
    unused_results,
    missing_copy_implementations,
    missing_debug_implementations,
    reason = "Crate-specific lints. There should be good reasoning when removing these."
)]

// Used in `create.rs`
use clap as _;
use cuprate_blockchain as _;
use hex as _;
use tokio as _;

pub mod fast_sync;
pub mod util;

pub use util::{hash_of_hashes, BlockId, HashOfHashes};
