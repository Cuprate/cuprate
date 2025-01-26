#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(
    clippy::missing_assert_message,
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::should_panic_without_expect,
    clippy::single_char_lifetime_names,
    missing_docs,
    unsafe_code,
    unused_results,
    missing_copy_implementations,
    missing_debug_implementations,
    reason = "Crate-specific lints. There should be good reasoning when removing these."
)]

mod route;
mod router_builder;
mod rpc_handler;
#[cfg(feature = "dummy")]
mod rpc_handler_dummy;
mod rpc_service;

pub use router_builder::RouterBuilder;
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
