use std::{
    collections::HashSet,
    future::Future,
    ops::Deref,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::FutureExt;
use monero_serai::ringct::RctType;
use monero_serai::transaction::{Timelock, Transaction};
use rayon::prelude::*;
use tokio::task::JoinSet;
use tower::{Service, ServiceExt};
use tracing::instrument;

use cuprate_consensus_rules::transactions::{check_decoy_info, output_unlocked};
use cuprate_consensus_rules::{
    transactions::{
        check_transaction_contextual, check_transaction_semantic, TransactionError,
        TxRingMembersInfo,
    },
    ConsensusError, HardFork, TxVersion,
};
use cuprate_helper::asynch::rayon_spawn_async;

use crate::transactions::contextual_data::{batch_get_decoy_info, batch_get_ring_member_info};
use crate::{
    batch_verifier::MultiThreadedBatchVerifier, Database, DatabaseRequest, DatabaseResponse,
    ExtendedConsensusError,
};

pub mod contextual_data;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum VerificationNeeded {
    SemanticAndContextual,
    Contextual,
}

/// Represents if a transaction has been fully validated and under what conditions
/// the transaction is valid in the future.
#[derive(Debug, Clone)]
pub enum CachedVerificationState {
    /// The transaction has not been fully validated.
    NotVerified,
    /// The transaction is valid* if the block represented by this hash is in the blockchain and the [`HardFork`]
    /// is the same.
    ///
    /// *V1 transactions require checks on their ring-length even if this hash is in the blockchain.
    ValidAtHashAndHF([u8; 32], HardFork),
    /// The transaction is valid* if the block represented by this hash is in the blockchain _and_ this
    /// given time lock is unlocked. The time lock here will represent the youngest used time based lock
    /// (If the transaction uses any time based time locks). This is because time locks are not monotonic
    /// so unlocked outputs cold become re-locked.
    ///
    /// *V1 transactions require checks on their ring-length even if this hash is in the blockchain.
    ValidAtHashAndHFWithTimeBasedLock([u8; 32], HardFork, Timelock),
}

impl CachedVerificationState {
    fn verified_at_block_hash(&self) -> Option<[u8; 32]> {
        match self {
            CachedVerificationState::NotVerified => None,
            CachedVerificationState::ValidAtHashAndHF(hash, _)
            | CachedVerificationState::ValidAtHashAndHFWithTimeBasedLock(hash, _, _) => Some(*hash),
        }
    }
}

/// Data needed to verify a transaction.
#[derive(Debug)]
pub struct TransactionVerificationData {
    pub tx: Transaction,
    pub version: TxVersion,
    pub tx_blob: Vec<u8>,
    pub tx_weight: usize,
    pub fee: u64,
    pub tx_hash: [u8; 32],
    pub cached_verification_state: CachedVerificationState,
}

impl TransactionVerificationData {
    pub fn new(tx: Transaction) -> Result<TransactionVerificationData, ConsensusError> {
        let tx_hash = tx.hash();
        let tx_blob = tx.serialize();

        // the tx weight is only different from the blobs length for bp(+) txs.
        let tx_weight = match tx.rct_signatures.rct_type() {
            RctType::Bulletproofs
            | RctType::BulletproofsCompactAmount
            | RctType::Clsag
            | RctType::BulletproofsPlus => tx.weight(),
            _ => tx_blob.len(),
        };

        Ok(TransactionVerificationData {
            tx_hash,
            tx_blob,
            tx_weight,
            fee: tx.rct_signatures.base.fee,
            cached_verification_state: CachedVerificationState::NotVerified,
            version: TxVersion::from_raw(tx.prefix.version)
                .ok_or(TransactionError::TransactionVersionInvalid)?,
            tx,
        })
    }
}

pub enum VerifyTxRequest {
    /// Verifies transactions in the context of a block.
    Prepped {
        txs: Vec<Arc<TransactionVerificationData>>,
        current_chain_height: u64,
        time_for_time_lock: u64,
        hf: HardFork,
    },
}

pub enum VerifyTxResponse {
    Ok,
}

#[derive(Clone)]
pub struct TxVerifierService<D: Clone> {
    database: D,
}

impl<D> TxVerifierService<D>
where
    D: Database + Clone + Send + 'static,
    D::Future: Send + 'static,
{
    pub fn new(database: D) -> TxVerifierService<D> {
        TxVerifierService { database }
    }
}

impl<D> Service<VerifyTxRequest> for TxVerifierService<D>
where
    D: Database + Clone + Send + Sync + 'static,
    D::Future: Send + 'static,
{
    type Response = VerifyTxResponse;
    type Error = ExtendedConsensusError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.database.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: VerifyTxRequest) -> Self::Future {
        let database = self.database.clone();

        async move {
            match req {
                VerifyTxRequest::Prepped {
                    txs,
                    current_chain_height,
                    time_for_time_lock,
                    hf,
                } => {
                    verify_prepped_transactions(
                        database,
                        txs,
                        current_chain_height,
                        time_for_time_lock,
                        hf,
                    )
                    .await
                }
            }
        }
        .boxed()
    }
}

#[instrument(name = "verify_txs", skip_all, level = "info")]
async fn verify_prepped_transactions<D>(
    mut database: D,
    txs: Vec<Arc<TransactionVerificationData>>,
    current_chain_height: u64,
    time_for_time_lock: u64,
    hf: HardFork,
) -> Result<VerifyTxResponse, ExtendedConsensusError>
where
    D: Database + Clone + Sync + Send + 'static,
{
    tracing::debug!("Verifying {} transactions", txs.len());

    let mut verified_at_block_hashes = txs
        .iter()
        .filter_map(|txs| txs.cached_verification_state.verified_at_block_hash())
        .collect::<HashSet<_>>();

    tracing::trace!(
        "Verified at hashes len: {}.",
        verified_at_block_hashes.len()
    );

    if !verified_at_block_hashes.is_empty() {
        tracing::trace!("Filtering block hashes not in the main chain.");

        let DatabaseResponse::FilteredHashes(known_hashes) = database
            .ready()
            .await?
            .call(DatabaseRequest::FilterUnknownHashes(
                verified_at_block_hashes,
            ))
            .await?
        else {
            panic!("Database returned wrong response!");
        };
        verified_at_block_hashes = known_hashes;
    }

    let (txs_needing_full_verification, txs_needing_partial_verification) =
        transactions_needing_verification(
            txs,
            verified_at_block_hashes,
            &hf,
            current_chain_height,
            time_for_time_lock,
        )?;

    futures::try_join!(
        verify_transactions_decoy_info(txs_needing_partial_verification, hf, database.clone()),
        verify_transactions(
            txs_needing_full_verification,
            current_chain_height,
            time_for_time_lock,
            hf,
            database
        )
    )?;

    Ok(VerifyTxResponse::Ok)

    /*
    let spent_kis = Arc::new(std::sync::Mutex::new(HashSet::new()));

    let cloned_spent_kis = spent_kis.clone();

    rayon_spawn_async(move || {
        txs.par_iter().try_for_each(|tx| {
            verify_transaction_for_block(
                tx,
                current_chain_height,
                time_for_time_lock,
                hf,
                cloned_spent_kis,
            )
        })
    })
    .await?;

    let DatabaseResponse::KeyImagesSpent(kis_spent) = database
        .oneshot(DatabaseRequest::KeyImagesSpent(
            Arc::into_inner(spent_kis).unwrap().into_inner().unwrap(),
        ))
        .await?
    else {
        panic!("Database sent incorrect response!");
    };

    if kis_spent {
        Err(ConsensusError::Transaction(TransactionError::KeyImageSpent))?;
    }

    Ok(VerifyTxResponse::Ok)

     */
}

fn transactions_needing_verification(
    txs: Vec<Arc<TransactionVerificationData>>,
    hashes_in_main_chain: HashSet<[u8; 32]>,
    current_hf: &HardFork,
    current_chain_height: u64,
    time_for_time_lock: u64,
) -> Result<
    (
        Vec<(Arc<TransactionVerificationData>, VerificationNeeded)>,
        Vec<Arc<TransactionVerificationData>>,
    ),
    ConsensusError,
> {
    // txs needing full validation: semantic and/or contextual
    let mut full_validation_transactions = Vec::new();
    // txs needing partial _contextual_ validation, not semantic.
    let mut partial_validation_transactions = Vec::new();

    for tx in txs.into_iter() {
        match tx.cached_verification_state {
            CachedVerificationState::NotVerified => {
                full_validation_transactions.push((tx, VerificationNeeded::SemanticAndContextual));
                continue;
            }
            CachedVerificationState::ValidAtHashAndHF(hash, hf) => {
                if current_hf != &hf {
                    full_validation_transactions
                        .push((tx, VerificationNeeded::SemanticAndContextual));
                    continue;
                }

                if !hashes_in_main_chain.contains(&hash) {
                    full_validation_transactions.push((tx, VerificationNeeded::Contextual));
                    continue;
                }
            }
            CachedVerificationState::ValidAtHashAndHFWithTimeBasedLock(hash, hf, lock) => {
                if current_hf != &hf {
                    full_validation_transactions
                        .push((tx, VerificationNeeded::SemanticAndContextual));
                    continue;
                }

                // If the time lock is still locked then the transaction is invalid.
                if !output_unlocked(&lock, current_chain_height, time_for_time_lock, &hf) {
                    return Err(ConsensusError::Transaction(
                        TransactionError::OneOrMoreRingMembersLocked,
                    ));
                }

                if !hashes_in_main_chain.contains(&hash) {
                    full_validation_transactions.push((tx, VerificationNeeded::Contextual));
                    continue;
                }
            }
            _ => (),
        }

        if tx.version == TxVersion::RingSignatures {
            partial_validation_transactions.push(tx);
            continue;
        }
    }

    Ok((
        full_validation_transactions,
        partial_validation_transactions,
    ))
}

async fn verify_transactions_decoy_info<D>(
    txs: Vec<Arc<TransactionVerificationData>>,
    hf: HardFork,
    database: D,
) -> Result<(), ExtendedConsensusError>
where
    D: Database + Clone + Sync + Send + 'static,
{
    batch_get_decoy_info(&txs, &hf, database)
        .await?
        .try_for_each(|decoy_info| decoy_info.and_then(|di| Ok(check_decoy_info(&di, &hf)?)))?;

    Ok(())
}

async fn verify_transactions<D>(
    txs: Vec<(Arc<TransactionVerificationData>, VerificationNeeded)>,
    current_chain_height: u64,
    current_time_lock_timestamp: u64,
    hf: HardFork,
    database: D,
) -> Result<(), ExtendedConsensusError>
where
    D: Database + Clone + Sync + Send + 'static,
{
    let txs_ring_member_info =
        batch_get_ring_member_info(txs.iter().map(|(tx, _)| tx), &hf, database).await?;

    rayon_spawn_async(move || {
        let batch_veriifier = MultiThreadedBatchVerifier::new(rayon::current_num_threads());

        txs.par_iter().zip(txs_ring_member_info).try_for_each(
            |((tx, verification_needed), ring)| {
                // do semantic validation if needed.
                if *verification_needed == VerificationNeeded::SemanticAndContextual {
                    batch_veriifier.queue_statement(|verifier| {
                        let fee = check_transaction_semantic(
                            &tx.tx,
                            tx.tx_blob.len(),
                            tx.tx_weight,
                            &tx.tx_hash,
                            &hf,
                            verifier,
                        )?;
                        // make sure monero-serai calculated the same fee.
                        assert_eq!(fee, tx.fee);
                        Ok(())
                    })?;
                }

                // Both variants of `VerificationNeeded` require contextual validation.
                Ok::<_, ConsensusError>(check_transaction_contextual(
                    &tx.tx,
                    &ring,
                    current_chain_height,
                    current_time_lock_timestamp,
                    &hf,
                )?)
            },
        )?;

        if !batch_veriifier.verify() {
            return Err(ExtendedConsensusError::OneOrMoreBatchVerificationStatmentsInvalid);
        }

        Ok(())
    })
    .await?;

    Ok(())
}
