#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(
    unused_imports,
    unreachable_pub,
    unreachable_code,
    unused_crate_dependencies,
    dead_code,
    unused_variables,
    clippy::needless_pass_by_value,
    clippy::unused_async,
    reason = "TODO: remove after v1.0.0"
)]

mod blockchain;
mod config;
mod constants;
mod p2p;
mod rpc;
mod signals;
mod statics;
mod txpool;

fn main() {
    // Initialize global static `LazyLock` data.
    statics::init_lazylock_statics();

    // TODO: everything else.
    todo!()
}
