//! RPC
//!
//! Will contain the code to initiate the RPC and a request handler.

mod bin;
mod constants;
mod handler;
mod helper;
mod json;
mod other;
mod request;

pub use handler::CupratedRpcHandler;
