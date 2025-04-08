//! RPC
//!
//! Will contain the code to initiate the RPC and a request handler.

mod constants;
mod handlers;
mod rpc_handler;
mod service;

pub use rpc_handler::CupratedRpcHandler;
