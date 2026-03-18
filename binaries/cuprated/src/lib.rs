//! `cuprated` library.
//!
//! Call [`Node::launch`] to initialize and run the node. Returns a [`Node`]
//! with handles to node services.
//!
//! # Example
//!
//! ```ignore
//! use cuprated::{config::Config, Node};
//!
//! let config = Config::read_from_path("cuprated.toml").unwrap();
//! cuprated::logging::init_logging(&config);
//!
//! let node = Node::launch(config).await;
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
pub mod monitor;
pub mod version;

mod p2p;
mod rpc;
mod tor;
mod txpool;

use std::sync::Arc;

use tokio::sync::{oneshot, RwLock};
use tower::{Service, ServiceExt};
use tracing::{error, info};

use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_consensus_context::BlockchainContextService;
use cuprate_p2p::NetworkInterface;
use cuprate_p2p_core::{ClearNet, Tor};
use cuprate_txpool::service::TxpoolReadHandle;
use cuprate_types::blockchain::BlockchainWriteRequest;

use cuprate_helper::network::Network;

use crate::{
    blockchain::{BlockchainManagerHandle, Syncer, SyncerHandle},
    commands::CommandHandle,
    config::Config,
    constants::PANIC_CRITICAL_SERVICE_ERROR,
    monitor::TaskExecutor,
    tor::initialize_tor_if_enabled,
    txpool::IncomingTxHandler,
};

/// Shared internal node state.
///
/// A field belongs here if it is `Clone + Send + Sync`, used by
/// multiple subsystems, and available before subsystem init begins.
/// Write handles, single-consumer channels, `!Sync` types,
/// and late-constructed services do _not_ belong here.
#[derive(Clone)]
pub struct NodeContext {
    /// Which Monero network this node is running on.
    pub network: Network,

    /// Per-network fast sync validation hashes.
    pub fast_sync_hashes: &'static [[u8; 32]],

    /// Reorg lock.
    ///
    /// A [`RwLock`] where a write lock is taken during a reorg and a read lock can be taken
    /// for any operation which must complete without a reorg happening.
    ///
    /// Currently, the only operation that needs to take a read lock is adding txs to the tx-pool,
    /// this can potentially be removed in the future, see: <https://github.com/Cuprate/cuprate/issues/305>
    pub reorg_lock: Arc<RwLock<()>>,

    /// Command channel to the blockchain manager.
    pub blockchain_manager: BlockchainManagerHandle,

    /// Read handle to the blockchain database.
    pub blockchain_read: BlockchainReadHandle,

    /// Blockchain context cache.
    pub blockchain_context: BlockchainContextService,

    /// Read handle to the transaction pool.
    pub txpool_read: TxpoolReadHandle,

    /// Syncer handle.
    pub syncer: SyncerHandle,

    /// The time this node was launched.
    pub start_instant: std::time::SystemTime,

    /// The time this node was launched as a UNIX timestamp.
    pub start_instant_unix: u64,

    /// Task spawning and shutdown coordination.
    pub task_executor: TaskExecutor,
}

/// An active `cuprated` node.
///
/// Returned by [`Node::launch`]. This is the embedder's handle to
/// the running node.
///
/// Fields here are the public API for callers. A field belongs here
/// if it is useful for an embedder to query or interact with after
/// launch. Internal wiring belongs in [`NodeContext`] instead.
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

    /// Syncer handle.
    pub syncer: SyncerHandle,

    /// Command channel.
    pub command: CommandHandle,

    /// Task spawning and shutdown executor.
    pub task_executor: TaskExecutor,
}

impl Node {
    /// Launch a new `cuprated` process.
    ///
    /// Sets up thread pools, databases, P2P networking, the blockchain manager,
    /// and RPC servers.
    ///
    /// The caller should set up the following before calling this:
    /// - Tracing/logging (the node emits tracing events during initialization)
    /// - Global rayon thread pool (optional, uses rayon defaults if not set)
    ///
    /// # Errors
    ///
    /// Returns an error if the database or critical service startup fails.
    pub async fn launch(config: Config) -> Result<Self, anyhow::Error> {
        let fast_sync_hashes = blockchain::get_fast_sync_hashes(config.fast_sync, config.network());

        // Initialize the database thread pool.
        let db_thread_pool = cuprate_database_service::init_thread_pool(
            cuprate_database_service::ReaderThreads::Number(config.storage.reader_threads),
        );

        // Start the blockchain & tx-pool databases.
        let (mut blockchain_read_handle, mut blockchain_write_handle, _) =
            cuprate_blockchain::service::init_with_pool(
                config.blockchain_config(),
                Arc::clone(&db_thread_pool),
            )
            .inspect_err(|e| error!("Blockchain database error: {e}"))?;

        let (txpool_read_handle, txpool_write_handle, _) =
            cuprate_txpool::service::init_with_pool(&config.txpool_config(), db_thread_pool)
                .inspect_err(|e| error!("Txpool database error: {e}"))?;

        // TODO: Add an argument/option for keeping alt blocks between restart.
        blockchain_write_handle
            .ready()
            .await?
            .call(BlockchainWriteRequest::FlushAltBlocks)
            .await?;

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
                .map_err(|e| anyhow::anyhow!(e))?;

        // Bootstrap or configure Tor if enabled.
        let tor_context = initialize_tor_if_enabled(&config).await;
        let tor_enabled = config.p2p.tor_net.enabled;

        // Create the syncer and handle.
        let (syncer, syncer_handle) = Syncer::new();

        // Create the blockchain manager handle and command receiver.
        let (blockchain_manager_handle, blockchain_manager_rx) = BlockchainManagerHandle::new();

        // Create the task executor for spawning and shutdown.
        let task_executor = TaskExecutor::new();

        // Create the node context.
        let start_instant = std::time::SystemTime::now();
        let node_ctx = NodeContext {
            network: config.network(),
            fast_sync_hashes,
            reorg_lock: Arc::new(RwLock::new(())),
            blockchain_manager: blockchain_manager_handle.clone(),
            blockchain_read: blockchain_read_handle.clone(),
            blockchain_context: context_svc.clone(),
            txpool_read: txpool_read_handle.clone(),
            syncer: syncer_handle,
            start_instant_unix: start_instant
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            start_instant,
            task_executor,
        };

        // Start clearnet P2P zone
        let (clearnet_interface, clearnet_tx_handler_subscriber) = p2p::initialize_clearnet_p2p(
            &config,
            context_svc.clone(),
            blockchain_read_handle.clone(),
            txpool_read_handle.clone(),
            &tor_context,
            node_ctx
                .syncer
                .callback(context_svc.clone(), blockchain_manager_handle.clone()),
            blockchain_manager_handle,
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
            node_ctx.clone(),
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

        // Command handle.
        let command_handle = CommandHandle::init(node_ctx.clone());

        // Create the node struct with cloned service handles for the caller.
        let node = Self {
            context: node_ctx.blockchain_context.clone(),
            blockchain: node_ctx.blockchain_read.clone(),
            txpool: node_ctx.txpool_read.clone(),
            clearnet: clearnet_interface.clone(),
            tor: if tor_enabled { Some(tor_rx) } else { None },
            syncer: node_ctx.syncer.clone(),
            command: command_handle,
            task_executor: node_ctx.task_executor.clone(),
        };

        // Initialize the blockchain manager.
        blockchain::init_blockchain_manager(
            clearnet_interface.clone(),
            blockchain_write_handle,
            tx_handler.txpool_manager.clone(),
            config.block_downloader_config(),
            syncer,
            blockchain_manager_rx,
            node_ctx.clone(),
        )
        .await;

        // Initialize the RPC server(s).
        rpc::init_rpc_servers(config.rpc.clone(), tx_handler.clone(), node_ctx.clone());

        // Start Tor P2P zone after sync completes.
        if tor_enabled {
            info!("Tor P2P zone will start after sync.");
            let task_executor = node_ctx.task_executor.clone();
            let shutdown_token = task_executor.cancellation_token().clone();

            task_executor.spawn(async move {
                // Wait for the node to synchronize with the network, or shutdown.
                tokio::select! {
                    result = node_ctx.syncer.wait_for_synced() => {
                        if result.is_err() {
                            tracing::info!("Not starting Tor P2P zone, syncer stopped");
                            return;
                        }
                    }
                    () = shutdown_token.cancelled() => {
                        return;
                    }
                }
                tracing::info!("Starting Tor P2P zone.");

                let (tor_interface, tor_tx_handler_tx) =
                    p2p::start_tor_p2p(&config, tor_context, node_ctx).await;

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

        Ok(node)
    }
}
