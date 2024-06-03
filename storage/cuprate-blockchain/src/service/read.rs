//! Database reader thread-pool definitions and logic.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    task::{Context, Poll},
};

use futures::{channel::oneshot, ready};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use thread_local::ThreadLocal;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::PollSemaphore;

use cuprate_helper::asynch::InfallibleOneshotReceiver;
use cuprate_types::{
    blockchain::{BCReadRequest, BCResponse},
    ExtendedBlockHeader, OutputOnChain,
};

use crate::{
    config::ReaderThreads,
    error::RuntimeError,
    ops::block::block_exists,
    ops::{
        block::{get_block_extended_header_from_height, get_block_info},
        blockchain::{cumulative_generated_coins, top_block_height},
        key_image::key_image_exists,
        output::id_to_output_on_chain,
    },
    service::types::{ResponseReceiver, ResponseResult, ResponseSender},
    tables::{BlockHeights, BlockInfos, Tables},
    types::BlockHash,
    types::{Amount, AmountIndex, BlockHeight, KeyImage, PreRctOutputId},
    ConcreteEnv, DatabaseRo, Env, EnvInner,
};

//---------------------------------------------------------------------------------------------------- DatabaseReadHandle
/// Read handle to the database.
///
/// This is cheaply [`Clone`]able handle that
/// allows `async`hronously reading from the database.
///
/// Calling [`tower::Service::call`] with a [`DatabaseReadHandle`] & [`BCReadRequest`]
/// will return an `async`hronous channel that can be `.await`ed upon
/// to receive the corresponding [`BCResponse`].
pub struct DatabaseReadHandle {
    /// Handle to the custom `rayon` DB reader thread-pool.
    ///
    /// Requests are [`rayon::ThreadPool::spawn`]ed in this thread-pool,
    /// and responses are returned via a channel we (the caller) provide.
    pool: Arc<rayon::ThreadPool>,

    /// Counting semaphore asynchronous permit for database access.
    /// Each [`tower::Service::poll_ready`] will acquire a permit
    /// before actually sending a request to the `rayon` DB threadpool.
    semaphore: PollSemaphore,

    /// An owned permit.
    /// This will be set to [`Some`] in `poll_ready()` when we successfully acquire
    /// the permit, and will be [`Option::take()`]n after `tower::Service::call()` is called.
    ///
    /// The actual permit will be dropped _after_ the rayon DB thread has finished
    /// the request, i.e., after [`map_request()`] finishes.
    permit: Option<OwnedSemaphorePermit>,

    /// Access to the database.
    env: Arc<ConcreteEnv>,
}

// `OwnedSemaphorePermit` does not implement `Clone`,
// so manually clone all elements, while keeping `permit`
// `None` across clones.
impl Clone for DatabaseReadHandle {
    fn clone(&self) -> Self {
        Self {
            pool: Arc::clone(&self.pool),
            semaphore: self.semaphore.clone(),
            permit: None,
            env: Arc::clone(&self.env),
        }
    }
}

impl DatabaseReadHandle {
    /// Initialize the `DatabaseReader` thread-pool backed by `rayon`.
    ///
    /// This spawns `N` amount of `DatabaseReader`'s
    /// attached to `env` and returns a handle to the pool.
    ///
    /// Should be called _once_ per actual database.
    #[cold]
    #[inline(never)] // Only called once.
    pub(super) fn init(env: &Arc<ConcreteEnv>, reader_threads: ReaderThreads) -> Self {
        // How many reader threads to spawn?
        let reader_count = reader_threads.as_threads().get();

        // Spawn `rayon` reader threadpool.
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(reader_count)
            .thread_name(|i| format!("cuprate_helper::service::read::DatabaseReader{i}"))
            .build()
            .unwrap();

        // Create a semaphore with the same amount of
        // permits as the amount of reader threads.
        let semaphore = PollSemaphore::new(Arc::new(Semaphore::new(reader_count)));

        // Return a handle to the pool.
        Self {
            pool: Arc::new(pool),
            semaphore,
            permit: None,
            env: Arc::clone(env),
        }
    }

    /// Access to the actual database environment.
    ///
    /// # ⚠️ Warning
    /// This function gives you access to the actual
    /// underlying database connected to by `self`.
    ///
    /// I.e. it allows you to read/write data _directly_
    /// instead of going through a request.
    ///
    /// Be warned that using the database directly
    /// in this manner has not been tested.
    #[inline]
    pub const fn env(&self) -> &Arc<ConcreteEnv> {
        &self.env
    }
}

impl tower::Service<BCReadRequest> for DatabaseReadHandle {
    type Response = BCResponse;
    type Error = RuntimeError;
    type Future = ResponseReceiver;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Check if we already have a permit.
        if self.permit.is_some() {
            return Poll::Ready(Ok(()));
        }

        // Acquire a permit before returning `Ready`.
        let permit =
            ready!(self.semaphore.poll_acquire(cx)).expect("this semaphore is never closed");

        self.permit = Some(permit);
        Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, request: BCReadRequest) -> Self::Future {
        let permit = self
            .permit
            .take()
            .expect("poll_ready() should have acquire a permit before calling call()");

        // Response channel we `.await` on.
        let (response_sender, receiver) = oneshot::channel();

        // Spawn the request in the rayon DB thread-pool.
        //
        // Note that this uses `self.pool` instead of `rayon::spawn`
        // such that any `rayon` parallel code that runs within
        // the passed closure uses the same `rayon` threadpool.
        //
        // INVARIANT:
        // The below `DatabaseReader` function impl block relies on this behavior.
        let env = Arc::clone(&self.env);
        self.pool.spawn(move || {
            let _permit: OwnedSemaphorePermit = permit;
            map_request(&env, request, response_sender);
        }); // drop(permit/env);

        InfallibleOneshotReceiver::from(receiver)
    }
}

//---------------------------------------------------------------------------------------------------- Request Mapping
// This function maps [`Request`]s to function calls
// executed by the rayon DB reader threadpool.

/// Map [`Request`]'s to specific database handler functions.
///
/// This is the main entrance into all `Request` handler functions.
/// The basic structure is:
/// 1. `Request` is mapped to a handler function
/// 2. Handler function is called
/// 3. [`BCResponse`] is sent
fn map_request(
    env: &ConcreteEnv,               // Access to the database
    request: BCReadRequest,          // The request we must fulfill
    response_sender: ResponseSender, // The channel we must send the response back to
) {
    use BCReadRequest as R;

    /* SOMEDAY: pre-request handling, run some code for each request? */

    let response = match request {
        R::BlockExtendedHeader(block) => block_extended_header(env, block),
        R::BlockHash(block) => block_hash(env, block),
        R::FilterUnknownHashes(hashes) => filter_unknown_hahses(env, hashes),
        R::BlockExtendedHeaderInRange(range) => block_extended_header_in_range(env, range),
        R::ChainHeight => chain_height(env),
        R::GeneratedCoins => generated_coins(env),
        R::Outputs(map) => outputs(env, map),
        R::NumberOutputsWithAmount(vec) => number_outputs_with_amount(env, vec),
        R::KeyImagesSpent(set) => key_images_spent(env, set),
    };

    if let Err(e) = response_sender.send(response) {
        // TODO: use tracing.
        println!("database reader failed to send response: {e:?}");
    }

    /* SOMEDAY: post-request handling, run some code for each request? */
}

//---------------------------------------------------------------------------------------------------- Thread Local
/// Q: Why does this exist?
///
/// A1: `heed`'s transactions and tables are not `Sync`, so we cannot use
/// them with rayon, however, we set a feature such that they are `Send`.
///
/// A2: When sending to rayon, we want to ensure each read transaction
/// is only being used by 1 thread only to scale reads
///
/// <https://github.com/Cuprate/cuprate/pull/113#discussion_r1576762346>
#[inline]
fn thread_local<T: Send>(env: &impl Env) -> ThreadLocal<T> {
    ThreadLocal::with_capacity(env.config().reader_threads.as_threads().get())
}

/// Take in a `ThreadLocal<impl Tables>` and return an `&impl Tables + Send`.
///
/// # Safety
/// See [`DatabaseRo`] docs.
///
/// We are safely using `UnsafeSendable` in `service`'s reader thread-pool
/// as we are pairing our usage with `ThreadLocal` - only 1 thread
/// will ever access a transaction at a time. This is an INVARIANT.
///
/// A `Mutex` was considered but:
/// - It is less performant
/// - It isn't technically needed for safety in our use-case
/// - It causes `DatabaseIter` function return issues as there is a `MutexGuard` object
///
/// <https://github.com/Cuprate/cuprate/pull/113#discussion_r1581684698>
///
/// # Notes
/// This is used for other backends as well instead of branching with `cfg_if`.
/// The other backends (as of current) are `Send + Sync` so this is fine.
/// <https://github.com/Cuprate/cuprate/pull/113#discussion_r1585618374>
macro_rules! get_tables {
    ($env_inner:ident, $tx_ro:ident, $tables:ident) => {{
        $tables.get_or_try(|| {
            #[allow(clippy::significant_drop_in_scrutinee)]
            match $env_inner.open_tables($tx_ro) {
                // SAFETY: see above macro doc comment.
                Ok(tables) => Ok(unsafe { crate::unsafe_sendable::UnsafeSendable::new(tables) }),
                Err(e) => Err(e),
            }
        })
    }};
}

//---------------------------------------------------------------------------------------------------- Handler functions
// These are the actual functions that do stuff according to the incoming [`Request`].
//
// Each function name is a 1-1 mapping (from CamelCase -> snake_case) to
// the enum variant name, e.g: `BlockExtendedHeader` -> `block_extended_header`.
//
// Each function will return the [`Response`] that we
// should send back to the caller in [`map_request()`].
//
// INVARIANT:
// These functions are called above in `tower::Service::call()`
// using a custom threadpool which means any call to `par_*()` functions
// will be using the custom rayon DB reader thread-pool, not the global one.
//
// All functions below assume that this is the case, such that
// `par_*()` functions will not block the _global_ rayon thread-pool.

// FIXME: implement multi-transaction read atomicity.
// <https://github.com/Cuprate/cuprate/pull/113#discussion_r1576874589>.

// TODO: The overhead of parallelism may be too much for every request, perfomace test to find optimal
// amount of parallelism.

/// [`BCReadRequest::BlockExtendedHeader`].
#[inline]
fn block_extended_header(env: &ConcreteEnv, block_height: BlockHeight) -> ResponseResult {
    // Single-threaded, no `ThreadLocal` required.
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;
    let tables = env_inner.open_tables(&tx_ro)?;

    Ok(BCResponse::BlockExtendedHeader(
        get_block_extended_header_from_height(&block_height, &tables)?,
    ))
}

/// [`BCReadRequest::BlockHash`].
#[inline]
fn block_hash(env: &ConcreteEnv, block_height: BlockHeight) -> ResponseResult {
    // Single-threaded, no `ThreadLocal` required.
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;
    let table_block_infos = env_inner.open_db_ro::<BlockInfos>(&tx_ro)?;

    Ok(BCResponse::BlockHash(
        get_block_info(&block_height, &table_block_infos)?.block_hash,
    ))
}

/// [`BCReadRequest::FilterUnknownHashes`].
#[inline]
fn filter_unknown_hahses(env: &ConcreteEnv, mut hashes: HashSet<BlockHash>) -> ResponseResult {
    // Single-threaded, no `ThreadLocal` required.
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;

    let table_block_heights = env_inner.open_db_ro::<BlockHeights>(&tx_ro)?;

    let mut err = None;

    hashes.retain(
        |block_hash| match block_exists(block_hash, &table_block_heights) {
            Ok(exists) => exists,
            Err(e) => {
                err.get_or_insert(e);
                false
            }
        },
    );

    if let Some(e) = err {
        Err(e)
    } else {
        Ok(BCResponse::FilterUnknownHashes(hashes))
    }
}

/// [`BCReadRequest::BlockExtendedHeaderInRange`].
#[inline]
fn block_extended_header_in_range(
    env: &ConcreteEnv,
    range: std::ops::Range<BlockHeight>,
) -> ResponseResult {
    // Prepare tx/tables in `ThreadLocal`.
    let env_inner = env.env_inner();
    let tx_ro = thread_local(env);
    let tables = thread_local(env);

    // Collect results using `rayon`.
    let vec = range
        .into_par_iter()
        .map(|block_height| {
            let tx_ro = tx_ro.get_or_try(|| env_inner.tx_ro())?;
            let tables = get_tables!(env_inner, tx_ro, tables)?.as_ref();
            get_block_extended_header_from_height(&block_height, tables)
        })
        .collect::<Result<Vec<ExtendedBlockHeader>, RuntimeError>>()?;

    Ok(BCResponse::BlockExtendedHeaderInRange(vec))
}

/// [`BCReadRequest::ChainHeight`].
#[inline]
fn chain_height(env: &ConcreteEnv) -> ResponseResult {
    // Single-threaded, no `ThreadLocal` required.
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;
    let table_block_heights = env_inner.open_db_ro::<BlockHeights>(&tx_ro)?;
    let table_block_infos = env_inner.open_db_ro::<BlockInfos>(&tx_ro)?;

    let chain_height = crate::ops::blockchain::chain_height(&table_block_heights)?;
    let block_hash =
        get_block_info(&chain_height.saturating_sub(1), &table_block_infos)?.block_hash;

    Ok(BCResponse::ChainHeight(chain_height, block_hash))
}

/// [`BCReadRequest::GeneratedCoins`].
#[inline]
fn generated_coins(env: &ConcreteEnv) -> ResponseResult {
    // Single-threaded, no `ThreadLocal` required.
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;
    let table_block_heights = env_inner.open_db_ro::<BlockHeights>(&tx_ro)?;
    let table_block_infos = env_inner.open_db_ro::<BlockInfos>(&tx_ro)?;

    let top_height = top_block_height(&table_block_heights)?;

    Ok(BCResponse::GeneratedCoins(cumulative_generated_coins(
        &top_height,
        &table_block_infos,
    )?))
}

/// [`BCReadRequest::Outputs`].
#[inline]
fn outputs(env: &ConcreteEnv, outputs: HashMap<Amount, HashSet<AmountIndex>>) -> ResponseResult {
    // Prepare tx/tables in `ThreadLocal`.
    let env_inner = env.env_inner();
    let tx_ro = thread_local(env);
    let tables = thread_local(env);

    // The 2nd mapping function.
    // This is pulled out from the below `map()` for readability.
    let inner_map = |amount, amount_index| -> Result<(AmountIndex, OutputOnChain), RuntimeError> {
        let tx_ro = tx_ro.get_or_try(|| env_inner.tx_ro())?;
        let tables = get_tables!(env_inner, tx_ro, tables)?.as_ref();

        let id = PreRctOutputId {
            amount,
            amount_index,
        };

        let output_on_chain = id_to_output_on_chain(&id, tables)?;

        Ok((amount_index, output_on_chain))
    };

    // Collect results using `rayon`.
    let map = outputs
        .into_par_iter()
        .map(|(amount, amount_index_set)| {
            Ok((
                amount,
                amount_index_set
                    .into_par_iter()
                    .map(|amount_index| inner_map(amount, amount_index))
                    .collect::<Result<HashMap<AmountIndex, OutputOnChain>, RuntimeError>>()?,
            ))
        })
        .collect::<Result<HashMap<Amount, HashMap<AmountIndex, OutputOnChain>>, RuntimeError>>()?;

    Ok(BCResponse::Outputs(map))
}

/// [`BCReadRequest::NumberOutputsWithAmount`].
#[inline]
fn number_outputs_with_amount(env: &ConcreteEnv, amounts: Vec<Amount>) -> ResponseResult {
    // Prepare tx/tables in `ThreadLocal`.
    let env_inner = env.env_inner();
    let tx_ro = thread_local(env);
    let tables = thread_local(env);

    // Cache the amount of RCT outputs once.
    // INVARIANT: #[cfg] @ lib.rs asserts `usize == u64`
    #[allow(clippy::cast_possible_truncation)]
    let num_rct_outputs = {
        let tx_ro = env_inner.tx_ro()?;
        let tables = env_inner.open_tables(&tx_ro)?;
        tables.rct_outputs().len()? as usize
    };

    // Collect results using `rayon`.
    let map = amounts
        .into_par_iter()
        .map(|amount| {
            let tx_ro = tx_ro.get_or_try(|| env_inner.tx_ro())?;
            let tables = get_tables!(env_inner, tx_ro, tables)?.as_ref();

            if amount == 0 {
                // v2 transactions.
                Ok((amount, num_rct_outputs))
            } else {
                // v1 transactions.
                match tables.num_outputs().get(&amount) {
                    // INVARIANT: #[cfg] @ lib.rs asserts `usize == u64`
                    #[allow(clippy::cast_possible_truncation)]
                    Ok(count) => Ok((amount, count as usize)),
                    // If we get a request for an `amount` that doesn't exist,
                    // we return `0` instead of an error.
                    Err(RuntimeError::KeyNotFound) => Ok((amount, 0)),
                    Err(e) => Err(e),
                }
            }
        })
        .collect::<Result<HashMap<Amount, usize>, RuntimeError>>()?;

    Ok(BCResponse::NumberOutputsWithAmount(map))
}

/// [`BCReadRequest::KeyImagesSpent`].
#[inline]
fn key_images_spent(env: &ConcreteEnv, key_images: HashSet<KeyImage>) -> ResponseResult {
    // Prepare tx/tables in `ThreadLocal`.
    let env_inner = env.env_inner();
    let tx_ro = thread_local(env);
    let tables = thread_local(env);

    // Key image check function.
    let key_image_exists = |key_image| {
        let tx_ro = tx_ro.get_or_try(|| env_inner.tx_ro())?;
        let tables = get_tables!(env_inner, tx_ro, tables)?.as_ref();
        key_image_exists(&key_image, tables.key_images())
    };

    // Collect results using `rayon`.
    match key_images
        .into_par_iter()
        .map(key_image_exists)
        // If the result is either:
        // `Ok(true)` => a key image was found, return early
        // `Err` => an error was found, return early
        //
        // Else, `Ok(false)` will continue the iterator.
        .find_any(|result| !matches!(result, Ok(false)))
    {
        None | Some(Ok(false)) => Ok(BCResponse::KeyImagesSpent(false)), // Key image was NOT found.
        Some(Ok(true)) => Ok(BCResponse::KeyImagesSpent(true)),          // Key image was found.
        Some(Err(e)) => Err(e), // A database error occurred.
    }
}
