use std::{
    collections::HashSet,
    future::Future,
    ops::Deref,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::FutureExt;
use monero_serai::transaction::Transaction;
use rayon::prelude::*;
use tower::{Service, ServiceExt};
use tracing::instrument;

use crate::{
    context::ReOrgToken, helper::rayon_spawn_async, ConsensusError, Database, DatabaseRequest,
    DatabaseResponse, HardFork,
};

mod contextual_data;
mod inputs;
pub(crate) mod outputs;
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
    rings_member_info: std::sync::Mutex<Option<contextual_data::TxRingMembersInfo>>,
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
        re_org_token: ReOrgToken,
    },
    /// Batches the setup of [`TransactionVerificationData`], does *minimal* verification, you need to call [`VerifyTxRequest::Block`]
    /// with the returned data.
    BatchSetup {
        txs: Vec<Transaction>,
        hf: HardFork,
        re_org_token: ReOrgToken,
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
                re_org_token,
            } => verify_transactions_for_block(
                database,
                txs,
                current_chain_height,
                time_for_time_lock,
                hf,
                re_org_token,
            )
            .boxed(),
            VerifyTxRequest::BatchSetup {
                txs,
                hf,
                re_org_token,
            } => batch_setup_transactions(database, txs, hf, re_org_token).boxed(),
        }
    }
}

async fn batch_setup_transactions<D>(
    database: D,
    txs: Vec<Transaction>,
    hf: HardFork,
    re_org_token: ReOrgToken,
) -> Result<VerifyTxResponse, ConsensusError>
where
    D: Database + Clone + Sync + Send + 'static,
{
    // Move out of the async runtime and use rayon to parallelize the serialisation and hashing of the txs.
    let txs = rayon_spawn_async(|| {
        txs.into_par_iter()
            .map(|tx| Ok(Arc::new(TransactionVerificationData::new(tx)?)))
            .collect::<Result<Vec<_>, ConsensusError>>()
    })
    .await?;

    contextual_data::batch_fill_ring_member_info(&txs, &hf, re_org_token, database).await?;

    Ok(VerifyTxResponse::BatchSetupOk(txs))
}

#[instrument(name = "verify_txs", skip_all, level = "info")]
async fn verify_transactions_for_block<D>(
    database: D,
    txs: Vec<Arc<TransactionVerificationData>>,
    current_chain_height: u64,
    time_for_time_lock: u64,
    hf: HardFork,
    re_org_token: ReOrgToken,
) -> Result<VerifyTxResponse, ConsensusError>
where
    D: Database + Clone + Sync + Send + 'static,
{
    tracing::debug!("Verifying transactions for block, amount: {}", txs.len());

    contextual_data::batch_refresh_ring_member_info(&txs, &hf, re_org_token, database.clone())
        .await?;

    let spent_kis = Arc::new(std::sync::Mutex::new(HashSet::new()));

    let cloned_spent_kis = spent_kis.clone();

    rayon_spawn_async(move || {
        txs.par_iter().try_for_each(|tx| {
            verify_transaction_for_block(
                tx,
                current_chain_height,
                time_for_time_lock,
                hf,
                cloned_spent_kis.clone(),
            )
        })
    })
    .await?;

    let DatabaseResponse::CheckKIsNotSpent(kis_spent) = database
        .oneshot(DatabaseRequest::CheckKIsNotSpent(
            Arc::into_inner(spent_kis).unwrap().into_inner().unwrap(),
        ))
        .await?
    else {
        panic!("Database sent incorrect response!");
    };

    if kis_spent {
        return Err(ConsensusError::TransactionHasInvalidInput(
            "One or more key image spent!",
        ));
    }

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
    decoy_info: &Option<contextual_data::DecoyInfo>,
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
