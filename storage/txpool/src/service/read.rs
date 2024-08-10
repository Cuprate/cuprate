use std::sync::Arc;

use rayon::ThreadPool;

use cuprate_database::{ConcreteEnv, DatabaseRo, Env, EnvInner};
use cuprate_database_service::{init_thread_pool, DatabaseReadService, ReaderThreads};

use crate::{
    ops::get_transaction_verification_data,
    service::{
        interface::{TxpoolReadRequest, TxpoolReadResponse},
        types::{ReadResponseResult, TxpoolReadHandle},
    },
    tables::{OpenTables, TransactionBlobs},
    types::TransactionHash,
};

// TODO: update the docs here
//---------------------------------------------------------------------------------------------------- init_read_service
/// Initialize the [`BCReadHandle`] thread-pool backed by `rayon`.
///
/// This spawns `threads` amount of reader threads
/// attached to `env` and returns a handle to the pool.
///
/// Should be called _once_ per actual database.
#[cold]
#[inline(never)] // Only called once.
pub fn init_read_service(env: Arc<ConcreteEnv>, threads: ReaderThreads) -> TxpoolReadHandle {
    init_read_service_with_pool(env, init_thread_pool(threads))
}

/// Initialize the blockchain database read service, with a specific rayon thread-pool instead of
/// creating a new one.
///
/// Should be called _once_ per actual database.
#[cold]
#[inline(never)] // Only called once.
pub fn init_read_service_with_pool(
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

    tx_blobs_table
        .get(tx_hash)
        .map(|blob| TxpoolReadResponse::TxBlob(blob.0))
}

/// [`TxpoolReadRequest::TxVerificationData`].
#[inline]
fn tx_verification_data(env: &ConcreteEnv, tx_hash: &TransactionHash) -> ReadResponseResult {
    let inner_env = env.env_inner();
    let tx_ro = inner_env.tx_ro()?;

    let mut tables = inner_env.open_tables(&tx_ro)?;

    get_transaction_verification_data(tx_hash, &mut tables)
        .map(TxpoolReadResponse::TxVerificationData)
}
