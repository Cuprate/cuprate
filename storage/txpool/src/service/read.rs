#![expect(
    unreachable_code,
    unused_variables,
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    reason = "TODO: finish implementing the signatures from <https://github.com/Cuprate/cuprate/pull/297>"
)]
use std::{
    collections::{HashMap, HashSet},
    num::NonZero,
    sync::Arc,
};

use rayon::ThreadPool;

use cuprate_database::{
    ConcreteEnv, DatabaseIter, DatabaseRo, DbResult, Env, EnvInner, InitError, RuntimeError,
};
use cuprate_database_service::{init_thread_pool, DatabaseReadService, ReaderThreads};

use crate::{
    ops::{get_transaction_verification_data, in_stem_pool},
    service::{
        interface::{TxpoolReadRequest, TxpoolReadResponse},
        types::{ReadResponseResult, TxpoolReadHandle},
    },
    tables::{KnownBlobHashes, OpenTables, TransactionBlobs, TransactionInfos},
    types::{TransactionBlobHash, TransactionHash},
    TxEntry,
};

// TODO: update the docs here
//---------------------------------------------------------------------------------------------------- init_read_service
/// Initialize the [`TxpoolReadHandle`] thread-pool backed by `rayon`.
///
/// This spawns `threads` amount of reader threads
/// attached to `env` and returns a handle to the pool.
///
/// Should be called _once_ per actual database.
#[cold]
#[inline(never)] // Only called once.
pub(super) fn init_read_service(
    env: Arc<ConcreteEnv>,
    threads: ReaderThreads,
) -> Result<TxpoolReadHandle, InitError> {
    Ok(init_read_service_with_pool(
        env,
        init_thread_pool(threads).map_err(|e| InitError::Unknown(Box::new(e)))?,
    ))
}

/// Initialize the [`TxpoolReadHandle`], with a specific rayon thread-pool instead of
/// creating a new one.
///
/// Should be called _once_ per actual database.
#[cold]
#[inline(never)] // Only called once.
pub(super) fn init_read_service_with_pool(
    env: Arc<ConcreteEnv>,
    pool: Arc<ThreadPool>,
) -> TxpoolReadHandle {
    DatabaseReadService::new(env, pool, map_request)
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
    env: &ConcreteEnv,          // Access to the database
    request: TxpoolReadRequest, // The request we must fulfill
) -> ReadResponseResult {
    match request {
        TxpoolReadRequest::TxBlob(tx_hash) => tx_blob(env, &tx_hash),
        TxpoolReadRequest::TxVerificationData(tx_hash) => tx_verification_data(env, &tx_hash),
        TxpoolReadRequest::FilterKnownTxBlobHashes(blob_hashes) => {
            filter_known_tx_blob_hashes(env, blob_hashes)
        }
        TxpoolReadRequest::TxsForBlock(txs_needed) => txs_for_block(env, txs_needed),
        TxpoolReadRequest::Backlog => backlog(env),
        TxpoolReadRequest::Size {
            include_sensitive_txs,
        } => size(env, include_sensitive_txs),
        TxpoolReadRequest::PoolInfo {
            include_sensitive_txs,
            max_tx_count,
            start_time,
        } => pool_info(env, include_sensitive_txs, max_tx_count, start_time),
        TxpoolReadRequest::TxsByHash {
            tx_hashes,
            include_sensitive_txs,
        } => txs_by_hash(env, tx_hashes, include_sensitive_txs),
        TxpoolReadRequest::KeyImagesSpent {
            key_images,
            include_sensitive_txs,
        } => key_images_spent(env, key_images, include_sensitive_txs),
        TxpoolReadRequest::KeyImagesSpentVec {
            key_images,
            include_sensitive_txs,
        } => key_images_spent_vec(env, key_images, include_sensitive_txs),
        TxpoolReadRequest::Pool {
            include_sensitive_txs,
        } => pool(env, include_sensitive_txs),
        TxpoolReadRequest::PoolStats {
            include_sensitive_txs,
        } => pool_stats(env, include_sensitive_txs),
        TxpoolReadRequest::AllHashes {
            include_sensitive_txs,
        } => all_hashes(env, include_sensitive_txs),
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
fn tx_blob(env: &ConcreteEnv, tx_hash: &TransactionHash) -> ReadResponseResult {
    let inner_env = env.env_inner();
    let tx_ro = inner_env.tx_ro()?;

    let tx_blobs_table = inner_env.open_db_ro::<TransactionBlobs>(&tx_ro)?;
    let tx_infos_table = inner_env.open_db_ro::<TransactionInfos>(&tx_ro)?;

    let tx_blob = tx_blobs_table.get(tx_hash)?.0;

    Ok(TxpoolReadResponse::TxBlob {
        tx_blob,
        state_stem: in_stem_pool(tx_hash, &tx_infos_table)?,
    })
}

/// [`TxpoolReadRequest::TxVerificationData`].
#[inline]
fn tx_verification_data(env: &ConcreteEnv, tx_hash: &TransactionHash) -> ReadResponseResult {
    let inner_env = env.env_inner();
    let tx_ro = inner_env.tx_ro()?;

    let tables = inner_env.open_tables(&tx_ro)?;

    get_transaction_verification_data(tx_hash, &tables).map(TxpoolReadResponse::TxVerificationData)
}

/// [`TxpoolReadRequest::FilterKnownTxBlobHashes`].
fn filter_known_tx_blob_hashes(
    env: &ConcreteEnv,
    mut blob_hashes: HashSet<TransactionBlobHash>,
) -> ReadResponseResult {
    let inner_env = env.env_inner();
    let tx_ro = inner_env.tx_ro()?;

    let tx_blob_hashes = inner_env.open_db_ro::<KnownBlobHashes>(&tx_ro)?;
    let tx_infos = inner_env.open_db_ro::<TransactionInfos>(&tx_ro)?;

    let mut stem_pool_hashes = Vec::new();

    // A closure that returns `true` if a tx with a certain blob hash is unknown.
    // This also fills in `stem_tx_hashes`.
    let mut tx_unknown = |blob_hash| -> DbResult<bool> {
        match tx_blob_hashes.get(&blob_hash) {
            Ok(tx_hash) => {
                if in_stem_pool(&tx_hash, &tx_infos)? {
                    stem_pool_hashes.push(tx_hash);
                }
                Ok(false)
            }
            Err(RuntimeError::KeyNotFound) => Ok(true),
            Err(e) => Err(e),
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
fn txs_for_block(env: &ConcreteEnv, txs: Vec<TransactionHash>) -> ReadResponseResult {
    let inner_env = env.env_inner();
    let tx_ro = inner_env.tx_ro()?;

    let tables = inner_env.open_tables(&tx_ro)?;

    let mut missing_tx_indexes = Vec::with_capacity(txs.len());
    let mut txs_verification_data = HashMap::with_capacity(txs.len());

    for (i, tx_hash) in txs.into_iter().enumerate() {
        match get_transaction_verification_data(&tx_hash, &tables) {
            Ok(tx) => {
                txs_verification_data.insert(tx_hash, tx);
            }
            Err(RuntimeError::KeyNotFound) => missing_tx_indexes.push(i),
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
fn backlog(env: &ConcreteEnv) -> ReadResponseResult {
    let inner_env = env.env_inner();
    let tx_ro = inner_env.tx_ro()?;

    let tx_infos_table = inner_env.open_db_ro::<TransactionInfos>(&tx_ro)?;

    let backlog = tx_infos_table
        .iter()?
        .map(|info| {
            let (id, info) = info?;

            Ok(TxEntry {
                id,
                weight: info.weight,
                fee: info.fee,
                private: info.flags.private(),
                received_at: info.received_at,
            })
        })
        .collect::<Result<_, RuntimeError>>()?;

    Ok(TxpoolReadResponse::Backlog(backlog))
}

/// [`TxpoolReadRequest::Size`].
#[inline]
fn size(env: &ConcreteEnv, include_sensitive_txs: bool) -> ReadResponseResult {
    Ok(TxpoolReadResponse::Size(todo!()))
}

/// [`TxpoolReadRequest::PoolInfo`].
fn pool_info(
    env: &ConcreteEnv,
    include_sensitive_txs: bool,
    max_tx_count: usize,
    start_time: Option<NonZero<usize>>,
) -> ReadResponseResult {
    Ok(TxpoolReadResponse::PoolInfo(todo!()))
}

/// [`TxpoolReadRequest::TxsByHash`].
fn txs_by_hash(
    env: &ConcreteEnv,
    tx_hashes: Vec<[u8; 32]>,
    include_sensitive_txs: bool,
) -> ReadResponseResult {
    Ok(TxpoolReadResponse::TxsByHash(todo!()))
}

/// [`TxpoolReadRequest::KeyImagesSpent`].
fn key_images_spent(
    env: &ConcreteEnv,
    key_images: HashSet<[u8; 32]>,
    include_sensitive_txs: bool,
) -> ReadResponseResult {
    Ok(TxpoolReadResponse::KeyImagesSpent(todo!()))
}

/// [`TxpoolReadRequest::KeyImagesSpentVec`].
fn key_images_spent_vec(
    env: &ConcreteEnv,
    key_images: Vec<[u8; 32]>,
    include_sensitive_txs: bool,
) -> ReadResponseResult {
    Ok(TxpoolReadResponse::KeyImagesSpent(todo!()))
}

/// [`TxpoolReadRequest::Pool`].
fn pool(env: &ConcreteEnv, include_sensitive_txs: bool) -> ReadResponseResult {
    Ok(TxpoolReadResponse::Pool {
        txs: todo!(),
        spent_key_images: todo!(),
    })
}

/// [`TxpoolReadRequest::PoolStats`].
fn pool_stats(env: &ConcreteEnv, include_sensitive_txs: bool) -> ReadResponseResult {
    Ok(TxpoolReadResponse::PoolStats(todo!()))
}

/// [`TxpoolReadRequest::AllHashes`].
fn all_hashes(env: &ConcreteEnv, include_sensitive_txs: bool) -> ReadResponseResult {
    Ok(TxpoolReadResponse::AllHashes(todo!()))
}
