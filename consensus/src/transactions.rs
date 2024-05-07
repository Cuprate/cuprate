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
use tower::{Service, ServiceExt};
use tracing::instrument;

use cuprate_consensus_rules::{
    transactions::{
        check_transaction_contextual, check_transaction_semantic, RingCTError, TransactionError,
        TxRingMembersInfo,
    },
    ConsensusError, HardFork, TxVersion,
};
use cuprate_helper::asynch::rayon_spawn_async;

use crate::{
    batch_verifier::MultiThreadedBatchVerifier, context::ReOrgToken, Database, DatabaseRequest,
    DatabaseResponse, ExtendedConsensusError,
};

pub mod contextual_data;

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
    /// Represents if a time-based time-lock was used with this transaction, if so the tx could become invalid
    /// as the time lock could become locked again as time-based locks are _not_ monotonic.
    youngest_time_based_time_lock_used: std::sync::Mutex<Option<Timelock>>,
    /// The hash the tx was valid at, if this is still in the chain, _most_ verification can be skipped.
    verified_at_hash: std::sync::Mutex<Option<[u8; 32]>>,
}

impl TransactionVerificationData {
    pub fn new(
        tx: Transaction,
        hf: &HardFork,
        verifier: Arc<MultiThreadedBatchVerifier>,
    ) -> Result<TransactionVerificationData, ConsensusError> {
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

        let fee = verifier.queue_statement(|verifier| {
            check_transaction_semantic(&tx, tx_blob.len(), tx_weight, &tx_hash, hf, verifier)
                .map_err(ConsensusError::Transaction)
        })?;

        Ok(TransactionVerificationData {
            tx_hash,
            tx_blob,
            tx_weight,
            fee,
            verified_at_hash: std::sync::Mutex::new(None),
            // set this as true for now, this will be changed when ring members are got
            youngest_time_based_time_lock_used: std::sync::Mutex::new(None),
            version: TxVersion::from_raw(tx.prefix.version)
                .ok_or(TransactionError::TransactionVersionInvalid)?,
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
                VerifyTxRequest::Block {
                    txs,
                    current_chain_height,
                    time_for_time_lock,
                    hf,
                    re_org_token,
                } => {
                    verify_transactions_for_block(
                        database,
                        txs,
                        current_chain_height,
                        time_for_time_lock,
                        hf,
                        re_org_token,
                    )
                    .await
                }
            }
        }
        .boxed()
    }
}

#[instrument(name = "verify_txs", skip_all, level = "info")]
async fn verify_transactions_for_block<D>(
    database: D,
    txs: Vec<Arc<TransactionVerificationData>>,
    current_chain_height: u64,
    time_for_time_lock: u64,
    hf: HardFork,
    re_org_token: ReOrgToken,
) -> Result<VerifyTxResponse, ExtendedConsensusError>
where
    D: Database + Clone + Sync + Send + 'static,
{
    tracing::debug!("Verifying transactions for block, amount: {}", txs.len());

    contextual_data::batch_refresh_ring_member_info(
        &txs,
        &hf,
        re_org_token,
        database.clone(),
        current_chain_height,
    )
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

    let rings_member_info_lock = tx_verification_data.rings_member_info.lock().unwrap();
    let rings_member_info = match rings_member_info_lock.deref() {
        Some(rings_member_info) => rings_member_info,
        None => panic!("rings_member_info needs to be set to be able to verify!"),
    };

    check_transaction_contextual(
        &tx_verification_data.tx,
        &rings_member_info.0,
        current_chain_height,
        time_for_time_lock,
        &hf,
        spent_kis,
    )?;

    Ok(())
}
