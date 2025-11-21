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

use std::{mem, sync::Arc};

use p2p::initialize_zones_p2p;
use tokio::sync::mpsc;
use tokio_util::task::TaskTracker;
use tower::{Service, ServiceExt};
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::{layer::SubscriberExt, reload::Handle, util::SubscriberInitExt, Registry};

use cuprate_consensus_context::{
    BlockChainContextRequest, BlockChainContextResponse, BlockchainContextService,
};
use cuprate_database::{InitError, DATABASE_CORRUPT_MSG};
use cuprate_helper::time::secs_to_hms;
use cuprate_p2p_core::{transports::Tcp, ClearNet};
use cuprate_types::blockchain::BlockchainWriteRequest;
use txpool::IncomingTxHandler;

use crate::{
    config::Config,
    constants::PANIC_CRITICAL_SERVICE_ERROR,
    logging::CupratedTracingFilter,
    tor::{initialize_tor_if_enabled, TorMode},
};
use crate::blockchain::handle::BlockchainManagerHandle;

mod blockchain;
mod commands;
mod config;
mod constants;
mod logging;
mod p2p;
mod rpc;
mod signals;
mod statics;
mod tor;
mod txpool;
mod version;
mod monitor;

fn main() {
    // Initialize global static `LazyLock` data.
    statics::init_lazylock_statics();

    let config = config::read_config_and_args();

    blockchain::set_fast_sync_hashes(config.fast_sync, config.network());

    // Initialize logging.
    logging::init_logging(&config);

    //Printing configuration
    info!("{config}");

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
        .inspect_err(|e| error!("Blockchain database error: {e}"))
        .expect(DATABASE_CORRUPT_MSG);

    let (txpool_read_handle, txpool_write_handle, _) =
        cuprate_txpool::service::init_with_pool(&config.txpool_config(), db_thread_pool)
            .inspect_err(|e| error!("Txpool database error: {e}"))
            .expect(DATABASE_CORRUPT_MSG);

    // Initialize async tasks.

    rt.block_on(async move {
        let (mut monitor, tasks) = monitor::new();
        
        // TODO: Add an argument/option for keeping alt blocks between restart.
        blockchain_write_handle
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(BlockchainWriteRequest::FlushAltBlocks)
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR);

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

        // Bootstrap or configure Tor if enabled.
        let tor_context = initialize_tor_if_enabled(&config).await;

        let (blockchain_manager_handle, blockchain_manager_handle_setter) = BlockchainManagerHandle::new();

        // Start p2p network zones
        let (network_interfaces, tx_handler_subscribers) = p2p::initialize_zones_p2p(
            &config,
            context_svc.clone(),
            blockchain_read_handle.clone(),
            txpool_read_handle.clone(),
            blockchain_manager_handle.clone(),
            tor_context,
        )
        .await;

        // Create the incoming tx handler service.
        let tx_handler = IncomingTxHandler::init(
            &tasks,
            config.storage.txpool.clone(),
            network_interfaces.clearnet_network_interface.clone(),
            network_interfaces.tor_network_interface,
            txpool_write_handle.clone(),
            txpool_read_handle.clone(),
            context_svc.clone(),
            blockchain_read_handle.clone(),
        )
        .await;

        // Send tx handler sender to all network zones
        for zone in tx_handler_subscribers {
            if zone.send(tx_handler.clone()).is_err() {
                unreachable!()
            }
        }

        // Initialize the blockchain manager.
        blockchain::init_blockchain_manager(
            &tasks,
            blockchain_manager_handle_setter,
            network_interfaces.clearnet_network_interface,
            blockchain_write_handle,
            blockchain_read_handle.clone(),
            tx_handler.txpool_manager.clone(),
            context_svc.clone(),
            config.block_downloader_config(),
        )
        .await;


        // Initialize the RPC server(s).
        rpc::init_rpc_servers(
            config.rpc,
            config.network,
            blockchain_read_handle,
            context_svc.clone(),
            txpool_read_handle,
            tx_handler,
        );

        // Start the command listener.
        if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
            let (command_tx, command_rx) = mpsc::channel(1);
            std::thread::spawn(|| commands::command_listener(command_tx));

            // Wait on the io_loop, spawned on a separate task as this improves performance.
            tokio::spawn(commands::io_loop(command_rx, context_svc, blockchain_manager_handle, monitor))
                .await
                .unwrap();
        } else {
            // If no STDIN, await OS exit signal.
            info!("Terminal/TTY not detected, disabling STDIN commands");
            tokio::signal::ctrl_c().await.unwrap();
        }
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
