#![expect(
    unreachable_code,
    unused_variables,
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    reason = "TODO: finish implementing the signatures from <https://github.com/Cuprate/cuprate/pull/297>"
)]
use crate::error::TxPoolError;
use crate::txpool::TxpoolDatabase;
use crate::types::TransactionInfo;
use crate::{
    ops::{get_transaction_verification_data, in_stem_pool},
    service::interface::{TxpoolReadRequest, TxpoolReadResponse},
    types::{TransactionBlobHash, TransactionHash},
    TxEntry,
};
use cuprate_helper::asynch::InfallibleOneshotReceiver;
use fjall::Readable;
use futures::channel::oneshot;
use rayon::ThreadPool;
use std::task::{Context, Poll};
use std::{
    collections::{HashMap, HashSet},
    num::NonZero,
    sync::Arc,
};
use tower::Service;

#[derive(Clone)]
pub struct TxpoolReadHandle {
    pub(crate) pool: Arc<ThreadPool>,

    pub(crate) txpool: Arc<TxpoolDatabase>,
}

impl Service<TxpoolReadRequest> for TxpoolReadHandle {
    type Response = TxpoolReadResponse;
    type Error = TxPoolError;
    type Future = InfallibleOneshotReceiver<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: TxpoolReadRequest) -> Self::Future {
        let (tx, rx) = oneshot::channel();

        let db = self.txpool.clone();
        self.pool.spawn(move || {
            let res = map_request(&db, req);

            let _ = tx.send(res);
        });

        InfallibleOneshotReceiver::from(rx)
    }
}

//---------------------------------------------------------------------------------------------------- Request Mapping
// This function maps [`Request`]s to function calls
// executed by the rayon DB reader threadpool.

/// Map [`TxpoolReadRequest`]'s to specific database handler functions.
///
/// This is the main entrance into all `Request` handler functions.
/// The basic structure is:
/// 1. `Request` is mapped to a handler function
/// 2. Handler function is called
/// 3. [`TxpoolReadResponse`] is returned
fn map_request(
    db: &TxpoolDatabase,        // Access to the database
    request: TxpoolReadRequest, // The request we must fulfill
) -> Result<TxpoolReadResponse, TxPoolError> {
    match request {
        TxpoolReadRequest::TxBlob(tx_hash) => tx_blob(db, &tx_hash),
        TxpoolReadRequest::TxVerificationData(tx_hash) => tx_verification_data(db, &tx_hash),
        TxpoolReadRequest::FilterKnownTxBlobHashes(blob_hashes) => {
            filter_known_tx_blob_hashes(db, blob_hashes)
        }
        TxpoolReadRequest::TxsForBlock(txs_needed) => txs_for_block(db, txs_needed),
        TxpoolReadRequest::Backlog => backlog(db),
        TxpoolReadRequest::Size {
            include_sensitive_txs,
        } => size(db, include_sensitive_txs),
        TxpoolReadRequest::PoolInfo {
            include_sensitive_txs,
            max_tx_count,
            start_time,
        } => pool_info(db, include_sensitive_txs, max_tx_count, start_time),
        TxpoolReadRequest::TxsByHash {
            tx_hashes,
            include_sensitive_txs,
        } => txs_by_hash(db, tx_hashes, include_sensitive_txs),
        TxpoolReadRequest::KeyImagesSpent {
            key_images,
            include_sensitive_txs,
        } => key_images_spent(db, key_images, include_sensitive_txs),
        TxpoolReadRequest::KeyImagesSpentVec {
            key_images,
            include_sensitive_txs,
        } => key_images_spent_vec(db, key_images, include_sensitive_txs),
        TxpoolReadRequest::Pool {
            include_sensitive_txs,
        } => pool(db, include_sensitive_txs),
        TxpoolReadRequest::PoolStats {
            include_sensitive_txs,
        } => pool_stats(db, include_sensitive_txs),
        TxpoolReadRequest::AllHashes {
            include_sensitive_txs,
        } => all_hashes(db, include_sensitive_txs),
    }
}

//---------------------------------------------------------------------------------------------------- Handler functions
// These are the actual functions that do stuff according to the incoming [`TxpoolReadRequest`].
//
// Each function name is a 1-1 mapping (from CamelCase -> snake_case) to
// the enum variant name, e.g: `TxBlob` -> `tx_blob`.
//
// Each function will return the [`TxpoolReadResponse`] that we
// should send back to the caller in [`map_request()`].
//
// INVARIANT:
// These functions are called above in `tower::Service::call()`
// using a custom threadpool which means any call to `par_*()` functions
// will be using the custom rayon DB reader thread-pool, not the global one.
//
// All functions below assume that this is the case, such that
// `par_*()` functions will not block the _global_ rayon thread-pool.

/// [`TxpoolReadRequest::TxBlob`].
#[inline]
fn tx_blob(
    db: &TxpoolDatabase,
    tx_hash: &TransactionHash,
) -> Result<TxpoolReadResponse, TxPoolError> {
    let snapshot = db.fjall_database.snapshot();

    let tx_blob = snapshot
        .get(&db.tx_blobs, tx_hash)?
        .ok_or(TxPoolError::NotFound)?
        .to_vec();

    Ok(TxpoolReadResponse::TxBlob {
        tx_blob,
        state_stem: in_stem_pool(tx_hash, &snapshot, db)?,
    })
}

/// [`TxpoolReadRequest::TxVerificationData`].
#[inline]
fn tx_verification_data(
    db: &TxpoolDatabase,
    tx_hash: &TransactionHash,
) -> Result<TxpoolReadResponse, TxPoolError> {
    let snapshot = db.fjall_database.snapshot();

    get_transaction_verification_data(tx_hash, &snapshot, db)
        .map(TxpoolReadResponse::TxVerificationData)
}

/// [`TxpoolReadRequest::FilterKnownTxBlobHashes`].
fn filter_known_tx_blob_hashes(
    db: &TxpoolDatabase,
    mut blob_hashes: HashSet<TransactionBlobHash>,
) -> Result<TxpoolReadResponse, TxPoolError> {
    let snapshot = db.fjall_database.snapshot();

    let mut stem_pool_hashes = Vec::new();

    // A closure that returns `true` if a tx with a certain blob hash is unknown.
    // This also fills in `stem_tx_hashes`.
    let mut tx_unknown = |blob_hash| -> Result<bool, TxPoolError> {
        match snapshot.get(&db.known_blob_hashes, &blob_hash)? {
            Some(tx_hash) => {
                let tx_hash = tx_hash.as_ref().try_into().unwrap();

                if in_stem_pool(&tx_hash, &snapshot, db)? {
                    stem_pool_hashes.push(tx_hash);
                }
                Ok(false)
            }
            None => Ok(true),
        }
    };

    let mut err = None;
    blob_hashes.retain(|blob_hash| match tx_unknown(*blob_hash) {
        Ok(res) => res,
        Err(e) => {
            err = Some(e);
            false
        }
    });

    if let Some(e) = err {
        return Err(e);
    }

    Ok(TxpoolReadResponse::FilterKnownTxBlobHashes {
        unknown_blob_hashes: blob_hashes,
        stem_pool_hashes,
    })
}

/// [`TxpoolReadRequest::TxsForBlock`].
fn txs_for_block(
    db: &TxpoolDatabase,
    txs: Vec<TransactionHash>,
) -> Result<TxpoolReadResponse, TxPoolError> {
    let snapshot = db.fjall_database.snapshot();

    let mut missing_tx_indexes = Vec::with_capacity(txs.len());
    let mut txs_verification_data = HashMap::with_capacity(txs.len());

    for (i, tx_hash) in txs.into_iter().enumerate() {
        match get_transaction_verification_data(&tx_hash, &snapshot, db) {
            Ok(tx) => {
                txs_verification_data.insert(tx_hash, tx);
            }
            Err(TxPoolError::NotFound) => missing_tx_indexes.push(i),
            Err(e) => return Err(e),
        }
    }

    Ok(TxpoolReadResponse::TxsForBlock {
        txs: txs_verification_data,
        missing: missing_tx_indexes,
    })
}

/// [`TxpoolReadRequest::Backlog`].
#[inline]
fn backlog(db: &TxpoolDatabase) -> Result<TxpoolReadResponse, TxPoolError> {
    let snapshot = db.fjall_database.snapshot();

    let backlog = snapshot
        .iter(&db.tx_infos)
        .map(|info| {
            let (id, tx_info) = info.into_inner()?;

            let tx_info: TransactionInfo = bytemuck::pod_read_unaligned(tx_info.as_ref());

            Ok(TxEntry {
                id: id.as_ref().try_into().unwrap(),
                weight: tx_info.weight,
                fee: tx_info.fee,
                private: tx_info.flags.private(),
                received_at: tx_info.received_at,
            })
        })
        .collect::<Result<_, TxPoolError>>()?;

    Ok(TxpoolReadResponse::Backlog(backlog))
}

/// [`TxpoolReadRequest::Size`].
#[inline]
fn size(
    db: &TxpoolDatabase,
    include_sensitive_txs: bool,
) -> Result<TxpoolReadResponse, TxPoolError> {
    Ok(TxpoolReadResponse::Size(todo!()))
}

/// [`TxpoolReadRequest::PoolInfo`].
fn pool_info(
    db: &TxpoolDatabase,
    include_sensitive_txs: bool,
    max_tx_count: usize,
    start_time: Option<NonZero<usize>>,
) -> Result<TxpoolReadResponse, TxPoolError> {
    Ok(TxpoolReadResponse::PoolInfo(todo!()))
}

/// [`TxpoolReadRequest::TxsByHash`].
fn txs_by_hash(
    db: &TxpoolDatabase,
    tx_hashes: Vec<[u8; 32]>,
    include_sensitive_txs: bool,
) -> Result<TxpoolReadResponse, TxPoolError> {
    Ok(TxpoolReadResponse::TxsByHash(todo!()))
}

/// [`TxpoolReadRequest::KeyImagesSpent`].
fn key_images_spent(
    db: &TxpoolDatabase,
    key_images: HashSet<[u8; 32]>,
    include_sensitive_txs: bool,
) -> Result<TxpoolReadResponse, TxPoolError> {
    Ok(TxpoolReadResponse::KeyImagesSpent(todo!()))
}

/// [`TxpoolReadRequest::KeyImagesSpentVec`].
fn key_images_spent_vec(
    db: &TxpoolDatabase,
    key_images: Vec<[u8; 32]>,
    include_sensitive_txs: bool,
) -> Result<TxpoolReadResponse, TxPoolError> {
    Ok(TxpoolReadResponse::KeyImagesSpent(todo!()))
}

/// [`TxpoolReadRequest::Pool`].
fn pool(
    db: &TxpoolDatabase,
    include_sensitive_txs: bool,
) -> Result<TxpoolReadResponse, TxPoolError> {
    Ok(TxpoolReadResponse::Pool {
        txs: todo!(),
        spent_key_images: todo!(),
    })
}

/// [`TxpoolReadRequest::PoolStats`].
fn pool_stats(
    db: &TxpoolDatabase,
    include_sensitive_txs: bool,
) -> Result<TxpoolReadResponse, TxPoolError> {
    Ok(TxpoolReadResponse::PoolStats(todo!()))
}

/// [`TxpoolReadRequest::AllHashes`].
fn all_hashes(
    db: &TxpoolDatabase,
    include_sensitive_txs: bool,
) -> Result<TxpoolReadResponse, TxPoolError> {
    Ok(TxpoolReadResponse::AllHashes(todo!()))
}
