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
    reason = "TODO: remove after v1.0.0"
)]

use tokio::runtime::Runtime;
use tracing::Level;

mod blockchain;
mod config;
mod constants;
mod p2p;
mod rpc;
mod signals;
mod statics;
mod txpool;

use config::Config;

fn main() {
    // Initialize global static `LazyLock` data.
    statics::init_lazylock_statics();

    let config = config::config();

    init_log(&config);

    let (mut bc_read_handle, mut bc_write_handle, _) =
        cuprate_blockchain::service::init(config.blockchain_config()).unwrap();

    let async_rt = init_tokio_rt(&config);

    async_rt.block_on(async move {
        blockchain:: check_add_genesis(&mut bc_read_handle, &mut bc_write_handle, &config.network()).await;

        let (block_verifier, _tx_verifier, context_svc) =
            blockchain::init_consensus(bc_read_handle.clone(), config.context_config())
                .await
                .unwrap();
    }

    // TODO: everything else.
    todo!()
}

fn init_log(_config: &Config) {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
}

fn init_tokio_rt(_config: &Config) -> Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
}
