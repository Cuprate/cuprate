use crate::constants::PANIC_CRITICAL_SERVICE_ERROR;
use crate::p2p::{CrossNetworkInternalPeerId, NetworkInterfaces};
use crate::txpool::dandelion::DiffuseService;
use crate::txpool::incoming_tx::{DandelionTx, TxId};
use bytes::Bytes;
use cuprate_dandelion_tower::pool::{DandelionPoolService, IncomingTx, IncomingTxBuilder};
use cuprate_dandelion_tower::traits::DiffuseRequest;
use cuprate_dandelion_tower::TxState;
use cuprate_helper::time::current_unix_timestamp;
use cuprate_txpool::service::interface::{
    TxpoolReadRequest, TxpoolReadResponse, TxpoolWriteRequest, TxpoolWriteResponse,
};
use cuprate_txpool::service::{TxpoolReadHandle, TxpoolWriteHandle};
use cuprate_types::TransactionVerificationData;
use futures::StreamExt;
use indexmap::IndexMap;
use rand::Rng;
use std::cmp::min;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, oneshot};
use tokio_util::{time::delay_queue, time::DelayQueue};
use tower::{Service, ServiceExt};

const TX_RERELAY_TIME: u64 = 300;

enum TxPoolRequest {
    IncomingTx,
}

struct TxInfo {
    weight: usize,
    fee: u64,
    received_at: u64,
    private: bool,

    timeout_key: Option<delay_queue::Key>,
}

pub struct TxpoolConfig {
    maximum_age: u64,
}

impl Default for TxpoolConfig {
    fn default() -> Self {
        Self {
            maximum_age: 60 * 60 * 24,
        }
    }
}

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
                    weight: tx.weight.try_into().unwrap(),
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
    pub async fn new_block(&mut self, spent_key_images: Vec<[u8; 32]>) -> anyhow::Result<()> {
        let (tx, rx) = oneshot::channel();

        drop(self.spent_kis_tx.send((spent_key_images, tx)).await);

        rx.await
            .map_err(|_| anyhow::anyhow!("txpool manager stopped"))
    }
}

struct TxpoolManager {
    current_txs: IndexMap<[u8; 32], TxInfo>,

    tx_timeouts: DelayQueue<[u8; 32]>,

    txpool_write_handle: TxpoolWriteHandle,
    txpool_read_handle: TxpoolReadHandle,

    /// The dandelion txpool manager.
    dandelion_pool_manager: DandelionPoolService<DandelionTx, TxId, CrossNetworkInternalPeerId>,
    promote_tx_channel: mpsc::Receiver<[u8; 32]>,

    diffuse_service: DiffuseService,

    config: TxpoolConfig,
}

impl TxpoolManager {
    async fn remove_tx_from_pool(&mut self, tx: [u8; 32], remove_from_db: bool) {
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

    async fn rerelay_tx(&mut self, tx: [u8; 32]) {
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

    async fn handle_tx_timeout(&mut self, tx: [u8; 32]) {
        let Some(tx_info) = self.current_txs.get(&tx) else {
            return;
        };

        let time_in_pool = current_unix_timestamp() - tx_info.received_at;

        if time_in_pool > self.config.maximum_age {
            self.remove_tx_from_pool(tx, true).await;
            return;
        }

        let received_at = tx_info.received_at;

        self.rerelay_tx(tx).await;

        let tx_info = self.current_txs.get_mut(&tx).unwrap();

        let next_timeout = calculate_next_timeout(received_at, self.config.maximum_age);
        tx_info.timeout_key = Some(
            self.tx_timeouts
                .insert(tx, Duration::from_secs(next_timeout)),
        );
    }

    fn track_tx(&mut self, tx: [u8; 32], weight: usize, fee: u64, private: bool) {
        let now = current_unix_timestamp();

        let timeout_key = if private {
            // The dandelion pool handles stem tx embargo.
            None
        } else {
            let timeout = calculate_next_timeout(now, self.config.maximum_age);
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

    async fn handle_incoming_tx(
        &mut self,
        tx: TransactionVerificationData,
        state: TxState<CrossNetworkInternalPeerId>,
    ) {
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
            .expect("TODO")
        else {
            unreachable!()
        };

        if let Some(tx_hash) = double_spend {
            return;
        };

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

    async fn promote_tx(&mut self, tx: [u8; 32]) {
        let Some(tx_info) = self.current_txs.get_mut(&tx) else {
            return;
        };
        // It's now in the public pool, pretend we just saw it.
        tx_info.received_at = current_unix_timestamp();

        let next_timeout = calculate_next_timeout(tx_info.received_at, self.config.maximum_age);
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

    async fn new_block(&mut self, spent_key_images: Vec<[u8; 32]>) {
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

    let time_till_max_timeout = max_time_in_pool - time_in_pool;

    let timeouts = time_in_pool / TX_RERELAY_TIME;

    let time_out = min((timeouts + 1) * TX_RERELAY_TIME, time_till_max_timeout);

    time_out
}
