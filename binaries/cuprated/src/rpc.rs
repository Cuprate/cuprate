//! RPC
//!
//! Will contain the code to initiate the RPC and a request handler.

mod bin;
mod constants;
mod handler;
mod helper;
mod json;
mod other;

pub use constants::{
    DEFAULT_PAYMENT_CREDITS_PER_HASH, DEFAULT_PAYMENT_DIFFICULTY, MAX_RESTRICTED_FAKE_OUTS_COUNT,
    MAX_RESTRICTED_GLOBAL_FAKE_OUTS_COUNT, OUTPUT_HISTOGRAM_RECENT_CUTOFF_RESTRICTION,
    RESTRICTED_BLOCK_COUNT, RESTRICTED_BLOCK_HEADER_RANGE, RESTRICTED_SPENT_KEY_IMAGES_COUNT,
    RESTRICTED_TRANSACTIONS_COUNT,
};
pub use handler::{CupratedRpcHandler, CupratedRpcHandlerState};
