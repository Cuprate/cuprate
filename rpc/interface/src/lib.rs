#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod route;
mod router_builder;
mod rpc_error;
mod rpc_handler;
#[cfg(feature = "dummy")]
mod rpc_handler_dummy;
mod rpc_request;
mod rpc_response;

pub use router_builder::RouterBuilder;
pub use rpc_error::RpcError;
pub use rpc_handler::RpcHandler;
#[cfg(feature = "dummy")]
pub use rpc_handler_dummy::RpcHandlerDummy;
pub use rpc_request::RpcRequest;
pub use rpc_response::RpcResponse;

// false-positive: used in `README.md`'s doc-test.
#[cfg(test)]
mod test {
    use axum as _;
    use serde_json as _;
    use tokio as _;
    use ureq as _;
}
