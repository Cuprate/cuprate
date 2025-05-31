use std::{
    cmp::min,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use bytes::Bytes;
use futures::StreamExt;
use indexmap::IndexMap;
use rand::Rng;
use tokio::sync::{mpsc, oneshot};
use tokio_util::{time::delay_queue, time::DelayQueue};
use tower::{Service, ServiceExt};
use tracing::{instrument, Instrument, Span};

use cuprate_dandelion_tower::{
    pool::{DandelionPoolService, IncomingTx, IncomingTxBuilder},
    traits::DiffuseRequest,
    TxState,
};
use cuprate_helper::time::current_unix_timestamp;
use cuprate_txpool::service::{
    interface::{TxpoolReadRequest, TxpoolReadResponse, TxpoolWriteRequest, TxpoolWriteResponse},
    TxpoolReadHandle, TxpoolWriteHandle,
};
use cuprate_types::TransactionVerificationData;

use crate::config::TxpoolConfig;
use crate::{
    constants::PANIC_CRITICAL_SERVICE_ERROR,
    p2p::{CrossNetworkInternalPeerId, NetworkInterfaces},
    txpool::{
        dandelion::DiffuseService,
        incoming_tx::{DandelionTx, TxId},
    },
};

/// The base time between re-relays to the p2p network.
const TX_RERELAY_TIME: u64 = 300;
/// Starts the transaction pool manager service.
///
/// # Panics
///
/// This function may panic if any inner service has an unrecoverable error.
pub async fn start_txpool_manager(
    mut txpool_write_handle: TxpoolWriteHandle,
    mut txpool_read_handle: TxpoolReadHandle,
    promote_tx_channel: mpsc::Receiver<[u8; 32]>,
    diffuse_service: DiffuseService,
    dandelion_pool_manager: DandelionPoolService<DandelionTx, TxId, CrossNetworkInternalPeerId>,
    config: TxpoolConfig,
) -> TxpoolManagerHandle {
    let TxpoolReadResponse::Backlog(backlog) = txpool_read_handle
        .ready()
        .await
        .expect(PANIC_CRITICAL_SERVICE_ERROR)
        .call(TxpoolReadRequest::Backlog)
        .await
        .expect(PANIC_CRITICAL_SERVICE_ERROR)
    else {
        unreachable!()
    };

    tracing::info!(txs_in_pool = backlog.len(), "starting txpool manager");

    let mut stem_txs = Vec::new();

    let mut tx_timeouts = DelayQueue::with_capacity(backlog.len());
    let current_txs = backlog
        .into_iter()
        .map(|tx| {
            let timeout_key = if tx.private {
                stem_txs.push(tx.id);
                None
            } else {
                let next_timeout = calculate_next_timeout(tx.received_at, config.maximum_age);
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

    let mut manager = TxpoolManager {
        current_txs,
        tx_timeouts,
        txpool_write_handle,
        txpool_read_handle,
        dandelion_pool_manager,
        promote_tx_channel,
        diffuse_service,
        config,
    };

    tracing::info!(stem_txs = stem_txs.len(), "promoting stem txs");

    for tx in stem_txs {
        manager.promote_tx(tx).await;
    }

    let (tx_tx, tx_rx) = mpsc::channel(100);
    let (spent_kis_tx, spent_kis_rx) = mpsc::channel(1);

    tokio::spawn(manager.run(tx_rx, spent_kis_rx));

    TxpoolManagerHandle {
        tx_tx,
        spent_kis_tx,
    }
}

#[derive(Clone)]
pub struct TxpoolManagerHandle {
    pub tx_tx: mpsc::Sender<(
        TransactionVerificationData,
        TxState<CrossNetworkInternalPeerId>,
    )>,

    spent_kis_tx: mpsc::Sender<(Vec<[u8; 32]>, oneshot::Sender<()>)>,
}

impl TxpoolManagerHandle {
    /// Create a mock [`TxpoolManagerHandle`] that does nothing.
    ///
    /// Useful for testing.
    #[expect(clippy::let_underscore_must_use)]
    pub fn mock() -> Self {
        let (spent_kis_tx, mut spent_kis_rx) = mpsc::channel(1);
        let (tx_tx, mut tx_rx) = mpsc::channel(100);

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
                if tx_rx.recv().await.is_none() {
                    return;
                }
            }
        });

        Self {
            tx_tx,
            spent_kis_tx,
        }
    }

    /// Tell the tx-pool about spent key images in an incoming block.
    pub async fn new_block(&mut self, spent_key_images: Vec<[u8; 32]>) -> anyhow::Result<()> {
        let (tx, rx) = oneshot::channel();

        drop(self.spent_kis_tx.send((spent_key_images, tx)).await);

        rx.await
            .map_err(|_| anyhow::anyhow!("txpool manager stopped"))
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

    txpool_write_handle: TxpoolWriteHandle,
    txpool_read_handle: TxpoolReadHandle,

    dandelion_pool_manager: DandelionPoolService<DandelionTx, TxId, CrossNetworkInternalPeerId>,
    /// The channel the dandelion manager will use to communicate that a tx should be promoted to the
    /// public pool.
    promote_tx_channel: mpsc::Receiver<[u8; 32]>,
    /// The [`DiffuseService`] to diffuse txs to the p2p network.
    ///
    /// Used for re-relays.
    diffuse_service: DiffuseService,

    config: TxpoolConfig,
}

impl TxpoolManager {
    /// Removes a transaction from the tx-pool manager, and optionally the database too.
    ///
    /// # Panics
    ///
    /// This function will panic if the tx is not in the tx-pool manager.
    #[instrument(level = "debug", skip_all, fields(tx_id = hex::encode(tx)))]
    async fn remove_tx_from_pool(&mut self, tx: [u8; 32], remove_from_db: bool) {
        tracing::debug!("removing tx from pool");

        let tx_info = self.current_txs.swap_remove(&tx).unwrap();

        tx_info
            .timeout_key
            .and_then(|key| self.tx_timeouts.try_remove(&key));

        if remove_from_db {
            self.txpool_write_handle
                .ready()
                .await
                .expect(PANIC_CRITICAL_SERVICE_ERROR)
                .call(TxpoolWriteRequest::RemoveTransaction(tx))
                .await
                .expect(PANIC_CRITICAL_SERVICE_ERROR);
        }
    }

    /// Re-relay a tx to the network.
    ///
    /// # Panics
    ///
    /// This function will panic if the tx is not in the tx-pool.
    #[instrument(level = "debug", skip_all, fields(tx_id = hex::encode(tx)))]
    async fn rerelay_tx(&mut self, tx: [u8; 32]) {
        tracing::debug!("re-relaying tx to network");

        let TxpoolReadResponse::TxBlob {
            tx_blob,
            state_stem: _,
        } = self
            .txpool_read_handle
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(TxpoolReadRequest::TxBlob(tx))
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
        else {
            unreachable!()
        };

        self.diffuse_service
            .call(DiffuseRequest(DandelionTx(Bytes::from(tx_blob))))
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR);
    }

    /// Handles a transaction timeout, be either rebroadcasting or dropping the tx from the pool.
    /// If a rebroadcast happens, this function will handle adding another timeout to the queue.
    #[instrument(level = "debug", skip_all, fields(tx_id = hex::encode(tx)))]
    async fn handle_tx_timeout(&mut self, tx: [u8; 32]) {
        let Some(tx_info) = self.current_txs.get(&tx) else {
            tracing::warn!("tx timed out, but tx not in pool");
            return;
        };

        let time_in_pool = current_unix_timestamp() - tx_info.received_at;

        if time_in_pool > self.config.maximum_age {
            tracing::warn!("tx has been in pool too long, removing from pool");
            self.remove_tx_from_pool(tx, true).await;
            return;
        }

        let received_at = tx_info.received_at;

        tracing::debug!(time_in_pool, "tx timed out, resending to network");

        self.rerelay_tx(tx).await;

        let tx_info = self.current_txs.get_mut(&tx).unwrap();

        let next_timeout = calculate_next_timeout(received_at, self.config.maximum_age);
        tracing::trace!(in_secs = next_timeout, "setting next tx timeout");

        tx_info.timeout_key = Some(
            self.tx_timeouts
                .insert(tx, Duration::from_secs(next_timeout)),
        );
    }

    /// Adds a tx to the tx-pool manager.
    #[instrument(level = "trace", skip_all, fields(tx_id = hex::encode(tx)))]
    fn track_tx(&mut self, tx: [u8; 32], weight: usize, fee: u64, private: bool) {
        let now = current_unix_timestamp();

        let timeout_key = if private {
            // The dandelion pool handles stem tx embargo.
            None
        } else {
            let timeout = calculate_next_timeout(now, self.config.maximum_age);

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
    ) {
        tracing::debug!("handling new tx");

        let incoming_tx =
            IncomingTxBuilder::new(DandelionTx(Bytes::copy_from_slice(&tx.tx_blob)), tx.tx_hash);

        let (tx_hash, tx_weight, tx_fee) = (tx.tx_hash, tx.tx_weight, tx.fee);

        let TxpoolWriteResponse::AddTransaction(double_spend) = self
            .txpool_write_handle
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(TxpoolWriteRequest::AddTransaction {
                tx: Box::new(tx),
                state_stem: state.is_stem_stage(),
            })
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
        else {
            unreachable!()
        };

        if let Some(tx_hash) = double_spend {
            tracing::debug!(
                double_spent = hex::encode(tx_hash),
                "transaction is a double spend, ignoring"
            );
            return;
        }

        self.track_tx(tx_hash, tx_weight, tx_fee, state.is_stem_stage());

        let incoming_tx = incoming_tx
            .with_routing_state(state)
            .with_state_in_db(None)
            .build()
            .unwrap();

        self.dandelion_pool_manager
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(incoming_tx)
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR);
    }

    /// Promote a tx to the public pool.
    #[instrument(level = "debug", skip_all, fields(tx_id = hex::encode(tx)))]
    async fn promote_tx(&mut self, tx: [u8; 32]) {
        let Some(tx_info) = self.current_txs.get_mut(&tx) else {
            tracing::debug!("not promoting tx, tx not in pool");
            return;
        };

        if !tx_info.private {
            tracing::trace!("not promoting tx, tx is already public");
            return;
        }

        tracing::debug!("promoting tx");

        // It's now in the public pool, pretend we just saw it.
        tx_info.received_at = current_unix_timestamp();

        let next_timeout = calculate_next_timeout(tx_info.received_at, self.config.maximum_age);
        tracing::trace!(in_secs = next_timeout, "setting next tx timeout");
        tx_info.timeout_key = Some(
            self.tx_timeouts
                .insert(tx, Duration::from_secs(next_timeout)),
        );

        self.txpool_write_handle
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(TxpoolWriteRequest::Promote(tx))
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR);
    }

    /// Handles removing all transactions that have been included/double spent in an incoming block.
    #[instrument(level = "debug", skip_all)]
    async fn new_block(&mut self, spent_key_images: Vec<[u8; 32]>) {
        tracing::debug!("handling new block");

        let TxpoolWriteResponse::NewBlock(removed_txs) = self
            .txpool_write_handle
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(TxpoolWriteRequest::NewBlock { spent_key_images })
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
        else {
            unreachable!()
        };

        for tx in removed_txs {
            self.remove_tx_from_pool(tx, false).await;
        }
    }

    #[expect(clippy::let_underscore_must_use)]
    async fn run(
        mut self,
        mut tx_rx: mpsc::Receiver<(
            TransactionVerificationData,
            TxState<CrossNetworkInternalPeerId>,
        )>,
        mut block_rx: mpsc::Receiver<(Vec<[u8; 32]>, oneshot::Sender<()>)>,
    ) {
        loop {
            tokio::select! {
                Some(tx) = self.tx_timeouts.next() => {
                    self.handle_tx_timeout(tx.into_inner()).await;
                }
                Some((tx, state)) = tx_rx.recv() => {
                    self.handle_incoming_tx(tx, state).await;
                }
                Some(tx) = self.promote_tx_channel.recv() => {
                    self.promote_tx(tx).await;
                }
                Some((spent_kis, tx)) = block_rx.recv() => {
                    self.new_block(spent_kis).await;
                    let _ = tx.send(());
                }
            }
        }
    }
}

/// Calculates the amount of time to wait before resending a tx to the network.
fn calculate_next_timeout(received_at: u64, max_time_in_pool: u64) -> u64 {
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
