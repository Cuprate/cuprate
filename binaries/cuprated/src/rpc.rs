//! RPC
//!
//! Will contain the code to initiate the RPC and a request handler.

// TODO: remove after impl.
#![allow(dead_code, unused_variables, clippy::needless_pass_by_value)]

mod bin;
mod handler;
mod json;
mod other;

pub use handler::CupratedRpcHandler;
