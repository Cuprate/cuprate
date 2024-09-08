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
    unreachable_code,
    reason = "TODO: remove after v1.0.0"
)]

mod blockchain;
mod config;
mod p2p;
mod rpc;
mod txpool;

fn main() {
    todo!()
}
