//! # Transaction Verifier Service.
//!
//! This module contains the transaction validation interface, which can be accessed with [`start_tx_verification`].
//!
//! Transaction verification states will be cached to prevent doing the expensive checks multiple times.
//!
//! ## Example Semantic Verification
//!
//! ```rust
//! # use cuprate_test_utils::data::TX_E2D393;
//! # use monero_serai::transaction::Transaction;
//! use cuprate_consensus::{transactions::start_tx_verification, HardFork, batch_verifier::MultiThreadedBatchVerifier};
//!
//! # fn main() -> Result<(), tower::BoxError> {
//! # let tx = Transaction::read(&mut TX_E2D393).unwrap();
//! let batch_verifier = MultiThreadedBatchVerifier::new(rayon::current_num_threads());
//!
//! let tx = start_tx_verification()
//!              .append_txs(vec![tx])
//!              .prepare()?
//!              .just_semantic(HardFork::V9)
//!              .queue(&batch_verifier)?;
//!
//! assert!(batch_verifier.verify());
//! Ok(())
//! # }
//! ```
use std::collections::HashSet;

use monero_serai::transaction::{Input, Timelock, Transaction};
use rayon::prelude::*;
use tower::ServiceExt;

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
use cuprate_types::output_cache::OutputCache;
use crate::{
    batch_verifier::MultiThreadedBatchVerifier,
    transactions::contextual_data::{batch_get_decoy_info, batch_get_ring_member_info},
    Database, ExtendedConsensusError,
};

pub mod contextual_data;
mod free;

pub use free::new_tx_verification_data;

/// An enum representing the type of validation that needs to be completed for this transaction.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum VerificationNeeded {
    /// Decoy check on a v1 transaction.
    V1DecoyCheck,
    /// Both semantic validation and contextual validation are needed.
    SemanticAndContextual,
    /// Only contextual validation is needed.
    Contextual,
    /// No verification needed.
    None,
}

/// Start the transaction verification process.
pub const fn start_tx_verification() -> PrepTransactions {
    PrepTransactions {
        txs: vec![],
        prepped_txs: vec![],
    }
}

/// The preparation phase of transaction verification.
///
/// The order of transactions will be kept throughout the verification process, transactions
/// inserted with [`PrepTransactions::append_prepped_txs`] will be put before transactions given
/// in [`PrepTransactions::append_txs`]
pub struct PrepTransactions {
    prepped_txs: Vec<TransactionVerificationData>,
    txs: Vec<Transaction>,
}

impl PrepTransactions {
    /// Append some new transactions to prepare.
    #[must_use]
    pub fn append_txs(mut self, mut txs: Vec<Transaction>) -> Self {
        self.txs.append(&mut txs);

        self
    }

    /// Append some already prepped transactions.
    #[must_use]
    pub fn append_prepped_txs(mut self, mut txs: Vec<TransactionVerificationData>) -> Self {
        self.prepped_txs.append(&mut txs);

        self
    }

    /// Prepare the transactions and advance to the next step: [`VerificationWanted`].
    ///
    /// # [`rayon`]
    ///
    /// This function will use [`rayon`] to parallelize the preparation process, so should not be called
    /// in an async function, unless all the transactions given were already prepared, i.e. [`Self::append_prepped_txs`].
    pub fn prepare(mut self) -> Result<VerificationWanted, ConsensusError> {
        if !self.txs.is_empty() {
            self.prepped_txs.append(
                &mut self
                    .txs
                    .into_par_iter()
                    .map(new_tx_verification_data)
                    .collect::<Result<_, _>>()?,
            );
        }

        Ok(VerificationWanted {
            prepped_txs: self.prepped_txs,
        })
    }
}

/// The step where the type of verification is decided.
pub struct VerificationWanted {
    prepped_txs: Vec<TransactionVerificationData>,
}

impl VerificationWanted {
    /// Only semantic verification.
    ///
    /// Semantic verification is verification that can done without other blockchain data. The [`HardFork`]
    /// is technically other blockchain data but excluding it reduces the amount of things that can be checked
    /// significantly, and it is easy to get compared to other blockchain data needed for contextual validation.
    pub fn just_semantic(self, hf: HardFork) -> SemanticVerification {
        SemanticVerification {
            prepped_txs: self.prepped_txs,
            hf,
        }
    }

    /// Full verification.
    ///
    /// Fully verify the transactions, all checks will be performed, if they were already performed then they
    /// won't be done again unless necessary.
    pub fn full<D: Database>(
        self,
        current_chain_height: usize,
        top_hash: [u8; 32],
        time_for_time_lock: u64,
        hf: HardFork,
        database: D,
        output_cache: Option<&OutputCache>
    ) -> FullVerification<D> {
        FullVerification {
            prepped_txs: self.prepped_txs,
            current_chain_height,
            top_hash,
            time_for_time_lock,
            hf,
            database,
            output_cache
        }
    }
}

/// Semantic transaction verification.
///
/// [`VerificationWanted::just_semantic`]
pub struct SemanticVerification {
    prepped_txs: Vec<TransactionVerificationData>,
    hf: HardFork,
}

impl SemanticVerification {
    /// Perform the semantic checks and queue any checks that can be batched into the batch verifier.
    ///
    /// If this function returns [`Ok`] the transaction(s) could still be semantically invalid,
    /// [`MultiThreadedBatchVerifier::verify`] must be called on the `batch_verifier` after.
    pub fn queue(
        mut self,
        batch_verifier: &MultiThreadedBatchVerifier,
    ) -> Result<Vec<TransactionVerificationData>, ConsensusError> {
        self.prepped_txs.par_iter_mut().try_for_each(|tx| {
            let fee = check_transaction_semantic(
                &tx.tx,
                tx.tx_blob.len(),
                tx.tx_weight,
                &tx.tx_hash,
                self.hf,
                batch_verifier,
            )?;
            // make sure we calculated the right fee.
            assert_eq!(fee, tx.fee);

            tx.cached_verification_state = CachedVerificationState::JustSemantic(self.hf);

            Ok::<_, ConsensusError>(())
        })?;

        Ok(self.prepped_txs)
    }
}

/// Full transaction verification.
///
/// [`VerificationWanted::full`]
pub struct FullVerification<'a, D> {
    prepped_txs: Vec<TransactionVerificationData>,

    current_chain_height: usize,
    top_hash: [u8; 32],
    time_for_time_lock: u64,
    hf: HardFork,
    database: D,
    output_cache: Option<&'a OutputCache>
}

impl<D: Database + Clone> FullVerification<'_, D> {
    /// Fully verify each transaction.
    pub async fn verify(
        mut self,
    ) -> Result<Vec<TransactionVerificationData>, ExtendedConsensusError> {
        check_kis_unique(&self.prepped_txs, &mut self.database).await?;

        let hashes_in_main_chain =
            hashes_referenced_in_main_chain(&self.prepped_txs, &mut self.database).await?;

        let (verification_needed, any_v1_decoy_check_needed) = verification_needed(
            &self.prepped_txs,
            &hashes_in_main_chain,
            self.hf,
            self.current_chain_height,
            self.time_for_time_lock,
        )?;

        if any_v1_decoy_check_needed {
            verify_transactions_decoy_info(
                self.prepped_txs
                    .iter()
                    .zip(verification_needed.iter())
                    .filter_map(|(tx, needed)| {
                        if *needed == VerificationNeeded::V1DecoyCheck {
                            Some(tx)
                        } else {
                            None
                        }
                    }),
                self.hf,
                self.database.clone(),
            )
            .await?;
        }

        verify_transactions(
            self.prepped_txs,
            verification_needed,
            self.current_chain_height,
            self.top_hash,
            self.time_for_time_lock,
            self.hf,
            self.database,
            self.output_cache
        )
        .await
    }
}

/// Check that each key image used in each transaction is unique in the whole chain.
async fn check_kis_unique<D: Database>(
    txs: &[TransactionVerificationData],
    database: &mut D,
) -> Result<(), ExtendedConsensusError> {
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
        return Err(ConsensusError::Transaction(TransactionError::KeyImageSpent).into());
    }

    Ok(())
}

/// Returns a [`HashSet`] of all the hashes referenced in each transaction's [`CachedVerificationState`], that
/// are also in the main chain.
async fn hashes_referenced_in_main_chain<D: Database>(
    txs: &[TransactionVerificationData],
    database: &mut D,
) -> Result<HashSet<[u8; 32]>, ExtendedConsensusError> {
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

    Ok(verified_at_block_hashes)
}

/// Returns a list of [`VerificationNeeded`] for each transaction passed in. The returned
/// [`Vec`] will be the same length as the inputted transactions.
///
/// A [`bool`] is also returned, which will be true if any transactions need [`VerificationNeeded::V1DecoyCheck`].
fn verification_needed(
    txs: &[TransactionVerificationData],
    hashes_in_main_chain: &HashSet<[u8; 32]>,
    current_hf: HardFork,
    current_chain_height: usize,
    time_for_time_lock: u64,
) -> Result<(Vec<VerificationNeeded>, bool), ConsensusError> {
    // txs needing full validation: semantic and/or contextual
    let mut verification_needed = Vec::with_capacity(txs.len());

    let mut any_v1_decoy_checks = false;

    for tx in txs {
        match &tx.cached_verification_state {
            CachedVerificationState::NotVerified => {
                // Tx not verified at all need all checks.
                verification_needed.push(VerificationNeeded::SemanticAndContextual);
                continue;
            }
            CachedVerificationState::JustSemantic(hf) => {
                if current_hf != *hf {
                    // HF changed must do semantic checks again.
                    verification_needed.push(VerificationNeeded::SemanticAndContextual);
                    continue;
                }
                // Tx already semantically valid for this HF only contextual checks needed.
                verification_needed.push(VerificationNeeded::Contextual);
                continue;
            }
            CachedVerificationState::ValidAtHashAndHF { block_hash, hf } => {
                if current_hf != *hf {
                    // HF changed must do all checks again.
                    verification_needed.push(VerificationNeeded::SemanticAndContextual);
                    continue;
                }

                if !hashes_in_main_chain.contains(block_hash) {
                    // The block we know this transaction was valid at is no longer in the chain do
                    // contextual checks again.
                    verification_needed.push(VerificationNeeded::Contextual);
                    continue;
                }
            }
            CachedVerificationState::ValidAtHashAndHFWithTimeBasedLock {
                block_hash,
                hf,
                time_lock,
            } => {
                if current_hf != *hf {
                    // HF changed must do all checks again.
                    verification_needed.push(VerificationNeeded::SemanticAndContextual);
                    continue;
                }

                if !hashes_in_main_chain.contains(block_hash) {
                    // The block we know this transaction was valid at is no longer in the chain do
                    // contextual checks again.
                    verification_needed.push(VerificationNeeded::Contextual);
                    continue;
                }

                // If the time lock is still locked then the transaction is invalid.
                // Time is not monotonic in Monero so these can become invalid with new blocks.
                if !output_unlocked(time_lock, current_chain_height, time_for_time_lock, *hf) {
                    return Err(ConsensusError::Transaction(
                        TransactionError::OneOrMoreRingMembersLocked,
                    ));
                }
            }
        }

        if tx.version == TxVersion::RingSignatures {
            // v1 txs always need at least decoy checks as they can become invalid with new blocks.
            verification_needed.push(VerificationNeeded::V1DecoyCheck);
            any_v1_decoy_checks = true;
            continue;
        }

        verification_needed.push(VerificationNeeded::None);
    }

    Ok((verification_needed, any_v1_decoy_checks))
}

/// Do [`VerificationNeeded::V1DecoyCheck`] on each tx passed in.
async fn verify_transactions_decoy_info<D: Database>(
    txs: impl Iterator<Item = &TransactionVerificationData> + Clone,
    hf: HardFork,
    database: D,
) -> Result<(), ExtendedConsensusError> {
    // Decoy info is not validated for V1 txs.
    if hf == HardFork::V1 {
        return Ok(());
    }

    batch_get_decoy_info(txs, hf, database)
        .await?
        .try_for_each(|decoy_info| decoy_info.and_then(|di| Ok(check_decoy_info(&di, hf)?)))?;

    Ok(())
}

/// Do [`VerificationNeeded::Contextual`] or [`VerificationNeeded::SemanticAndContextual`].
///
/// The inputs to this function are the txs wanted to be verified and a list of [`VerificationNeeded`],
/// if any other [`VerificationNeeded`] is specified other than [`VerificationNeeded::Contextual`] or
/// [`VerificationNeeded::SemanticAndContextual`], nothing will be verified for that tx.
async fn verify_transactions<D>(
    mut txs: Vec<TransactionVerificationData>,
    verification_needed: Vec<VerificationNeeded>,
    current_chain_height: usize,
    top_hash: [u8; 32],
    current_time_lock_timestamp: u64,
    hf: HardFork,
    database: D,
    output_cache: Option<&OutputCache>
) -> Result<Vec<TransactionVerificationData>, ExtendedConsensusError>
where
    D: Database,
{
    /// A filter each tx not [`VerificationNeeded::Contextual`] or
    /// [`VerificationNeeded::SemanticAndContextual`]
    const fn tx_filter<T>((_, needed): &(T, &VerificationNeeded)) -> bool {
        matches!(
            needed,
            VerificationNeeded::Contextual | VerificationNeeded::SemanticAndContextual
        )
    }

    let txs_ring_member_info = batch_get_ring_member_info(
        txs.iter()
            .zip(verification_needed.iter())
            .filter(tx_filter)
            .map(|(tx, _)| tx),
        hf,
        database,
        output_cache
    )
    .await?;

    rayon_spawn_async(move || {
        let batch_verifier = MultiThreadedBatchVerifier::new(rayon::current_num_threads());

        txs.iter()
            .zip(verification_needed.iter())
            .filter(tx_filter)
            .zip(txs_ring_member_info.iter())
            .par_bridge()
            .try_for_each(|((tx, verification_needed), ring)| {
                // do semantic validation if needed.
                if *verification_needed == VerificationNeeded::SemanticAndContextual {
                    let fee = check_transaction_semantic(
                        &tx.tx,
                        tx.tx_blob.len(),
                        tx.tx_weight,
                        &tx.tx_hash,
                        hf,
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
                    hf,
                )?;

                Ok::<_, ConsensusError>(())
            })?;

        if !batch_verifier.verify() {
            return Err(ExtendedConsensusError::OneOrMoreBatchVerificationStatementsInvalid);
        }

        txs.iter_mut()
            .zip(verification_needed.iter())
            .filter(tx_filter)
            .zip(txs_ring_member_info)
            .for_each(|((tx, _), ring)| {
                tx.cached_verification_state = if ring.time_locked_outs.is_empty() {
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
                            Timelock::Time(time) => Some(time),
                            _ => None,
                        })
                        .min();

                    if let Some(time) = youngest_timebased_lock {
                        // time-based lock used.
                        CachedVerificationState::ValidAtHashAndHFWithTimeBasedLock {
                            block_hash: top_hash,
                            hf,
                            time_lock: Timelock::Time(*time),
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

        Ok(txs)
    })
    .await
}
