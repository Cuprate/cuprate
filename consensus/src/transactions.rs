use std::ops::Deref;
use std::{
    collections::HashSet,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::FutureExt;
use monero_serai::transaction::Transaction;
use rayon::prelude::*;
use tower::Service;
use tracing::instrument;

use crate::{ConsensusError, Database, HardFork};

mod inputs;
pub(crate) mod outputs;
mod ring;
mod sigs;
mod time_lock;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum TxVersion {
    RingSignatures,
    RingCT,
}

impl TxVersion {
    pub fn from_raw(version: u64) -> Result<TxVersion, ConsensusError> {
        match version {
            1 => Ok(TxVersion::RingSignatures),
            2 => Ok(TxVersion::RingCT),
            _ => Err(ConsensusError::TransactionVersionInvalid),
        }
    }
}

/// Data needed to verify a transaction.
///
#[derive(Debug)]
pub struct TransactionVerificationData {
    pub tx: Transaction,
    pub version: TxVersion,
    pub tx_blob: Vec<u8>,
    pub tx_weight: usize,
    pub fee: u64,
    pub tx_hash: [u8; 32],
    /// We put this behind a mutex as the information is not constant and is based of past outputs idxs
    /// which could change on re-orgs.
    rings_member_info: std::sync::Mutex<Option<ring::TxRingMembersInfo>>,
}

impl TransactionVerificationData {
    pub fn new(tx: Transaction) -> Result<TransactionVerificationData, ConsensusError> {
        Ok(TransactionVerificationData {
            tx_hash: tx.hash(),
            tx_blob: tx.serialize(),
            tx_weight: tx.weight(),
            fee: tx.rct_signatures.base.fee,
            rings_member_info: std::sync::Mutex::new(None),
            version: TxVersion::from_raw(tx.prefix.version)?,
            tx,
        })
    }
}

pub enum VerifyTxRequest {
    /// Verifies transactions in the context of a block.
    Block {
        txs: Vec<Arc<TransactionVerificationData>>,
        current_chain_height: u64,
        time_for_time_lock: u64,
        hf: HardFork,
    },
    /// Batches the setup of [`TransactionVerificationData`].
    BatchSetup { txs: Vec<Transaction>, hf: HardFork },
    /// Batches the setup of [`TransactionVerificationData`] and verifies the transactions
    /// in the context of a block.
    BatchSetupVerifyBlock {
        txs: Vec<Transaction>,
        current_chain_height: u64,
        time_for_time_lock: u64,
        hf: HardFork,
    },
}

pub enum VerifyTxResponse {
    BatchSetupOk(Vec<Arc<TransactionVerificationData>>),
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
    type Error = ConsensusError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.database.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: VerifyTxRequest) -> Self::Future {
        let database = self.database.clone();

        match req {
            VerifyTxRequest::Block {
                txs,
                current_chain_height,
                time_for_time_lock,
                hf,
            } => verify_transactions_for_block(
                database,
                txs,
                current_chain_height,
                time_for_time_lock,
                hf,
            )
            .boxed(),
            VerifyTxRequest::BatchSetup { txs, hf } => {
                batch_setup_transactions(database, txs, hf).boxed()
            }
            VerifyTxRequest::BatchSetupVerifyBlock {
                txs,
                current_chain_height,
                time_for_time_lock,
                hf,
            } => batch_setup_verify_transactions_for_block(
                database,
                txs,
                current_chain_height,
                time_for_time_lock,
                hf,
            )
            .boxed(),
        }
    }
}

async fn set_missing_ring_members<D>(
    database: D,
    txs: &[Arc<TransactionVerificationData>],
    hf: &HardFork,
) -> Result<(), ConsensusError>
where
    D: Database + Clone + Sync + Send + 'static,
{
    // TODO: handle re-orgs.

    let txs_needing_ring_members = txs
        .iter()
        // Safety: we must not hold the mutex lock for long to not block the async runtime.
        .filter(|tx| tx.rings_member_info.lock().unwrap().is_none())
        .cloned()
        .collect::<Vec<_>>();

    tracing::debug!(
        "Retrieving ring members for {} txs",
        txs_needing_ring_members.len()
    );

    ring::batch_fill_ring_member_info(&txs_needing_ring_members, hf, database).await?;

    Ok(())
}

async fn batch_setup_transactions<D>(
    database: D,
    txs: Vec<Transaction>,
    hf: HardFork,
) -> Result<VerifyTxResponse, ConsensusError>
where
    D: Database + Clone + Sync + Send + 'static,
{
    // Move out of the async runtime and use rayon to parallelize the serialisation and hashing of the txs.
    let txs = tokio::task::spawn_blocking(|| {
        txs.into_par_iter()
            .map(|tx| Ok(Arc::new(TransactionVerificationData::new(tx)?)))
            .collect::<Result<Vec<_>, ConsensusError>>()
    })
    .await
    .unwrap()?;

    set_missing_ring_members(database, &txs, &hf).await?;

    Ok(VerifyTxResponse::BatchSetupOk(txs))
}

async fn batch_setup_verify_transactions_for_block<D>(
    database: D,
    txs: Vec<Transaction>,
    current_chain_height: u64,
    time_for_time_lock: u64,
    hf: HardFork,
) -> Result<VerifyTxResponse, ConsensusError>
where
    D: Database + Clone + Sync + Send + 'static,
{
    // Move out of the async runtime and use rayon to parallelize the serialisation and hashing of the txs.
    let txs = tokio::task::spawn_blocking(|| {
        txs.into_par_iter()
            .map(|tx| Ok(Arc::new(TransactionVerificationData::new(tx)?)))
            .collect::<Result<Vec<_>, ConsensusError>>()
    })
    .await
    .unwrap()?;

    verify_transactions_for_block(
        database,
        txs.clone(),
        current_chain_height,
        time_for_time_lock,
        hf,
    )
    .await?;
    Ok(VerifyTxResponse::BatchSetupOk(txs))
}

#[instrument(name = "verify_txs", skip_all, level = "info")]
async fn verify_transactions_for_block<D>(
    database: D,
    txs: Vec<Arc<TransactionVerificationData>>,
    current_chain_height: u64,
    time_for_time_lock: u64,
    hf: HardFork,
) -> Result<VerifyTxResponse, ConsensusError>
where
    D: Database + Clone + Sync + Send + 'static,
{
    tracing::debug!("Verifying transactions for block, amount: {}", txs.len());

    set_missing_ring_members(database, &txs, &hf).await?;

    let spent_kis = Arc::new(std::sync::Mutex::new(HashSet::new()));

    tokio::task::spawn_blocking(move || {
        txs.par_iter().try_for_each(|tx| {
            verify_transaction_for_block(
                tx,
                current_chain_height,
                time_for_time_lock,
                hf,
                spent_kis.clone(),
            )
        })
    })
    .await
    .unwrap()?;

    Ok(VerifyTxResponse::Ok)
}

fn verify_transaction_for_block(
    tx_verification_data: &TransactionVerificationData,
    current_chain_height: u64,
    time_for_time_lock: u64,
    hf: HardFork,
    spent_kis: Arc<std::sync::Mutex<HashSet<[u8; 32]>>>,
) -> Result<(), ConsensusError> {
    tracing::debug!(
        "Verifying transaction: {}",
        hex::encode(tx_verification_data.tx_hash)
    );

    let tx_version = &tx_verification_data.version;

    let rings_member_info_lock = tx_verification_data.rings_member_info.lock().unwrap();
    let rings_member_info = match rings_member_info_lock.deref() {
        Some(rings_member_info) => rings_member_info,
        None => panic!("rings_member_info needs to be set to be able to verify!"),
    };

    check_tx_version(&rings_member_info.decoy_info, tx_version, &hf)?;

    time_lock::check_all_time_locks(
        &rings_member_info.time_locked_outs,
        current_chain_height,
        time_for_time_lock,
        &hf,
    )?;

    let sum_outputs =
        outputs::check_outputs(&tx_verification_data.tx.prefix.outputs, &hf, tx_version)?;

    let sum_inputs = inputs::check_inputs(
        &tx_verification_data.tx.prefix.inputs,
        rings_member_info,
        current_chain_height,
        &hf,
        tx_version,
        spent_kis,
    )?;

    if tx_version == &TxVersion::RingSignatures {
        if sum_outputs >= sum_inputs {
            return Err(ConsensusError::TransactionOutputsTooMuch);
        }
        // check that monero-serai is calculating the correct value here, why can't we just use this
        // value? because we don't have this when we create the object.
        assert_eq!(tx_verification_data.fee, sum_inputs - sum_outputs);
    }

    sigs::verify_signatures(&tx_verification_data.tx, &rings_member_info.rings)?;

    Ok(())
}

/// Checks the version is in the allowed range.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#version
fn check_tx_version(
    decoy_info: &Option<ring::DecoyInfo>,
    version: &TxVersion,
    hf: &HardFork,
) -> Result<(), ConsensusError> {
    if let Some(decoy_info) = decoy_info {
        let max = max_tx_version(hf);
        if version > &max {
            return Err(ConsensusError::TransactionVersionInvalid);
        }

        // TODO: Doc is wrong here
        let min = min_tx_version(hf);
        if version < &min && decoy_info.not_mixable != 0 {
            return Err(ConsensusError::TransactionVersionInvalid);
        }
    } else {
        // This will only happen for hard-fork 1 when only RingSignatures are allowed.
        if version != &TxVersion::RingSignatures {
            return Err(ConsensusError::TransactionVersionInvalid);
        }
    }

    Ok(())
}

fn max_tx_version(hf: &HardFork) -> TxVersion {
    if hf <= &HardFork::V3 {
        TxVersion::RingSignatures
    } else {
        TxVersion::RingCT
    }
}

fn min_tx_version(hf: &HardFork) -> TxVersion {
    if hf >= &HardFork::V6 {
        TxVersion::RingCT
    } else {
        TxVersion::RingSignatures
    }
}
