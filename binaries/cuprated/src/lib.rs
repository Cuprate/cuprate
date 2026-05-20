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
//! let config = Config::read_from_path("cuprated.toml")?;
//! cuprated::logging::init_logging(&config);
//!
//! let mut node = Node::launch(config).await;
//! let height = node.blockchain.context().chain_height;
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

use anyhow::Context;
use tokio::sync::{oneshot, RwLock};
use tower::{Service, ServiceExt};
use tracing::error;

use cuprate_p2p::NetworkInterface;
use cuprate_p2p_core::{ClearNet, Tor};
use cuprate_txpool::service::TxpoolReadHandle;
use cuprate_types::blockchain::BlockchainWriteRequest;

use crate::{
    blockchain::{BlockchainInterface, BlockchainManagerHandle, BlockchainSyncerHandle},
    config::Config,
    constants::DATABASE_CORRUPT_MSG,
    monitor::TaskExecutor,
    tor::initialize_tor_if_enabled,
    txpool::IncomingTxHandler,
};

/// Captures the necessary context for launching the node.
///
/// A field belongs here if it is `Clone + Send + Sync`, used by
/// multiple subsystems, and available before subsystem init begins.
/// Write handles, single-consumer channels, `!Sync` types,
/// and late-constructed services do _not_ belong here.
#[derive(Clone)]
pub(crate) struct LaunchContext {
    /// The configuration this node was launched with.
    pub config: Arc<Config>,

    /// Reorg lock.
    ///
    /// A [`RwLock`] where a write lock is taken during a reorg and a read lock can be taken
    /// for any operation which must complete without a reorg happening.
    ///
    /// Currently, the only operation that needs to take a read lock is adding txs to the tx-pool,
    /// this can potentially be removed in the future, see: <https://github.com/Cuprate/cuprate/issues/305>
    pub reorg_lock: Arc<RwLock<()>>,

    /// Interface to the blockchain (database reads, cached state, mutations).
    pub blockchain: BlockchainInterface,

    /// Read handle to the transaction pool.
    pub txpool_read: TxpoolReadHandle,

    /// Task spawning and shutdown coordination.
    pub task_executor: TaskExecutor,
}

/// An active `cuprated` node.
///
/// Returned by [`Node::launch`]. Use this to interact with the running node.
#[must_use]
pub struct Node {
    /// Interface to the blockchain.
    pub blockchain: BlockchainInterface,

    /// Transaction pool queries.
    pub txpool: TxpoolReadHandle,

    /// Clearnet P2P interface.
    pub clearnet: NetworkInterface<ClearNet>,

    /// Tor P2P interface (available after sync).
    pub tor: Option<oneshot::Receiver<NetworkInterface<Tor>>>,

    /// The configuration this node was launched with.
    pub config: Arc<Config>,

    /// Task spawning and shutdown executor.
    pub task_executor: TaskExecutor,
}

impl Drop for Node {
    fn drop(&mut self) {
        self.shutdown();
    }
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
    /// - Memory resolution (call [`resolve_max_memory`](crate::config::resolve_max_memory))
    ///
    /// # Errors
    ///
    /// Returns an error if the database is corrupt, critical services fail to start,
    /// or `target_max_memory` is unresolved.
    pub async fn launch(config: impl Into<Arc<Config>>) -> Result<Self, anyhow::Error> {
        let config: Arc<Config> = config.into();
        let task_executor = TaskExecutor::new();
        let shutdown_token = task_executor.cancellation_token();

        let node: Result<Self, anyhow::Error> = async move {
            // Initialize the database thread pool.
            let db_thread_pool = Arc::new(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(config.storage.reader_threads)
                    .build()
                    .context("failed to build rayon database thread pool")?,
            );

            // Start the blockchain & tx-pool databases.
            let fjall_db = fjall::Database::builder(config.fjall_directory())
                .cache_size(config.fjall_cache_size())
                .open()
                .context(DATABASE_CORRUPT_MSG)?;

            let (mut blockchain_read_handle, mut blockchain_write_handle, _) =
                cuprate_blockchain::service::init_with_pool(
                    &config.blockchain_config(),
                    fjall_db.clone(),
                    Arc::clone(&db_thread_pool),
                )
                .context(DATABASE_CORRUPT_MSG)?;

            let (txpool_read_handle, txpool_write_handle) =
                cuprate_txpool::service::init_with_pool(fjall_db, db_thread_pool)
                    .context(DATABASE_CORRUPT_MSG)?;

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
            .await?;

            // Start the context service and the block/tx verifier.
            let context_svc =
                blockchain::init_consensus(blockchain_read_handle.clone(), config.context_config())
                    .await
                    .map_err(anyhow::Error::from_boxed)?;

            // Create the blockchain syncer handle and synced signal sender.
            let (blockchain_syncer_handle, synced_tx) = BlockchainSyncerHandle::new();

            // Create the blockchain manager handle and command receiver.
            let (blockchain_manager_handle, command_rx) = BlockchainManagerHandle::new();

            // Create the blockchain interface.
            let blockchain_interface = BlockchainInterface::new(
                blockchain_read_handle,
                context_svc,
                blockchain_manager_handle,
                blockchain_syncer_handle,
            );

            // Create the launch context.
            let launch_ctx = LaunchContext {
                config,
                reorg_lock: Arc::new(RwLock::new(())),
                blockchain: blockchain_interface,
                txpool_read: txpool_read_handle,
                task_executor,
            };

            // Bootstrap or configure Tor if enabled.
            let tor_enabled = launch_ctx.config.p2p.tor_net.enabled;
            let tor_context = initialize_tor_if_enabled(&launch_ctx).await;

            // Start clearnet P2P zone
            let (clearnet_interface, clearnet_tx_handler_subscriber) =
                p2p::initialize_clearnet_p2p(&launch_ctx, &tor_context).await?;

            // Create Tor router delivery channel.
            let (tor_router_tx, tor_router_rx) = tor_enabled.then(oneshot::channel).unzip();

            // Create the incoming tx handler service.
            let tx_handler = IncomingTxHandler::init(
                &launch_ctx,
                clearnet_interface.clone(),
                tor_router_rx,
                txpool_write_handle.clone(),
            )
            .await?;

            // Send tx handler sender to clearnet zone
            if clearnet_tx_handler_subscriber
                .send(tx_handler.clone())
                .is_err()
            {
                unreachable!()
            }

            // Tor interface channel - populated when Tor starts after sync.
            let (tor_tx, tor_rx) = oneshot::channel();

            // Initialize the blockchain manager.
            blockchain::init_blockchain_manager(
                &launch_ctx,
                clearnet_interface.clone(),
                blockchain_write_handle,
                tx_handler.txpool_manager.clone(),
                synced_tx,
                command_rx,
            )
            .await?;

            // Initialize the RPC server(s).
            rpc::init_rpc_servers(&launch_ctx, tx_handler.clone())?;

            // Start Tor P2P zone after sync completes.
            if tor_enabled {
                p2p::initialize_tor_p2p(
                    launch_ctx.clone(),
                    tor_context,
                    tx_handler,
                    tor_tx,
                    tor_router_tx,
                );
            }

            let LaunchContext {
                blockchain,
                txpool_read,
                config,
                task_executor,
                ..
            } = launch_ctx;

            Ok(Self {
                blockchain,
                txpool: txpool_read,
                clearnet: clearnet_interface,
                tor: if tor_enabled { Some(tor_rx) } else { None },
                config,
                task_executor,
            })
        }
        .await;

        if node.is_err() {
            shutdown_token.cancel();
        }
        node
    }

    /// Trigger a graceful shutdown.
    pub fn shutdown(&self) {
        self.task_executor.trigger_shutdown();
    }

    /// Wait for shutdown to be triggered, then await all tracked tasks.
    pub async fn wait_for_shutdown(&self) {
        self.task_executor.wait_for_shutdown().await;
    }
}
