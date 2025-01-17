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
    clippy::diverging_sub_expression,
    unused_mut,
    clippy::let_unit_value,
    clippy::needless_pass_by_ref_mut,
    reason = "TODO: remove after v1.0.0"
)]

mod blockchain;
mod config;
mod constants;
mod killswitch;
mod p2p;
mod rpc;
mod signals;
mod statics;
mod txpool;
mod version;

fn main() {
    // Initialize the killswitch.
    killswitch::init_killswitch();

    // Initialize global static `LazyLock` data.
    statics::init_lazylock_statics();

    let _config = config::read_config_and_args();

    // TODO: everything else.
    todo!()
}
