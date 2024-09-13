#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(
    unused_imports,
    unreachable_pub,
    unused_crate_dependencies,
    dead_code,
    unused_variables,
    clippy::needless_pass_by_value,
    clippy::unused_async,
    clippy::diverging_sub_expression,
    unreachable_code,
    reason = "TODO: remove after v1.0.0"
)]

mod blockchain;
mod config;
mod p2p;
mod rpc;
mod statics;
mod txpool;
mod version;

use std::sync::LazyLock;

fn main() {
    // Initialize global static `LazyLock` data.
    statics::init_lazylock_statics();
}
