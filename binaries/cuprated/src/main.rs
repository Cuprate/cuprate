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

use std::mem;
use std::sync::Arc;
use tokio::sync::mpsc;
use tower::{Service, ServiceExt};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, reload::Handle, util::SubscriberInitExt, Registry};

use cuprate_consensus_context::{
    BlockChainContextRequest, BlockChainContextResponse, BlockchainContextService,
};
use cuprate_helper::time::secs_to_hms;

use crate::{
    config::Config, constants::PANIC_CRITICAL_SERVICE_ERROR, logging::CupratedTracingFilter,
};

mod blockchain;
mod commands;
mod config;
mod constants;
mod killswitch;
mod logging;
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

    let config = config::read_config_and_args();

    // Initialize logging.
    logging::init_logging(&config);

    // Initialize the thread-pools

    init_global_rayon_pool(&config);

    let rt = init_tokio_rt(&config);

    let db_thread_pool = cuprate_database_service::init_thread_pool(
        cuprate_database_service::ReaderThreads::Number(config.storage.reader_threads),
    );

    // Start the blockchain & tx-pool databases.

    let (mut blockchain_read_handle, mut blockchain_write_handle, _) =
        cuprate_blockchain::service::init_with_pool(
            config.blockchain_config(),
            Arc::clone(&db_thread_pool),
        )
        .unwrap();
    let (txpool_read_handle, txpool_write_handle, _) =
        cuprate_txpool::service::init_with_pool(config.txpool_config(), db_thread_pool).unwrap();

    // Initialize async tasks.

    rt.block_on(async move {
        // Check add the genesis block to the blockchain.
        blockchain::check_add_genesis(
            &mut blockchain_read_handle,
            &mut blockchain_write_handle,
            config.network(),
        )
        .await;

        // Start the context service and the block/tx verifier.
        let context_svc =
            blockchain::init_consensus(blockchain_read_handle.clone(), config.context_config())
                .await
                .unwrap();

        // Start clearnet P2P.
        let (clearnet, incoming_tx_handler_tx) = p2p::start_clearnet_p2p(
            blockchain_read_handle.clone(),
            context_svc.clone(),
            txpool_read_handle.clone(),
            config.clearnet_p2p_config(),
        )
        .await
        .unwrap();

        // Create the incoming tx handler service.
        let tx_handler = txpool::IncomingTxHandler::init(
            clearnet.clone(),
            txpool_write_handle.clone(),
            txpool_read_handle,
            context_svc.clone(),
            blockchain_read_handle.clone(),
        );
        if incoming_tx_handler_tx.send(tx_handler).is_err() {
            unreachable!()
        }

        // Initialize the blockchain manager.
        blockchain::init_blockchain_manager(
            clearnet,
            blockchain_write_handle,
            blockchain_read_handle,
            txpool_write_handle,
            context_svc.clone(),
            config.block_downloader_config(),
        )
        .await;

        // Start the command listener.
        let (command_tx, command_rx) = mpsc::channel(1);
        std::thread::spawn(|| commands::command_listener(command_tx));

        // Wait on the io_loop, spawned on a separate task as this improves performance.
        tokio::spawn(commands::io_loop(command_rx, context_svc))
            .await
            .unwrap();
    });
}

/// Initialize the [`tokio`] runtime.
fn init_tokio_rt(config: &Config) -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.tokio.threads)
        .thread_name("cuprated-tokio")
        .enable_all()
        .build()
        .unwrap()
}

/// Initialize the global [`rayon`] thread-pool.
fn init_global_rayon_pool(config: &Config) {
    rayon::ThreadPoolBuilder::new()
        .num_threads(config.rayon.threads)
        .thread_name(|index| format!("cuprated-rayon-{index}"))
        .build_global()
        .unwrap();
}
