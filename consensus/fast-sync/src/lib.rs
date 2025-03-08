// Used in `create.rs`
use clap as _;
use cuprate_blockchain as _;
use hex as _;
use tokio as _;

mod fast_sync;

pub use fast_sync::{
    block_to_verified_block_information, fast_sync_stop_height, set_fast_sync_hashes,
    validate_entries, FAST_SYNC_BATCH_LEN,
};
