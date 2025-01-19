//! # Transaction Verifier Service.
//!
//! This module contains the [`TxVerifierService`] which handles consensus validation of transactions.
//!
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
    V1DecoyCheck,
    /// Both semantic validation and contextual validation are needed.
    SemanticAndContextual,
    /// Only contextual validation is needed.
    Contextual,
    None,
}

pub struct PrepTransactionsState {
    txs: Vec<Transaction>,
    prepped_txs: Vec<TransactionVerificationData>,
}

impl PrepTransactionsState {
    pub fn new() -> Self {
        Self {
            txs: vec![],
            prepped_txs: vec![],
        }
    }

    pub fn append_txs(mut self, mut txs: Vec<Transaction>) -> Self {
        self.txs.append(&mut txs);

        self
    }

    pub fn append_prepped_txs(mut self, mut txs: Vec<TransactionVerificationData>) -> Self {
        self.prepped_txs.append(&mut txs);

        self
    }

    pub fn prepare(mut self) -> Result<BlockchainDataState, ConsensusError> {
        if !self.txs.is_empty() {
            self.prepped_txs.append(
                &mut self
                    .txs
                    .into_par_iter()
                    .map(|tx| new_tx_verification_data(tx))
                    .collect::<Result<_, _>>()?,
            );
        }

        Ok(BlockchainDataState {
            prepped_txs: self.prepped_txs,
        })
    }
}

pub struct BlockchainDataState {
    prepped_txs: Vec<TransactionVerificationData>,
}

impl BlockchainDataState {
    pub fn just_semantic(self, hf: HardFork) -> SemanticVerificationState {
        SemanticVerificationState {
            prepped_txs: self.prepped_txs,
            hf,
        }
    }

    pub fn full<D: Database>(
        self,
        current_chain_height: usize,
        top_hash: [u8; 32],
        time_for_time_lock: u64,
        hf: HardFork,
        database: D,
    ) -> FullVerificationState<D> {
        FullVerificationState {
            prepped_txs: self.prepped_txs,
            current_chain_height,
            top_hash,
            time_for_time_lock,
            hf,
            database,
        }
    }
}

pub struct SemanticVerificationState {
    prepped_txs: Vec<TransactionVerificationData>,
    hf: HardFork,
}

impl SemanticVerificationState {
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

pub struct FullVerificationState<D> {
    prepped_txs: Vec<TransactionVerificationData>,

    current_chain_height: usize,
    top_hash: [u8; 32],
    time_for_time_lock: u64,
    hf: HardFork,
    database: D,
}

impl<D: Database + Clone> FullVerificationState<D> {
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
        )
        .await
    }
}

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

fn verification_needed(
    txs: &[TransactionVerificationData],
    hashes_in_main_chain: &HashSet<[u8; 32]>,
    current_hf: HardFork,
    current_chain_height: usize,
    time_for_time_lock: u64,
) -> Result<(Vec<VerificationNeeded>, bool), ConsensusError> {
    // txs needing full validation: semantic and/or contextual
    let mut verification_needed = Vec::new();

    let mut any_v1_decoy_checks = false;

    for tx in txs {
        match &tx.cached_verification_state {
            CachedVerificationState::NotVerified => {
                verification_needed.push(VerificationNeeded::SemanticAndContextual);
                continue;
            }
            CachedVerificationState::JustSemantic(hf) => {
                if current_hf != *hf {
                    verification_needed.push(VerificationNeeded::SemanticAndContextual);
                    continue;
                }

                verification_needed.push(VerificationNeeded::Contextual);
                continue;
            }
            CachedVerificationState::ValidAtHashAndHF { block_hash, hf } => {
                if current_hf != *hf {
                    verification_needed.push(VerificationNeeded::SemanticAndContextual);
                    continue;
                }

                if !hashes_in_main_chain.contains(block_hash) {
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
                    verification_needed.push(VerificationNeeded::SemanticAndContextual);
                    continue;
                }

                if !hashes_in_main_chain.contains(block_hash) {
                    verification_needed.push(VerificationNeeded::Contextual);
                    continue;
                }

                // If the time lock is still locked then the transaction is invalid.
                if !output_unlocked(time_lock, current_chain_height, time_for_time_lock, *hf) {
                    return Err(ConsensusError::Transaction(
                        TransactionError::OneOrMoreRingMembersLocked,
                    ));
                }
            }
        }

        if tx.version == TxVersion::RingSignatures {
            verification_needed.push(VerificationNeeded::V1DecoyCheck);
            any_v1_decoy_checks = true;
            continue;
        }

        verification_needed.push(VerificationNeeded::None)
    }

    Ok((verification_needed, any_v1_decoy_checks))
}

async fn verify_transactions_decoy_info<D>(
    txs: impl Iterator<Item = &TransactionVerificationData> + Clone,
    hf: HardFork,
    database: D,
) -> Result<(), ExtendedConsensusError>
where
    D: Database,
{
    // Decoy info is not validated for V1 txs.
    if hf == HardFork::V1 {
        return Ok(());
    }

    batch_get_decoy_info(txs, hf, database)
        .await?
        .try_for_each(|decoy_info| decoy_info.and_then(|di| Ok(check_decoy_info(&di, hf)?)))?;

    Ok(())
}

async fn verify_transactions<D>(
    mut txs: Vec<TransactionVerificationData>,
    verification_needed: Vec<VerificationNeeded>,
    current_chain_height: usize,
    top_hash: [u8; 32],
    current_time_lock_timestamp: u64,
    hf: HardFork,
    database: D,
) -> Result<Vec<TransactionVerificationData>, ExtendedConsensusError>
where
    D: Database,
{
    fn tx_filter<T>((_, needed): &(T, &VerificationNeeded)) -> bool {
        if matches!(
            needed,
            VerificationNeeded::Contextual | VerificationNeeded::SemanticAndContextual
        ) {
            true
        } else {
            false
        }
    }

    let txs_ring_member_info = batch_get_ring_member_info(
        txs.iter()
            .zip(verification_needed.iter())
            .filter(tx_filter)
            .map(|(tx, _)| tx),
        hf,
        database,
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
