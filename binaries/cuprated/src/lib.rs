//! `cuprated` library.
//!
//! Call [`Node::launch`] to initialize and run the node. Returns a [`Node`]
//! with handles to node services.
//!
//! # Example
//!
//! ```ignore
//! let config = cuprated::config::read_config_and_args();
//! cuprated::logging::init_logging(&config);
//!
//! let node = cuprated::Node::launch(config).await;
//! let height = node.context.blockchain_context().chain_height;
//! ```

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

pub mod blockchain;
pub mod commands;
pub mod config;
pub mod constants;
pub mod logging;
pub mod statics;
pub mod version;

mod p2p;
mod rpc;
mod signals;
mod tor;
mod txpool;

use std::sync::Arc;

use tokio::sync::oneshot;
use tower::{Service, ServiceExt};
use tracing::{error, info};

use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_consensus_context::BlockchainContextService;
use cuprate_p2p::NetworkInterface;
use cuprate_p2p_core::{ClearNet, Tor};
use cuprate_txpool::service::TxpoolReadHandle;
use cuprate_types::blockchain::BlockchainWriteRequest;

use crate::{
    blockchain::SyncState,
    commands::CommandHandler,
    config::Config,
    constants::{DATABASE_CORRUPT_MSG, PANIC_CRITICAL_SERVICE_ERROR},
    tor::initialize_tor_if_enabled,
    txpool::IncomingTxHandler,
};

/// An active `cuprated` node.
///
/// Returned by [`Node::launch`].
pub struct Node {
    /// Cached chain state (height, HF, difficulty, top hash).
    pub context: BlockchainContextService,

    /// Blockchain database queries (blocks, transactions).
    pub blockchain: BlockchainReadHandle,

    /// Transaction pool queries.
    pub txpool: TxpoolReadHandle,

    /// Clearnet P2P network.
    pub clearnet: NetworkInterface<ClearNet>,

    /// Tor P2P network (available after sync).
    pub tor: Option<oneshot::Receiver<NetworkInterface<Tor>>>,

    /// Sync state.
    pub sync: SyncState,

    /// Command channel.
    pub command: CommandHandler,
}

impl Node {
    /// Launch a new `cuprated` process.
    ///
    /// Sets up thread pools, databases, P2P networking, the blockchain manager,
    /// and RPC servers.
    ///
    /// The caller should set up tracing/logging before calling this, as the node
    /// emits tracing events during initialization.
    ///
    /// # Panics
    ///
    /// Panics if the database is corrupt or critical services fail to start.
    pub async fn launch(config: Config) -> Self {
        // Initialize global static `LazyLock` data.
        statics::init_lazylock_statics();

        // Initialize the thread-pools.
        blockchain::set_fast_sync_hashes(config.fast_sync, config.network());

        rayon::ThreadPoolBuilder::new()
            .num_threads(config.rayon.threads)
            .thread_name(|index| format!("cuprated-rayon-{index}"))
            .build_global()
            .unwrap();

        // Initialize the database thread pool.
        let db_thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(config.storage.reader_threads)
                .build()
                .unwrap(),
        );

        // Start the blockchain & tx-pool databases.
        let fjall_db = fjall::Database::builder(config.fjall_directory())
            .cache_size(config.fjall_cache_size())
            .open()
            .unwrap();

        let (mut blockchain_read_handle, mut blockchain_write_handle, _) =
            cuprate_blockchain::service::init_with_pool(
                &config.blockchain_config(),
                fjall_db.clone(),
                Arc::clone(&db_thread_pool),
            )
            .inspect_err(|e| error!("Blockchain database error: {e}"))
            .expect(DATABASE_CORRUPT_MSG);

        let (txpool_read_handle, txpool_write_handle) =
            cuprate_txpool::service::init_with_pool(fjall_db, db_thread_pool)
                .inspect_err(|e| error!("Txpool database error: {e}"))
                .expect(DATABASE_CORRUPT_MSG);

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

        // Create the sync notifier and handle.
        let (sync_state, syncer_handle) = SyncState::new();

        // Start clearnet P2P zone
        let (clearnet_interface, clearnet_tx_handler_subscriber) = p2p::initialize_clearnet_p2p(
            &config,
            context_svc.clone(),
            blockchain_read_handle.clone(),
            txpool_read_handle.clone(),
            &tor_context,
            sync_state.callback(context_svc.clone()),
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
        )
        .await;

        // Send tx handler sender to clearnet zone
        if clearnet_tx_handler_subscriber
            .send(tx_handler.clone())
            .is_err()
        {
            unreachable!()
        }

        // Tor interface channel - populated when Tor starts after sync.
        let (tor_tx, tor_rx) = oneshot::channel();

        // Command handler.
        let command_handler = CommandHandler::init(context_svc.clone());

        // Create the node struct with cloned service handles for the caller.
        let node = Self {
            context: context_svc.clone(),
            blockchain: blockchain_read_handle.clone(),
            txpool: txpool_read_handle.clone(),
            clearnet: clearnet_interface.clone(),
            tor: if tor_enabled { Some(tor_rx) } else { None },
            sync: sync_state.clone(),
            command: command_handler,
        };

        // Initialize the blockchain manager.
        blockchain::init_blockchain_manager(
            clearnet_interface,
            blockchain_write_handle,
            blockchain_read_handle.clone(),
            tx_handler.txpool_manager.clone(),
            context_svc.clone(),
            config.block_downloader_config(),
            syncer_handle,
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
        );

        // Start Tor P2P zone after sync completes.
        let sync_state_clone = sync_state.clone();
        if tor_enabled {
            info!("Tor P2P zone will start after sync.");
            let context_svc = context_svc.clone();

            tokio::spawn(async move {
                // Wait for the node to synchronize with the network
                if sync_state_clone.wait_for_synced().await.is_err() {
                    tracing::info!("Not starting Tor P2P zone, syncer stopped");
                    return;
                }
                tracing::info!("Starting Tor P2P zone.");

                let (tor_interface, tor_tx_handler_tx) = p2p::start_tor_p2p(
                    &config,
                    context_svc,
                    blockchain_read_handle,
                    txpool_read_handle,
                    tor_context,
                )
                .await;

                // Publish the Tor interface for consumers
                drop(tor_tx.send(tor_interface.clone()));

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

        node
    }
}
