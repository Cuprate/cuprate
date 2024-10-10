use std::collections::HashSet;
use std::future::ready;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::blockchain::ConcreteTxVerifierService;
use crate::txpool::txs_being_handled::{tx_blob_hash, TxBeingHandledLocally, TxsBeingHandled};
use bytes::Bytes;
use cuprate_consensus::transactions::new_tx_verification_data;
use cuprate_consensus::{
    BlockChainContextRequest, BlockChainContextResponse, BlockChainContextService,
    ExtendedConsensusError, TxVerifierService, VerifyTxRequest, VerifyTxResponse,
};
use cuprate_dandelion_tower::pool::{DandelionPoolService, IncomingTx, IncomingTxBuilder};
use cuprate_dandelion_tower::TxState;
use cuprate_helper::asynch::rayon_spawn_async;
use cuprate_txpool::service::interface::{
    TxpoolReadRequest, TxpoolWriteRequest, TxpoolWriteResponse,
};
use cuprate_txpool::service::{TxpoolReadHandle, TxpoolWriteHandle};
use cuprate_wire::NetworkAddress;
use dashmap::DashSet;
use futures::future::BoxFuture;
use futures::FutureExt;
use monero_serai::transaction::Transaction;
use sha3::{Digest, Sha3_256};
use tower::{Service, ServiceExt};

pub enum IncomingTxError {
    Parse(std::io::Error),
    Consensus(ExtendedConsensusError),
    DuplicateTransaction,
}

pub enum IncomingTxs {
    Bytes {
        txs: Vec<Bytes>,
        state: TxState<NetworkAddress>,
    },
}

struct DandelionTx(Bytes);

type TxId = [u8; 32];

pub struct IncomingTxHandler {
    txs_being_added: Arc<TxsBeingHandled>,

    blockchain_context_cache: BlockChainContextService,

    dandelion_pool_manager: DandelionPoolService<DandelionTx, TxId, NetworkAddress>,
    tx_verifier_service: ConcreteTxVerifierService,

    txpool_write_handle: TxpoolWriteHandle,

    txpool_read_handle: TxpoolReadHandle,
}

impl Service<IncomingTxs> for IncomingTxHandler {
    type Response = ();
    type Error = IncomingTxError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: IncomingTxs) -> Self::Future {
        let IncomingTxs::Bytes { mut txs, state } = req;

        let mut local_tracker = self.txs_being_added.local_tracker();

        txs.retain(|bytes| local_tracker.try_add_tx(bytes.as_ref()));

        if txs.is_empty() {
            return ready(Ok(())).boxed();
        }

        let mut blockchain_context_cache = self.blockchain_context_cache.clone();
        let mut tx_verifier_service = self.tx_verifier_service.clone();
        let mut txpool_write_handle = self.txpool_write_handle.clone();

        async move {
            let txs = rayon_spawn_async(move || {
                txs.into_iter()
                    .map(|bytes| {
                        let tx = Transaction::read(&mut bytes.as_ref())
                            .map_err(IncomingTxError::Parse)?;

                        let tx = new_tx_verification_data(tx)
                            .map_err(|e| IncomingTxError::Consensus(e.into()))?;

                        Ok(Arc::new(tx))
                    })
                    .collect::<Result<Vec<_>, IncomingTxError>>()
            })
            .await?;

            let BlockChainContextResponse::Context(context) = blockchain_context_cache
                .ready()
                .await?
                .call(BlockChainContextRequest::GetContext)
                .await?
            else {
                unreachable!()
            };

            let context = context.unchecked_blockchain_context();

            tx_verifier_service
                .ready()
                .await?
                .call(VerifyTxRequest::Prepped {
                    txs: txs.clone(),
                    current_chain_height: context.chain_height,
                    top_hash: context.top_hash,
                    time_for_time_lock: context.current_adjusted_timestamp_for_time_lock(),
                    hf: context.current_hf,
                })
                .await?;

            txpool_write_handle
                .ready()
                .await?
                .call(TxpoolWriteRequest::AddTransaction {
                    tx,
                    state_stem: state.state_stem(),
                })
                .await;

            todo!()
        }
        .boxed()
    }
}

async fn handle_incoming_txs(
    txs: Vec<Bytes>,
    state: TxState<NetworkAddress>,
    tx_being_handled_locally: TxBeingHandledLocally,
    mut blockchain_context_cache: BlockChainContextService,
    mut tx_verifier_service: ConcreteTxVerifierService,
    mut txpool_write_handle: TxpoolWriteHandle,
    mut txpool_read_handle: TxpoolReadHandle,
    mut dandelion_pool_manager: DandelionPoolService<DandelionTx, TxId, NetworkAddress>,
) -> Result<(), IncomingTxError> {
    let mut tx_blob_hashes = HashSet::new();

    let txs = txs
        .into_iter()
        .map(|tx_blob| {
            let tx_blob_hash = tx_blob_hash(tx_blob.as_ref());
            if !tx_blob_hashes.insert(tx_blob_hash) {
                return Err(IncomingTxError::DuplicateTransaction);
            }

            Ok((tx_blob_hash, tx_blob))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let TxpoolReadRequest::FilterKnownTxBlobHashes(tx_blob_hashes) = txpool_read_handle
        .ready()
        .await?
        .call(TxpoolReadRequest::FilterKnownTxBlobHashes(tx_blob_hashes))
        .await?
    else {
        unreachable!()
    };

    let txs = rayon_spawn_async(move || {
        txs.into_iter()
            .filter_map(|(tx_blob_hash, tx_blob)| {
                if tx_blob_hashes.contains(&tx_blob_hash) {
                    Some(tx_blob)
                } else {
                    None
                }
            })
            .map(|bytes| {
                let tx = Transaction::read(&mut bytes.as_ref()).map_err(IncomingTxError::Parse)?;

                let tx = new_tx_verification_data(tx)
                    .map_err(|e| IncomingTxError::Consensus(e.into()))?;

                Ok(Arc::new(tx))
            })
            .collect::<Result<Vec<_>, IncomingTxError>>()
    })
    .await?;

    let BlockChainContextResponse::Context(context) = blockchain_context_cache
        .ready()
        .await?
        .call(BlockChainContextRequest::GetContext)
        .await?
    else {
        unreachable!()
    };

    let context = context.unchecked_blockchain_context();

    tx_verifier_service
        .ready()
        .await?
        .call(VerifyTxRequest::Prepped {
            txs: txs.clone(),
            current_chain_height: context.chain_height,
            top_hash: context.top_hash,
            time_for_time_lock: context.current_adjusted_timestamp_for_time_lock(),
            hf: context.current_hf,
        })
        .await?;

    for tx in txs {
        let incoming_tx = IncomingTxBuilder::new(Bytes::copy_from_slice(&tx.tx_blob), tx.tx_hash);

        let TxpoolWriteResponse::AddTransaction(double_spend) = txpool_write_handle
            .ready()
            .await?
            .call(TxpoolWriteRequest::AddTransaction {
                tx,
                state_stem: state.state_stem(),
            })
            .await?
        else {
            unreachable!()
        };

        // TODO: track double spends to quickly ignore them from their blob hash.
        if let Some(tx_hash) = double_spend {
            continue;
        };

        // TODO: check blockchain for double spends to prevent a race condition.

        // TODO: fill this in properly.
        let incoming_tx = incoming_tx
            .with_routing_state(state.clone())
            .with_state_in_db(None)
            .build()
            .unwrap();

        dandelion_pool_manager
            .ready()
            .await?
            .call(incoming_tx)
            .await?;
    }
}
