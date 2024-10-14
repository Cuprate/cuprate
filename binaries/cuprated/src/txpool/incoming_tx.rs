use std::{
    collections::HashSet,
    future::ready,
    sync::Arc,
    task::{Context, Poll},
};

use bytes::Bytes;
use dashmap::DashSet;
use futures::{future::BoxFuture, FutureExt};
use monero_serai::transaction::Transaction;
use sha3::{Digest, Sha3_256};
use tower::{Service, ServiceExt};

use cuprate_consensus::{
    transactions::new_tx_verification_data, BlockChainContextRequest, BlockChainContextResponse,
    BlockChainContextService, ExtendedConsensusError, TxVerifierService, VerifyTxRequest,
    VerifyTxResponse,
};
use cuprate_dandelion_tower::{
    pool::{DandelionPoolService, IncomingTx, IncomingTxBuilder},
    State, TxState,
};
use cuprate_helper::asynch::rayon_spawn_async;
use cuprate_txpool::service::{
    interface::{TxpoolReadRequest, TxpoolReadResponse, TxpoolWriteRequest, TxpoolWriteResponse},
    TxpoolReadHandle, TxpoolWriteHandle,
};
use cuprate_types::TransactionVerificationData;
use cuprate_wire::NetworkAddress;

use crate::{
    blockchain::ConcreteTxVerifierService,
    constants::PANIC_CRITICAL_SERVICE_ERROR,
    signals::REORG_LOCK,
    txpool::txs_being_handled::{tx_blob_hash, TxBeingHandledLocally, TxsBeingHandled},
};

/// An error that can happen handling an incoming tx.
pub enum IncomingTxError {
    Parse(std::io::Error),
    Consensus(ExtendedConsensusError),
    DuplicateTransaction,
}

/// Incoming transactions.
pub struct IncomingTxs {
    pub txs: Vec<Bytes>,
    pub state: TxState<NetworkAddress>,
}

///  The transaction type used for dandelion++.
#[derive(Clone)]
pub struct DandelionTx(pub Bytes);

/// A transaction ID/hash.
pub(super) type TxId = [u8; 32];

/// The service than handles incoming transaction pool transactions.
///
/// This service handles everything including verifying the tx, adding it to the pool and routing it to other nodes.
pub struct IncomingTxHandler {
    /// A store of txs currently being handled in incoming tx requests.
    pub(super) txs_being_handled: TxsBeingHandled,
    /// The blockchain context cache.
    pub(super) blockchain_context_cache: BlockChainContextService,
    /// The dandelion txpool manager.
    pub(super) dandelion_pool_manager: DandelionPoolService<DandelionTx, TxId, NetworkAddress>,
    /// The transaction verifier service.
    pub(super) tx_verifier_service: ConcreteTxVerifierService,
    /// The txpool write handle.
    pub(super) txpool_write_handle: TxpoolWriteHandle,
    /// The txpool read handle.
    pub(super) txpool_read_handle: TxpoolReadHandle,
}

impl Service<IncomingTxs> for IncomingTxHandler {
    type Response = ();
    type Error = IncomingTxError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: IncomingTxs) -> Self::Future {
        handle_incoming_txs(
            req.txs,
            req.state,
            self.txs_being_handled.clone(),
            self.blockchain_context_cache.clone(),
            self.tx_verifier_service.clone(),
            self.txpool_write_handle.clone(),
            self.txpool_read_handle.clone(),
            self.dandelion_pool_manager.clone(),
        )
        .boxed()
    }
}

#[expect(clippy::too_many_arguments)]
async fn handle_incoming_txs(
    txs: Vec<Bytes>,
    state: TxState<NetworkAddress>,
    txs_being_handled: TxsBeingHandled,
    mut blockchain_context_cache: BlockChainContextService,
    mut tx_verifier_service: ConcreteTxVerifierService,
    mut txpool_write_handle: TxpoolWriteHandle,
    mut txpool_read_handle: TxpoolReadHandle,
    mut dandelion_pool_manager: DandelionPoolService<DandelionTx, TxId, NetworkAddress>,
) -> Result<(), IncomingTxError> {
    let reorg_guard = REORG_LOCK.read().await;

    let (txs, stem_pool_txs, txs_being_handled_guard) =
        prepare_incoming_txs(txs, txs_being_handled, &mut txpool_read_handle).await?;

    let BlockChainContextResponse::Context(context) = blockchain_context_cache
        .ready()
        .await
        .expect(PANIC_CRITICAL_SERVICE_ERROR)
        .call(BlockChainContextRequest::Context)
        .await
        .expect(PANIC_CRITICAL_SERVICE_ERROR)
    else {
        unreachable!()
    };

    let context = context.unchecked_blockchain_context();

    tx_verifier_service
        .ready()
        .await
        .expect(PANIC_CRITICAL_SERVICE_ERROR)
        .call(VerifyTxRequest::Prepped {
            txs: txs.clone(),
            current_chain_height: context.chain_height,
            top_hash: context.top_hash,
            time_for_time_lock: context.current_adjusted_timestamp_for_time_lock(),
            hf: context.current_hf,
        })
        .await
        .map_err(IncomingTxError::Consensus)?;

    for tx in txs {
        handle_valid_tx(
            tx,
            state.clone(),
            &mut txpool_write_handle,
            &mut dandelion_pool_manager,
        )
        .await;
    }

    for stem_tx in stem_pool_txs {
        rerelay_stem_tx(
            &stem_tx,
            state.clone(),
            &mut txpool_read_handle,
            &mut dandelion_pool_manager,
        )
        .await;
    }

    Ok(())
}

/// Prepares the incoming transactions for verification.
///
/// This will filter out all transactions already in the pool or txs already being handled in another request.
async fn prepare_incoming_txs(
    tx_blobs: Vec<Bytes>,
    txs_being_handled: TxsBeingHandled,
    txpool_read_handle: &mut TxpoolReadHandle,
) -> Result<
    (
        Vec<Arc<TransactionVerificationData>>,
        Vec<TxId>,
        TxBeingHandledLocally,
    ),
    IncomingTxError,
> {
    let mut tx_blob_hashes = HashSet::new();
    let mut txs_being_handled_locally = txs_being_handled.local_tracker();

    // Compute the blob hash for each tx and filter out the txs currently being handled by another incoming tx batch.
    let txs = tx_blobs
        .into_iter()
        .filter_map(|tx_blob| {
            let tx_blob_hash = tx_blob_hash(tx_blob.as_ref());

            // If a duplicate is in here the incoming tx batch contained the same tx twice.
            if !tx_blob_hashes.insert(tx_blob_hash) {
                return Some(Err(IncomingTxError::DuplicateTransaction));
            }

            // If a duplicate is here it is being handled in another batch.
            if !txs_being_handled_locally.try_add_tx(tx_blob_hash) {
                return None;
            }

            Some(Ok((tx_blob_hash, tx_blob)))
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Filter the txs already in the txpool out.
    // This will leave the txs already in the pool in [`TxBeingHandledLocally`] but that shouldn't be an issue.
    let TxpoolReadResponse::FilterKnownTxBlobHashes {
        unknown_blob_hashes,
        stem_pool_hashes,
    } = txpool_read_handle
        .ready()
        .await
        .expect(PANIC_CRITICAL_SERVICE_ERROR)
        .call(TxpoolReadRequest::FilterKnownTxBlobHashes(tx_blob_hashes))
        .await
        .expect(PANIC_CRITICAL_SERVICE_ERROR)
    else {
        unreachable!()
    };

    // Now prepare the txs for verification.
    rayon_spawn_async(move || {
        let txs = txs
            .into_iter()
            .filter_map(|(tx_blob_hash, tx_blob)| {
                if unknown_blob_hashes.contains(&tx_blob_hash) {
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
            .collect::<Result<Vec<_>, IncomingTxError>>()?;

        Ok((txs, stem_pool_hashes, txs_being_handled_locally))
    })
    .await
}

async fn handle_valid_tx(
    tx: Arc<TransactionVerificationData>,
    state: TxState<NetworkAddress>,
    txpool_write_handle: &mut TxpoolWriteHandle,
    dandelion_pool_manager: &mut DandelionPoolService<DandelionTx, TxId, NetworkAddress>,
) {
    let incoming_tx =
        IncomingTxBuilder::new(DandelionTx(Bytes::copy_from_slice(&tx.tx_blob)), tx.tx_hash);

    let TxpoolWriteResponse::AddTransaction(double_spend) = txpool_write_handle
        .ready()
        .await
        .expect(PANIC_CRITICAL_SERVICE_ERROR)
        .call(TxpoolWriteRequest::AddTransaction {
            tx,
            state_stem: state.state_stem(),
        })
        .await
        .expect("TODO")
    else {
        unreachable!()
    };

    // TODO: track double spends to quickly ignore them from their blob hash.
    if let Some(tx_hash) = double_spend {
        return;
    };

    // TODO: There is a race condition possible if a tx and block come in at the same time <https://github.com/Cuprate/cuprate/issues/314>.

    let incoming_tx = incoming_tx
        .with_routing_state(state)
        .with_state_in_db(None)
        .build()
        .unwrap();

    dandelion_pool_manager
        .ready()
        .await
        .expect(PANIC_CRITICAL_SERVICE_ERROR)
        .call(incoming_tx)
        .await
        .expect(PANIC_CRITICAL_SERVICE_ERROR);
}

async fn rerelay_stem_tx(
    tx_hash: &TxId,
    state: TxState<NetworkAddress>,
    txpool_read_handle: &mut TxpoolReadHandle,
    dandelion_pool_manager: &mut DandelionPoolService<DandelionTx, TxId, NetworkAddress>,
) {
    let TxpoolReadResponse::TxBlob { tx_blob, .. } = txpool_read_handle
        .ready()
        .await
        .expect(PANIC_CRITICAL_SERVICE_ERROR)
        .call(TxpoolReadRequest::TxBlob(*tx_hash))
        .await
        .expect("TODO")
    else {
        unreachable!()
    };

    let incoming_tx =
        IncomingTxBuilder::new(DandelionTx(Bytes::copy_from_slice(&tx_blob)), *tx_hash);

    // TODO: fill this in properly.
    let incoming_tx = incoming_tx
        .with_routing_state(state)
        .with_state_in_db(Some(State::Stem))
        .build()
        .unwrap();

    dandelion_pool_manager
        .ready()
        .await
        .expect(PANIC_CRITICAL_SERVICE_ERROR)
        .call(incoming_tx)
        .await
        .expect(PANIC_CRITICAL_SERVICE_ERROR);
}
