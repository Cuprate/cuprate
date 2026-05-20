use std::{
    cmp::min,
    collections::BTreeSet,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use bytes::Bytes;
use futures::StreamExt;
use indexmap::IndexMap;
use rand::Rng;
use tokio::sync::{mpsc, oneshot};
use tokio_util::{sync::CancellationToken, time::delay_queue, time::DelayQueue};
use tower::{Service, ServiceExt};
use tracing::{instrument, Instrument, Span};

use cuprate_dandelion_tower::{
    pool::{DandelionPoolService, IncomingTx, IncomingTxBuilder},
    traits::DiffuseRequest,
    TxState,
};
use cuprate_helper::time::current_unix_timestamp;
use cuprate_p2p_core::ClearNet;
use cuprate_txpool::{
    service::{
        interface::{
            TxpoolReadRequest, TxpoolReadResponse, TxpoolWriteRequest, TxpoolWriteResponse,
        },
        TxpoolReadHandle, TxpoolWriteHandle,
    },
    TxPoolError,
};
use cuprate_types::TransactionVerificationData;

use crate::{
    config::TxpoolConfig,
    monitor::TaskExecutor,
    p2p::{CrossNetworkInternalPeerId, NetworkInterfaces},
    txpool::{
        dandelion::DiffuseService,
        incoming_tx::{DandelionTx, TxId},
    },
};

const INCOMING_TX_QUEUE_SIZE: usize = 100;

/// The maximum number of recently-removed public transactions to remember.
///
/// When this limit is reached, the oldest entry (by removal timestamp) is dropped.
const MAX_RECENTLY_REMOVED_TXS: usize = 5000;

/// Starts the transaction pool manager service.
///
/// # Errors
///
/// This function will return an [`Err`] if any inner service has an unrecoverable error.
pub async fn start_txpool_manager(
    mut txpool_write_handle: TxpoolWriteHandle,
    mut txpool_read_handle: TxpoolReadHandle,
    promote_tx_channel: mpsc::UnboundedReceiver<[u8; 32]>,
    diffuse_service: DiffuseService<ClearNet>,
    dandelion_pool_manager: DandelionPoolService<DandelionTx, TxId, CrossNetworkInternalPeerId>,
    config: TxpoolConfig,
    task_executor: TaskExecutor,
) -> anyhow::Result<TxpoolManagerHandle> {
    let TxpoolReadResponse::Backlog(backlog) = txpool_read_handle
        .ready()
        .await?
        .call(TxpoolReadRequest::Backlog)
        .await?
    else {
        unreachable!()
    };

    tracing::info!(txs_in_pool = backlog.len(), "starting txpool manager");

    let mut stem_txs = Vec::new();

    let mut tx_timeouts = DelayQueue::with_capacity(backlog.len());
    let current_txs: IndexMap<[u8; 32], TxInfo> = backlog
        .into_iter()
        .map(|tx| {
            let timeout_key = if tx.private {
                stem_txs.push(tx.id);
                None
            } else {
                let next_timeout = calculate_next_timeout(tx.received_at, config.maximum_age_secs);
                Some(tx_timeouts.insert(tx.id, Duration::from_secs(next_timeout)))
            };

            (
                tx.id,
                TxInfo {
                    weight: tx.weight,
                    fee: tx.fee,
                    received_at: tx.received_at,
                    private: tx.private,
                    timeout_key,
                },
            )
        })
        .collect();

    let public_pool_timestamps: BTreeSet<(u64, [u8; 32])> = current_txs
        .iter()
        .filter(|(_, info)| !info.private)
        .map(|(id, info)| (info.received_at, *id))
        .collect();

    let mut manager = TxpoolManager {
        current_txs,
        tx_timeouts,
        public_pool_timestamps,
        recently_removed_txs: BTreeSet::new(),
        removed_txs_start_time: current_unix_timestamp(),
        txpool_write_handle,
        txpool_read_handle,
        dandelion_pool_manager,
        promote_tx_channel,
        diffuse_service,
        config,
    };

    tracing::info!(stem_txs = stem_txs.len(), "promoting stem txs");

    for tx in stem_txs {
        manager.promote_tx(tx).await?;
    }

    let (command_tx, command_rx) = mpsc::channel(INCOMING_TX_QUEUE_SIZE);
    let (spent_kis_tx, spent_kis_rx) = mpsc::channel(1);

    let shutdown_token = task_executor.cancellation_token();
    task_executor.spawn_critical(
        "txpool manager",
        manager.run(command_rx, spent_kis_rx, shutdown_token),
    );

    Ok(TxpoolManagerHandle {
        command_tx,
        spent_kis_tx,
    })
}

/// Commands sent to the [`TxpoolManager`] via [`TxpoolManagerHandle`].
#[expect(
    clippy::large_enum_variant,
    reason = "`IncomingTx` is the most common command"
)]
pub enum TxpoolManagerCommand {
    /// An incoming transaction to add to the pool.
    IncomingTx(
        TransactionVerificationData,
        TxState<CrossNetworkInternalPeerId>,
    ),

    /// Request hashes of public-pool transactions added/removed at or after a UNIX timestamp.
    PoolInfoSince {
        since: u64,
        response_tx: oneshot::Sender<PoolInfoSinceResponse>,
    },
}

/// Response to [`TxpoolManagerCommand::PoolInfoSince`].
pub struct PoolInfoSinceResponse {
    /// `true` if the manager's incremental tracking does not reach back to the
    /// requested timestamp, so the caller must send a full pool snapshot
    ///
    /// When set, `added` contains the entire public pool and `removed` is empty.
    pub full_required: bool,
    /// Hashes of public-pool txs that entered the pool at or after `since`.
    pub added: Vec<[u8; 32]>,
    /// Hashes of public-pool txs that were removed from the pool at or after `since`.
    pub removed: Vec<[u8; 32]>,
}

/// A handle to the tx-pool manager.
#[derive(Clone)]
pub struct TxpoolManagerHandle {
    /// Channel for sending commands to the manager.
    pub command_tx: mpsc::Sender<TxpoolManagerCommand>,

    /// The spent key images in a new block tx.
    spent_kis_tx: mpsc::Sender<(Vec<[u8; 32]>, oneshot::Sender<()>)>,
}

impl TxpoolManagerHandle {
    /// Create a mock [`TxpoolManagerHandle`] that does nothing.
    ///
    /// Useful for testing.
    #[expect(clippy::let_underscore_must_use)]
    pub fn mock() -> Self {
        let (spent_kis_tx, mut spent_kis_rx) = mpsc::channel(1);
        let (command_tx, mut command_rx) = mpsc::channel(100);

        tokio::spawn(async move {
            loop {
                let Some(rec): Option<(_, oneshot::Sender<()>)> = spent_kis_rx.recv().await else {
                    return;
                };

                let _ = rec.1.send(());
            }
        });

        tokio::spawn(async move {
            loop {
                if command_rx.recv().await.is_none() {
                    return;
                }
            }
        });

        Self {
            command_tx,
            spent_kis_tx,
        }
    }

    /// Tell the tx-pool about spent key images in an incoming block.
    pub async fn new_block(
        &mut self,
        spent_key_images: Vec<[u8; 32]>,
    ) -> Result<(), tower::BoxError> {
        let (tx, rx) = oneshot::channel();

        drop(self.spent_kis_tx.send((spent_key_images, tx)).await);

        rx.await.map_err(|_| "txpool manager stopped".into())
    }
}

/// Information on a transaction in the tx-pool.
struct TxInfo {
    /// The weight of the transaction.
    weight: usize,
    /// The fee the transaction paid.
    fee: u64,
    /// The UNIX timestamp when the tx was received.
    received_at: u64,
    /// Whether the tx is in the private pool.
    private: bool,

    /// The [`delay_queue::Key`] for the timeout queue in the manager.
    ///
    /// This will be [`None`] if the tx is private as timeouts for them are handled in the dandelion pool.
    timeout_key: Option<delay_queue::Key>,
}

struct TxpoolManager {
    current_txs: IndexMap<[u8; 32], TxInfo>,

    /// A [`DelayQueue`] for waiting on tx timeouts.
    ///
    /// Timeouts can be for re-relaying or removal from the pool.
    tx_timeouts: DelayQueue<[u8; 32]>,

    /// Sorted `(public_timestamp, tx_hash)` for every tx currently in the public pool.
    ///
    /// Stem txs are never present here.
    public_pool_timestamps: BTreeSet<(u64, [u8; 32])>,

    /// Sorted `(removal_timestamp, tx_hash)` for recently removed public-pool transactions.
    ///
    /// Bounded by [`MAX_RECENTLY_REMOVED_TXS`], when the limit is exceeded the entry with the
    /// lowest removal timestamp is dropped. Only public (non-stem) txs are tracked here.
    recently_removed_txs: BTreeSet<(u64, [u8; 32])>,

    /// The earliest timestamp incremental removed-tx tracking covers. Advanced when old entries are
    /// evicted from `recently_removed_txs`.
    removed_txs_start_time: u64,

    txpool_write_handle: TxpoolWriteHandle,
    txpool_read_handle: TxpoolReadHandle,

    dandelion_pool_manager: DandelionPoolService<DandelionTx, TxId, CrossNetworkInternalPeerId>,
    /// The channel the dandelion manager will use to communicate that a tx should be promoted to the
    /// public pool.
    promote_tx_channel: mpsc::UnboundedReceiver<[u8; 32]>,
    /// The [`DiffuseService`] to diffuse txs to the p2p network.
    ///
    /// Used for re-relays.
    diffuse_service: DiffuseService<ClearNet>,

    config: TxpoolConfig,
}

impl TxpoolManager {
    /// Removes a transaction from the tx-pool manager, and optionally the database too.
    ///
    /// # Panics
    ///
    /// This function will panic if the tx is not in the tx-pool manager.
    #[instrument(level = "debug", skip_all, fields(tx_id = hex::encode(tx)))]
    async fn remove_tx_from_pool(
        &mut self,
        tx: [u8; 32],
        remove_from_db: bool,
    ) -> Result<(), TxPoolError> {
        tracing::debug!("removing tx from pool");

        if remove_from_db {
            self.txpool_write_handle
                .ready()
                .await?
                .call(TxpoolWriteRequest::RemoveTransaction(tx))
                .await?;
        }

        let tx_info = self.current_txs.swap_remove(&tx).unwrap();

        tx_info
            .timeout_key
            .and_then(|key| self.tx_timeouts.try_remove(&key));

        if !tx_info.private {
            self.public_pool_timestamps
                .remove(&(tx_info.received_at, tx));

            let removal_timestamp = current_unix_timestamp();
            self.recently_removed_txs.insert((removal_timestamp, tx));
            if self.recently_removed_txs.len() > MAX_RECENTLY_REMOVED_TXS {
                if let Some((evicted_timestamp, _)) = self.recently_removed_txs.pop_first() {
                    self.removed_txs_start_time =
                        self.removed_txs_start_time.max(evicted_timestamp);
                }
            }
        }

        Ok(())
    }

    /// Re-relay a tx to the network.
    ///
    /// # Panics
    ///
    /// This function will panic if the tx is not in the tx-pool.
    #[instrument(level = "debug", skip_all, fields(tx_id = hex::encode(tx)))]
    async fn rerelay_tx(&mut self, tx: [u8; 32]) -> Result<(), TxPoolError> {
        tracing::debug!("re-relaying tx to network");

        let TxpoolReadResponse::TxBlob { tx_blob, .. } = self
            .txpool_read_handle
            .ready()
            .await?
            .call(TxpoolReadRequest::TxBlob(tx))
            .await?
        else {
            unreachable!()
        };

        self.diffuse_service
            .call(DiffuseRequest(DandelionTx(Bytes::from(tx_blob))))
            .await
            .expect("Diffuse service should not return an error");

        Ok(())
    }

    /// Handles a transaction timeout, be either rebroadcasting or dropping the tx from the pool.
    /// If a rebroadcast happens, this function will handle adding another timeout to the queue.
    #[instrument(level = "debug", skip_all, fields(tx_id = hex::encode(tx)))]
    async fn handle_tx_timeout(&mut self, tx: [u8; 32]) -> Result<(), TxPoolError> {
        let Some(tx_info) = self.current_txs.get(&tx) else {
            tracing::warn!("tx timed out, but tx not in pool");
            return Ok(());
        };

        let time_in_pool = current_unix_timestamp() - tx_info.received_at;

        // Check if the tx has timed out, with a small buffer to prevent rebroadcasting if the time is
        // slightly off.
        if time_in_pool + 10 > self.config.maximum_age_secs {
            tracing::warn!("tx has been in pool too long, removing from pool");
            self.remove_tx_from_pool(tx, true).await?;
            return Ok(());
        }

        let received_at = tx_info.received_at;

        tracing::debug!(time_in_pool, "tx timed out, resending to network");

        self.rerelay_tx(tx).await?;

        let tx_info = self.current_txs.get_mut(&tx).unwrap();

        let next_timeout = calculate_next_timeout(received_at, self.config.maximum_age_secs);
        tracing::trace!(in_secs = next_timeout, "setting next tx timeout");

        tx_info.timeout_key = Some(
            self.tx_timeouts
                .insert(tx, Duration::from_secs(next_timeout)),
        );

        Ok(())
    }

    /// Adds a tx to the tx-pool manager.
    #[instrument(level = "trace", skip_all, fields(tx_id = hex::encode(tx)))]
    fn track_tx(&mut self, tx: [u8; 32], weight: usize, fee: u64, private: bool) {
        let now = current_unix_timestamp();

        let timeout_key = if private {
            // The dandelion pool handles stem tx embargo.
            None
        } else {
            self.public_pool_timestamps.insert((now, tx));

            let timeout = calculate_next_timeout(now, self.config.maximum_age_secs);

            tracing::trace!(in_secs = timeout, "setting next tx timeout");

            Some(self.tx_timeouts.insert(tx, Duration::from_secs(timeout)))
        };

        self.current_txs.insert(
            tx,
            TxInfo {
                weight,
                fee,
                received_at: now,
                private,
                timeout_key,
            },
        );
    }

    /// Handles an incoming tx, adding it to the pool and routing it.
    #[instrument(level = "debug", skip_all, fields(tx_id = hex::encode(tx.tx_hash), state))]
    async fn handle_incoming_tx(
        &mut self,
        tx: TransactionVerificationData,
        state: TxState<CrossNetworkInternalPeerId>,
    ) -> Result<(), TxPoolError> {
        tracing::debug!("handling new tx");

        let incoming_tx =
            IncomingTxBuilder::new(DandelionTx(Bytes::copy_from_slice(&tx.tx_blob)), tx.tx_hash);

        let (tx_hash, tx_weight, tx_fee) = (tx.tx_hash, tx.tx_weight, tx.fee);

        let TxpoolWriteResponse::AddTransaction(double_spend) = self
            .txpool_write_handle
            .ready()
            .await?
            .call(TxpoolWriteRequest::AddTransaction {
                tx: Box::new(tx),
                state_stem: state.is_stem_stage(),
            })
            .await?
        else {
            unreachable!()
        };

        if let Some(tx_hash) = double_spend {
            tracing::debug!(
                double_spent = hex::encode(tx_hash),
                "transaction is a double spend, ignoring"
            );
            return Ok(());
        }

        self.track_tx(tx_hash, tx_weight, tx_fee, state.is_stem_stage());

        let incoming_tx = incoming_tx
            .with_routing_state(state)
            .with_state_in_db(None)
            .build()
            .unwrap();

        if let Err(e) = async {
            self.dandelion_pool_manager
                .ready()
                .await?
                .call(incoming_tx)
                .await
        }
        .await
        {
            tracing::warn!("Dandelion pool manager failed for incoming tx: {e}");
        }

        Ok(())
    }

    /// Promote a tx to the public pool.
    #[instrument(level = "debug", skip_all, fields(tx_id = hex::encode(tx)))]
    async fn promote_tx(&mut self, tx: [u8; 32]) -> Result<(), TxPoolError> {
        let Some(tx_info) = self.current_txs.get_mut(&tx) else {
            tracing::debug!("not promoting tx, tx not in pool");
            return Ok(());
        };

        if !tx_info.private {
            tracing::trace!("not promoting tx, tx is already public");
            return Ok(());
        }

        tracing::debug!("promoting tx");

        self.txpool_write_handle
            .ready()
            .await?
            .call(TxpoolWriteRequest::Promote(tx))
            .await?;

        tx_info.private = false;
        // It's now in the public pool, pretend we just saw it.
        tx_info.received_at = current_unix_timestamp();
        self.public_pool_timestamps
            .insert((tx_info.received_at, tx));

        let next_timeout =
            calculate_next_timeout(tx_info.received_at, self.config.maximum_age_secs);
        tracing::trace!(in_secs = next_timeout, "setting next tx timeout");
        tx_info.timeout_key = Some(
            self.tx_timeouts
                .insert(tx, Duration::from_secs(next_timeout)),
        );

        Ok(())
    }

    /// Returns the hashes of all public-pool transactions that entered the public pool at or
    /// after `timestamp`.
    fn public_txs_from(&self, timestamp: u64) -> Vec<[u8; 32]> {
        self.public_pool_timestamps
            .range((timestamp, [0_u8; 32])..)
            .map(|(_, hash)| *hash)
            .collect()
    }

    /// Returns the hashes of recently removed public-pool transactions that were removed at or
    /// after `timestamp` (inclusive, UNIX seconds).
    fn removed_txs_from(&self, timestamp: u64) -> Vec<[u8; 32]> {
        self.recently_removed_txs
            .range((timestamp, [0_u8; 32])..)
            .map(|(_, hash)| *hash)
            .collect()
    }

    /// Handles removing all transactions that have been included/double spent in an incoming block.
    #[instrument(level = "debug", skip_all)]
    async fn new_block(&mut self, spent_key_images: Vec<[u8; 32]>) -> Result<(), TxPoolError> {
        tracing::debug!("handling new block");

        let TxpoolWriteResponse::NewBlock(removed_txs) = self
            .txpool_write_handle
            .ready()
            .await?
            .call(TxpoolWriteRequest::NewBlock { spent_key_images })
            .await?
        else {
            unreachable!()
        };

        for tx in removed_txs {
            self.remove_tx_from_pool(tx, false).await?;
        }
        Ok(())
    }

    async fn run(
        mut self,
        mut command_rx: mpsc::Receiver<TxpoolManagerCommand>,
        mut block_rx: mpsc::Receiver<(Vec<[u8; 32]>, oneshot::Sender<()>)>,
        shutdown_token: CancellationToken,
    ) -> Result<(), TxPoolError> {
        loop {
            tokio::select! {
                biased;
                () = shutdown_token.cancelled() => {
                    break;
                }
                Some((spent_kis, tx)) = block_rx.recv() => {
                    self.new_block(spent_kis).await?;
                    let _ = tx.send(());
                }
                Some(tx) = self.tx_timeouts.next() => {
                    self.handle_tx_timeout(tx.into_inner()).await?;
                }
                Some(command) = command_rx.recv() => {
                    match command {
                        TxpoolManagerCommand::IncomingTx(tx, state) => {
                            self.handle_incoming_tx(tx, state).await?;
                        }
                        TxpoolManagerCommand::PoolInfoSince { since, response_tx } => {
                            // If `since` is 0 the requester wants the full pool. If `since` is older than `removed_txs_start_time`,
                            // then we need to send a full pool as the requester might have missed some removals.
                            let full_required =
                                since == 0 || since <= self.removed_txs_start_time;

                            let response = if full_required {
                                PoolInfoSinceResponse {
                                    full_required: true,
                                    added: self.public_txs_from(0),
                                    removed: vec![],
                                }
                            } else {
                                PoolInfoSinceResponse {
                                    full_required: false,
                                    added: self.public_txs_from(since),
                                    removed: self.removed_txs_from(since),
                                }
                            };
                            let _ = response_tx.send(response);
                        }
                    }
                }
                Some(tx) = self.promote_tx_channel.recv() => {
                    self.promote_tx(tx).await?;
                }
            }
        }

        tracing::info!("Txpool manager shut down.");
        Ok(())
    }
}

/// Calculates the amount of time to wait before resending a tx to the network.
fn calculate_next_timeout(received_at: u64, max_time_in_pool: u64) -> u64 {
    /// The base time between re-relays to the p2p network.
    const TX_RERELAY_TIME: u64 = 300;

    /*
    This is a simple exponential backoff.
    The first timeout is TX_RERELAY_TIME seconds, the second is 2 * TX_RERELAY_TIME seconds, then 4, 8, 16, etc.
     */
    let now = current_unix_timestamp();

    let time_in_pool = now - received_at;

    let time_till_max_timeout = max_time_in_pool.saturating_sub(time_in_pool);

    let timeouts = time_in_pool / TX_RERELAY_TIME;

    min((timeouts + 1) * TX_RERELAY_TIME, time_till_max_timeout)
}
