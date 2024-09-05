#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod route;
mod router_builder;
mod rpc_error;
mod rpc_handler;
#[cfg(feature = "dummy")]
mod rpc_handler_dummy;
mod rpc_service;

pub use router_builder::RouterBuilder;
pub use rpc_error::RpcError;
pub use rpc_handler::RpcHandler;
#[cfg(feature = "dummy")]
pub use rpc_handler_dummy::RpcHandlerDummy;
pub use rpc_service::RpcService;

// false-positive: used in `README.md`'s doc-test.
#[cfg(test)]
mod test {
    extern crate axum;
    extern crate cuprate_test_utils;
    extern crate serde_json;
    extern crate tokio;
    extern crate ureq;
}
