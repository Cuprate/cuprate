//! RPC
//!
//! Will contain the code to initiate the RPC and a request handler.

mod bin;
mod handler;
mod json;
mod other;

pub use handler::CupratedRpcHandler;
