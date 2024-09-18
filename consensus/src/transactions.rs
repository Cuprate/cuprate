//! # Transaction Verifier Service.
//!
//! This module contains the [`TxVerifierService`] which handles consensus validation of transactions.
//!
use std::{
    collections::HashSet,
    future::Future,
    ops::Deref,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::FutureExt;
use monero_serai::transaction::{Input, Timelock, Transaction};
use rayon::prelude::*;
use tower::{Service, ServiceExt};
use tracing::instrument;

use cuprate_consensus_rules::{
    transactions::{
        check_decoy_info, check_transaction_contextual, check_transaction_semantic,
        output_unlocked, TransactionError,
    },
    ConsensusError, HardFork,
};
use cuprate_helper::asynch::rayon_spawn_async;
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    CachedVerificationState, TransactionVerificationData, TxVersion,
};

use crate::{
    batch_verifier::MultiThreadedBatchVerifier,
    transactions::contextual_data::{batch_get_decoy_info, batch_get_ring_member_info},
    Database, ExtendedConsensusError,
};

pub mod contextual_data;
mod free;

pub use free::new_tx_verification_data;

/// A struct representing the type of validation that needs to be completed for this transaction.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum VerificationNeeded {
    /// Both semantic validation and contextual validation are needed.
    SemanticAndContextual,
    /// Only contextual validation is needed.
    Contextual,
}

/// A request to verify a transaction.
pub enum VerifyTxRequest {
    /// Verifies a batch of prepared txs.
    Prepped {
        /// The transactions to verify.
        // TODO: Can we use references to remove the Vec? wont play nicely with Service though
        txs: Vec<Arc<TransactionVerificationData>>,
        /// The current chain height.
        current_chain_height: usize,
        /// The top block hash.
        top_hash: [u8; 32],
        /// The value for time to use to check time locked outputs.
        time_for_time_lock: u64,
        /// The current [`HardFork`]
        hf: HardFork,
    },
    /// Verifies a batch of new txs.
    /// Returning [`VerifyTxResponse::OkPrepped`]
    New {
        /// The transactions to verify.
        txs: Vec<Transaction>,
        /// The current chain height.
        current_chain_height: usize,
        /// The top block hash.
        top_hash: [u8; 32],
        /// The value for time to use to check time locked outputs.
        time_for_time_lock: u64,
        /// The current [`HardFork`]
        hf: HardFork,
    },
}

/// A response from a verify transaction request.
#[derive(Debug)]
pub enum VerifyTxResponse {
    OkPrepped(Vec<Arc<TransactionVerificationData>>),
    Ok,
}

/// The transaction verifier service.
#[derive(Clone)]
pub struct TxVerifierService<D> {
    /// The database.
    database: D,
}

impl<D> TxVerifierService<D>
where
    D: Database + Clone + Send + 'static,
    D::Future: Send + 'static,
{
    /// Creates a new [`TxVerifierService`].
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
                VerifyTxRequest::New {
                    txs,
                    current_chain_height,
                    top_hash,
                    time_for_time_lock,
                    hf,
                } => {
                    prep_and_verify_transactions(
                        database,
                        txs,
                        current_chain_height,
                        top_hash,
                        time_for_time_lock,
                        hf,
                    )
                    .await
                }

                VerifyTxRequest::Prepped {
                    txs,
                    current_chain_height,
                    top_hash,
                    time_for_time_lock,
                    hf,
                } => {
                    verify_prepped_transactions(
                        database,
                        &txs,
                        current_chain_height,
                        top_hash,
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

/// Prepares transactions for verification, then verifies them.
async fn prep_and_verify_transactions<D>(
    database: D,
    txs: Vec<Transaction>,
    current_chain_height: usize,
    top_hash: [u8; 32],
    time_for_time_lock: u64,
    hf: HardFork,
) -> Result<VerifyTxResponse, ExtendedConsensusError>
where
    D: Database + Clone + Sync + Send + 'static,
{
    let span = tracing::info_span!("prep_txs", amt = txs.len());

    tracing::debug!(parent: &span, "prepping transactions for verification.");
    let txs = rayon_spawn_async(|| {
        txs.into_par_iter()
            .map(|tx| new_tx_verification_data(tx).map(Arc::new))
            .collect::<Result<Vec<_>, _>>()
    })
    .await?;

    verify_prepped_transactions(
        database,
        &txs,
        current_chain_height,
        top_hash,
        time_for_time_lock,
        hf,
    )
    .await?;

    Ok(VerifyTxResponse::OkPrepped(txs))
}

#[instrument(name = "verify_txs", skip_all, fields(amt = txs.len()) level = "info")]
async fn verify_prepped_transactions<D>(
    mut database: D,
    txs: &[Arc<TransactionVerificationData>],
    current_chain_height: usize,
    top_hash: [u8; 32],
    time_for_time_lock: u64,
    hf: HardFork,
) -> Result<VerifyTxResponse, ExtendedConsensusError>
where
    D: Database + Clone + Sync + Send + 'static,
{
    tracing::debug!("Verifying transactions");

    tracing::trace!("Checking for duplicate key images");

    let mut spent_kis = HashSet::with_capacity(txs.len());

    txs.iter().try_for_each(|tx| {
        tx.tx.prefix().inputs.iter().try_for_each(|input| {
            if let Input::ToKey { key_image, .. } = input {
                if !spent_kis.insert(key_image.compress().0) {
                    tracing::debug!("Duplicate key image found in batch.");
                    return Err(ConsensusError::Transaction(TransactionError::KeyImageSpent));
                }
            }

            Ok(())
        })
    })?;

    let BlockchainResponse::KeyImagesSpent(kis_spent) = database
        .ready()
        .await?
        .call(BlockchainReadRequest::KeyImagesSpent(spent_kis))
        .await?
    else {
        panic!("Database sent incorrect response!");
    };

    if kis_spent {
        tracing::debug!("One or more key images in batch already spent.");
        Err(ConsensusError::Transaction(TransactionError::KeyImageSpent))?;
    }

    let mut verified_at_block_hashes = txs
        .iter()
        .filter_map(|txs| {
            txs.cached_verification_state
                .lock()
                .unwrap()
                .verified_at_block_hash()
        })
        .collect::<HashSet<_>>();

    tracing::trace!(
        "Verified at hashes len: {}.",
        verified_at_block_hashes.len()
    );

    if !verified_at_block_hashes.is_empty() {
        tracing::trace!("Filtering block hashes not in the main chain.");

        let BlockchainResponse::FilterUnknownHashes(known_hashes) = database
            .ready()
            .await?
            .call(BlockchainReadRequest::FilterUnknownHashes(
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
            top_hash,
            time_for_time_lock,
            hf,
            database
        )
    )?;

    Ok(VerifyTxResponse::Ok)
}

#[allow(clippy::type_complexity)] // I don't think the return is too complex
fn transactions_needing_verification(
    txs: &[Arc<TransactionVerificationData>],
    hashes_in_main_chain: HashSet<[u8; 32]>,
    current_hf: &HardFork,
    current_chain_height: usize,
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

    for tx in txs.iter() {
        let guard = tx.cached_verification_state.lock().unwrap();

        match guard.deref() {
            CachedVerificationState::NotVerified => {
                drop(guard);
                full_validation_transactions
                    .push((tx.clone(), VerificationNeeded::SemanticAndContextual));
                continue;
            }
            CachedVerificationState::ValidAtHashAndHF { block_hash, hf } => {
                if current_hf != hf {
                    drop(guard);
                    full_validation_transactions
                        .push((tx.clone(), VerificationNeeded::SemanticAndContextual));
                    continue;
                }

                if !hashes_in_main_chain.contains(block_hash) {
                    drop(guard);
                    full_validation_transactions.push((tx.clone(), VerificationNeeded::Contextual));
                    continue;
                }
            }
            CachedVerificationState::ValidAtHashAndHFWithTimeBasedLock {
                block_hash,
                hf,
                time_lock,
            } => {
                if current_hf != hf {
                    drop(guard);
                    full_validation_transactions
                        .push((tx.clone(), VerificationNeeded::SemanticAndContextual));
                    continue;
                }

                if !hashes_in_main_chain.contains(block_hash) {
                    drop(guard);
                    full_validation_transactions.push((tx.clone(), VerificationNeeded::Contextual));
                    continue;
                }

                // If the time lock is still locked then the transaction is invalid.
                if !output_unlocked(time_lock, current_chain_height, time_for_time_lock, hf) {
                    return Err(ConsensusError::Transaction(
                        TransactionError::OneOrMoreRingMembersLocked,
                    ));
                }
            }
        }

        if tx.version == TxVersion::RingSignatures {
            drop(guard);
            partial_validation_transactions.push(tx.clone());
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
    // Decoy info is not validated for V1 txs.
    if hf == HardFork::V1 || txs.is_empty() {
        return Ok(());
    }

    batch_get_decoy_info(&txs, hf, database)
        .await?
        .try_for_each(|decoy_info| decoy_info.and_then(|di| Ok(check_decoy_info(&di, &hf)?)))?;

    Ok(())
}

async fn verify_transactions<D>(
    txs: Vec<(Arc<TransactionVerificationData>, VerificationNeeded)>,
    current_chain_height: usize,
    top_hash: [u8; 32],
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
        let batch_verifier = MultiThreadedBatchVerifier::new(rayon::current_num_threads());

        txs.par_iter()
            .zip(txs_ring_member_info.par_iter())
            .try_for_each(|((tx, verification_needed), ring)| {
                // do semantic validation if needed.
                if *verification_needed == VerificationNeeded::SemanticAndContextual {
                    let fee = check_transaction_semantic(
                        &tx.tx,
                        tx.tx_blob.len(),
                        tx.tx_weight,
                        &tx.tx_hash,
                        &hf,
                        &batch_verifier,
                    )?;
                    // make sure we calculated the right fee.
                    assert_eq!(fee, tx.fee);
                }

                // Both variants of `VerificationNeeded` require contextual validation.
                check_transaction_contextual(
                    &tx.tx,
                    ring,
                    current_chain_height,
                    current_time_lock_timestamp,
                    &hf,
                )?;

                Ok::<_, ConsensusError>(())
            })?;

        if !batch_verifier.verify() {
            return Err(ExtendedConsensusError::OneOrMoreBatchVerificationStatementsInvalid);
        }

        txs.iter()
            .zip(txs_ring_member_info)
            .for_each(|((tx, _), ring)| {
                *tx.cached_verification_state.lock().unwrap() = if ring.time_locked_outs.is_empty()
                {
                    // no outputs with time-locks used.
                    CachedVerificationState::ValidAtHashAndHF {
                        block_hash: top_hash,
                        hf,
                    }
                } else {
                    // an output with a time-lock was used, check if it was time-based.
                    let youngest_timebased_lock = ring
                        .time_locked_outs
                        .iter()
                        .filter_map(|lock| match lock {
                            Timelock::Time(time) => Some(*time),
                            _ => None,
                        })
                        .min();

                    if let Some(time) = youngest_timebased_lock {
                        // time-based lock used.
                        CachedVerificationState::ValidAtHashAndHFWithTimeBasedLock {
                            block_hash: top_hash,
                            hf,
                            time_lock: Timelock::Time(time),
                        }
                    } else {
                        // no time-based locked output was used.
                        CachedVerificationState::ValidAtHashAndHF {
                            block_hash: top_hash,
                            hf,
                        }
                    }
                }
            });

        Ok(())
    })
    .await?;

    Ok(())
}
