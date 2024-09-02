//! RPC
//!
//! Will contain the code to initiate the RPC and a request handler.

#![allow(clippy::needless_pass_by_value)] // TODO: remove after impl.

mod bin;
mod handler;
mod json;
mod other;

pub use handler::CupratedRpcHandler;
