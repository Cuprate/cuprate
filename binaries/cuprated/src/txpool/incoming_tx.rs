use std::{
    collections::HashSet,
    sync::Arc,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures::{future::BoxFuture, FutureExt};
use monero_serai::transaction::Transaction;
use tower::{BoxError, Service, ServiceExt};

use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_consensus::transactions::{start_tx_verification, PrepTransactions};
use cuprate_consensus::{
    transactions::new_tx_verification_data, BlockChainContextRequest, BlockChainContextResponse,
    BlockchainContextService, ExtendedConsensusError,
};
use cuprate_dandelion_tower::{
    pool::{DandelionPoolService, IncomingTxBuilder},
    State, TxState,
};
use cuprate_helper::asynch::rayon_spawn_async;
use cuprate_p2p::NetworkInterface;
use cuprate_p2p_core::{ClearNet, Tor};
use cuprate_txpool::{
    service::{
        interface::{
            TxpoolReadRequest, TxpoolReadResponse, TxpoolWriteRequest, TxpoolWriteResponse,
        },
        TxpoolReadHandle, TxpoolWriteHandle,
    },
    transaction_blob_hash,
};
use cuprate_types::TransactionVerificationData;

use crate::{
    blockchain::ConsensusBlockchainReadHandle,
    constants::PANIC_CRITICAL_SERVICE_ERROR,
    p2p::CrossNetworkInternalPeerId,
    signals::REORG_LOCK,
    txpool::{
        dandelion::{self, AnonTxService, ConcreteDandelionRouter, MainDandelionRouter},
        relay_rules::{check_tx_relay_rules, RelayRuleError},
        txs_being_handled::{TxsBeingHandled, TxsBeingHandledLocally},
    },
};

/// An error that can happen handling an incoming tx.
#[derive(Debug, thiserror::Error)]
pub enum IncomingTxError {
    #[error("Error parsing tx: {0}")]
    Parse(std::io::Error),
    #[error(transparent)]
    Consensus(ExtendedConsensusError),
    #[error("Duplicate tx in message")]
    DuplicateTransaction,
    #[error("Relay rule was broken: {0}")]
    RelayRule(RelayRuleError),
}

/// Incoming transactions.
pub struct IncomingTxs {
    /// The raw bytes of the transactions.
    pub txs: Vec<Bytes>,
    /// The routing state of the transactions.
    pub state: TxState<CrossNetworkInternalPeerId>,
    /// If [`true`], transactions breaking relay
    /// rules will be ignored and processing will continue,
    /// otherwise the service will return an early error.
    pub drop_relay_rule_errors: bool,
    /// If [`true`], only checks will be done,
    /// the transaction will not be relayed.
    pub do_not_relay: bool,
}

///  The transaction type used for dandelion++.
#[derive(Clone)]
pub struct DandelionTx(pub Bytes);

/// A transaction ID/hash.
pub(super) type TxId = [u8; 32];

/// The service than handles incoming transaction pool transactions.
///
/// This service handles everything including verifying the tx, adding it to the pool and routing it to other nodes.
#[derive(Clone)]
pub struct IncomingTxHandler {
    /// A store of txs currently being handled in incoming tx requests.
    pub(super) txs_being_handled: TxsBeingHandled,
    /// The blockchain context cache.
    pub(super) blockchain_context_cache: BlockchainContextService,
    /// The dandelion txpool manager.
    pub(super) dandelion_pool_manager:
        DandelionPoolService<DandelionTx, TxId, CrossNetworkInternalPeerId>,
    /// The txpool write handle.
    pub(super) txpool_write_handle: TxpoolWriteHandle,
    /// The txpool read handle.
    pub(super) txpool_read_handle: TxpoolReadHandle,
    /// The blockchain read handle.
    pub(super) blockchain_read_handle: ConsensusBlockchainReadHandle,
}

impl IncomingTxHandler {
    /// Initialize the [`IncomingTxHandler`].
    #[expect(clippy::significant_drop_tightening)]
    pub fn init(
        clear_net: NetworkInterface<ClearNet>,
        tor_net: Option<NetworkInterface<Tor>>,
        txpool_write_handle: TxpoolWriteHandle,
        txpool_read_handle: TxpoolReadHandle,
        blockchain_context_cache: BlockchainContextService,
        blockchain_read_handle: BlockchainReadHandle,
    ) -> Self {
        let clearnet_router = dandelion::dandelion_router(clear_net);
        let tor_router = tor_net.map(AnonTxService::new);

        let dandelion_router = MainDandelionRouter::new(clearnet_router, tor_router);

        let dandelion_pool_manager = dandelion::start_dandelion_pool_manager(
            dandelion_router,
            txpool_read_handle.clone(),
            txpool_write_handle.clone(),
        );

        Self {
            txs_being_handled: TxsBeingHandled::new(),
            blockchain_context_cache,
            dandelion_pool_manager,
            txpool_write_handle,
            txpool_read_handle,
            blockchain_read_handle: ConsensusBlockchainReadHandle::new(
                blockchain_read_handle,
                BoxError::from,
            ),
        }
    }
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
            req,
            self.txs_being_handled.clone(),
            self.blockchain_context_cache.clone(),
            self.blockchain_read_handle.clone(),
            self.txpool_write_handle.clone(),
            self.txpool_read_handle.clone(),
            self.dandelion_pool_manager.clone(),
        )
        .boxed()
    }
}

/// Handles the incoming txs.
async fn handle_incoming_txs(
    IncomingTxs {
        txs,
        state,
        drop_relay_rule_errors,
        do_not_relay,
    }: IncomingTxs,
    txs_being_handled: TxsBeingHandled,
    mut blockchain_context_cache: BlockchainContextService,
    blockchain_read_handle: ConsensusBlockchainReadHandle,
    mut txpool_write_handle: TxpoolWriteHandle,
    mut txpool_read_handle: TxpoolReadHandle,
    mut dandelion_pool_manager: DandelionPoolService<DandelionTx, TxId, CrossNetworkInternalPeerId>,
) -> Result<(), IncomingTxError> {
    let _reorg_guard = REORG_LOCK.read().await;

    let (txs, stem_pool_txs, txs_being_handled_guard) =
        prepare_incoming_txs(txs, txs_being_handled, &mut txpool_read_handle).await?;

    let context = blockchain_context_cache.blockchain_context();

    let txs = start_tx_verification()
        .append_prepped_txs(txs)
        .prepare()
        .map_err(|e| IncomingTxError::Consensus(e.into()))?
        .full(
            context.chain_height,
            context.top_hash,
            context.current_adjusted_timestamp_for_time_lock(),
            context.current_hf,
            blockchain_read_handle,
            None,
        )
        .verify()
        .await
        .map_err(IncomingTxError::Consensus)?;

    for tx in txs {
        // TODO: this could be a DoS, if someone spams us with txs that violate these rules?
        // Maybe we should remember these invalid txs for some time to prevent them getting repeatedly sent.
        if let Err(e) = check_tx_relay_rules(&tx, context) {
            if drop_relay_rule_errors {
                tracing::debug!(err = %e, tx = hex::encode(tx.tx_hash), "Tx failed relay check, skipping.");
                continue;
            }

            return Err(IncomingTxError::RelayRule(e));
        }

        if !do_not_relay {
            handle_valid_tx(
                tx,
                state.clone(),
                &mut txpool_write_handle,
                &mut dandelion_pool_manager,
            )
            .await;
        }
    }

    // Re-relay any txs we got in the block that were already in our stem pool.
    if !do_not_relay {
        for stem_tx in stem_pool_txs {
            rerelay_stem_tx(
                &stem_tx,
                state.clone(),
                &mut txpool_read_handle,
                &mut dandelion_pool_manager,
            )
            .await;
        }
    }

    Ok(())
}

/// Prepares the incoming transactions for verification.
///
/// This will filter out all transactions already in the pool or txs already being handled in another request.
///
/// Returns in order:
///   - The [`TransactionVerificationData`] for all the txs we did not already have
///   - The Ids of the transactions in the incoming message that are in our stem-pool
///   - A [`TxsBeingHandledLocally`] guard that prevents verifying the same tx at the same time across 2 tasks.
async fn prepare_incoming_txs(
    tx_blobs: Vec<Bytes>,
    txs_being_handled: TxsBeingHandled,
    txpool_read_handle: &mut TxpoolReadHandle,
) -> Result<
    (
        Vec<TransactionVerificationData>,
        Vec<TxId>,
        TxsBeingHandledLocally,
    ),
    IncomingTxError,
> {
    let mut tx_blob_hashes = HashSet::new();
    let mut txs_being_handled_locally = txs_being_handled.local_tracker();

    // Compute the blob hash for each tx and filter out the txs currently being handled by another incoming tx batch.
    let txs = tx_blobs
        .into_iter()
        .filter_map(|tx_blob| {
            let tx_blob_hash = transaction_blob_hash(&tx_blob);

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

                Ok(tx)
            })
            .collect::<Result<Vec<_>, IncomingTxError>>()?;

        Ok((txs, stem_pool_hashes, txs_being_handled_locally))
    })
    .await
}

/// Handle a verified tx.
///
/// This will add the tx to the txpool and route it to the network.
async fn handle_valid_tx(
    tx: TransactionVerificationData,
    state: TxState<CrossNetworkInternalPeerId>,
    txpool_write_handle: &mut TxpoolWriteHandle,
    dandelion_pool_manager: &mut DandelionPoolService<
        DandelionTx,
        TxId,
        CrossNetworkInternalPeerId,
    >,
) {
    let incoming_tx =
        IncomingTxBuilder::new(DandelionTx(Bytes::copy_from_slice(&tx.tx_blob)), tx.tx_hash);

    let TxpoolWriteResponse::AddTransaction(double_spend) = txpool_write_handle
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

    // TODO: track double spends to quickly ignore them from their blob hash.
    if let Some(tx_hash) = double_spend {
        return;
    }

    // TODO: There is a race condition possible if a tx and block come in at the same time: <https://github.com/Cuprate/cuprate/issues/314>.

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

/// Re-relay a tx that was already in our stem pool.
async fn rerelay_stem_tx(
    tx_hash: &TxId,
    state: TxState<CrossNetworkInternalPeerId>,
    txpool_read_handle: &mut TxpoolReadHandle,
    dandelion_pool_manager: &mut DandelionPoolService<
        DandelionTx,
        TxId,
        CrossNetworkInternalPeerId,
    >,
) {
    let Ok(TxpoolReadResponse::TxBlob { tx_blob, .. }) = txpool_read_handle
        .ready()
        .await
        .expect(PANIC_CRITICAL_SERVICE_ERROR)
        .call(TxpoolReadRequest::TxBlob(*tx_hash))
        .await
    else {
        // The tx could have been dropped from the pool.
        return;
    };

    let incoming_tx =
        IncomingTxBuilder::new(DandelionTx(Bytes::copy_from_slice(&tx_blob)), *tx_hash);

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
