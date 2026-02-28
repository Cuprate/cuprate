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

use tokio::sync::{mpsc, oneshot};
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
    config::Config, constants::PANIC_CRITICAL_SERVICE_ERROR, logging::CupratedTracingFilter,
    tor::initialize_tor_if_enabled,
};

mod blockchain;
mod commands;
mod config;
mod constants;
mod error;
mod logging;
mod p2p;
mod rpc;
mod signals;
mod statics;
mod supervisor;
mod tor;
mod txpool;
mod version;

fn main() {
    // Set global private permissions for created files.
    cuprate_helper::fs::set_private_global_file_permissions();

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
        // Create the supervisor and task handle, spawn signal handlers.
        let (supervisor, task) = supervisor::new();
        supervisor::spawn_signal_handler(supervisor.shutdown_handle.clone());

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
        let tor_enabled = config.p2p.tor_net.enabled;

        // Start clearnet P2P zone
        let (clearnet_interface, clearnet_tx_handler_subscriber) = p2p::initialize_clearnet_p2p(
            &config,
            context_svc.clone(),
            blockchain_read_handle.clone(),
            txpool_read_handle.clone(),
            &tor_context,
            task.shutdown_handle.clone(),
        )
        .await;

        // Create Tor router delivery channel.
        let (tor_router_tx, tor_router_rx) = tor_enabled.then(oneshot::channel).unzip();

        // Create the incoming tx handler service.
        let tx_handler = IncomingTxHandler::init(
            config.storage.txpool.clone(),
            clearnet_interface.clone(),
            tor_router_rx,
            txpool_write_handle.clone(),
            txpool_read_handle.clone(),
            context_svc.clone(),
            blockchain_read_handle.clone(),
            task.clone(),
        )
        .await;

        // Send tx handler sender to clearnet zone
        if clearnet_tx_handler_subscriber
            .send(tx_handler.clone())
            .is_err()
        {
            unreachable!()
        }

        // Initialize the blockchain manager.
        let synced_notify = blockchain::init_blockchain_manager(
            clearnet_interface,
            blockchain_write_handle,
            blockchain_read_handle.clone(),
            tx_handler.txpool_manager.clone(),
            context_svc.clone(),
            config.block_downloader_config(),
            task.clone(),
        )
        .await;

        // Initialize the RPC server(s).
        rpc::init_rpc_servers(
            config.rpc.clone(),
            config.network,
            blockchain_read_handle.clone(),
            context_svc.clone(),
            txpool_read_handle.clone(),
            tx_handler.clone(),
            task.clone(),
        );

        // Start Tor P2P zone after sync completes.
        if tor_enabled {
            info!("Tor P2P zone will start after sync.");
            let context_svc = context_svc.clone();
            let shutdown_handle = task.shutdown_handle.clone();

            task.task_tracker.spawn(async move {
                // Wait for the node to synchronize with the network, or shutdown.
                tokio::select! {
                    () = synced_notify.notified() => {}
                    () = shutdown_handle.cancelled() => {
                        return;
                    }
                }
                tracing::info!("Starting Tor P2P zone.");

                let (tor_interface, tor_tx_handler_tx) = p2p::start_tor_p2p(
                    &config,
                    context_svc,
                    blockchain_read_handle,
                    txpool_read_handle,
                    tor_context,
                    shutdown_handle.clone(),
                )
                .await;

                // Send the tx handler to the Tor zone
                if tor_tx_handler_tx.send(tx_handler).is_err() {
                    tracing::warn!("Failed to send tx handler to Tor zone.");
                    return;
                }

                // Deliver the Tor network interface to the dandelion router.
                if let Some(tx) = tor_router_tx {
                    if tx.send(tor_interface).is_err() {
                        tracing::warn!("Failed to deliver Tor router to dandelion pool.");
                    }
                }
            });
        }

        // Start the command listener.
        if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
            let (command_tx, command_rx) = mpsc::channel(1);
            std::thread::spawn(|| commands::command_listener(command_tx));

            // Wait on the io_loop, spawned on a separate task as this improves performance.
            task.task_tracker.spawn(commands::io_loop(
                command_rx,
                context_svc,
                task.shutdown_handle.clone(),
            ));
        } else {
            info!("Terminal/TTY not detected, disabling STDIN commands");
        }
        // Wait for shutdown (signal, command, or critical task failure).
        supervisor.shutdown_handle.cancelled().await;
        supervisor.task_tracker.close();
        supervisor.task_tracker.wait().await;
    });
    drop(rt);
    info!("Shutdown complete.");
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
