//! Database reader thread-pool definitions and logic.

// `EnvInner` is a RwLock for `heed`.
// Clippy thinks it should be dropped earlier but it
// needs to be open until most functions return.
#![allow(clippy::significant_drop_tightening)]

//---------------------------------------------------------------------------------------------------- Import
use std::{
    collections::{HashMap, HashSet},
    num::NonZeroUsize,
    ops::Range,
    sync::{Arc, RwLock},
    task::{Context, Poll},
};

use cfg_if::cfg_if;
use crossbeam::channel::Receiver;
use curve25519_dalek::{constants::ED25519_BASEPOINT_POINT, edwards::CompressedEdwardsY, Scalar};
use futures::{channel::oneshot, ready};
use monero_serai::{transaction::Timelock, H};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use thread_local::ThreadLocal;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::PollSemaphore;

use cuprate_helper::asynch::InfallibleOneshotReceiver;
use cuprate_types::{
    service::{ReadRequest, Response},
    ExtendedBlockHeader, OutputOnChain,
};

use crate::{
    config::ReaderThreads,
    constants::DATABASE_CORRUPT_MSG,
    error::RuntimeError,
    ops::{
        block::{get_block_extended_header_from_height, get_block_info},
        blockchain::{cumulative_generated_coins, top_block_height},
        key_image::key_image_exists,
        output::get_output,
    },
    service::types::{ResponseReceiver, ResponseResult, ResponseSender},
    tables::{BlockHeights, BlockInfos, KeyImages, NumOutputs, Outputs, Tables},
    types::{Amount, AmountIndex, BlockHeight, KeyImage, OutputFlags, PreRctOutputId},
    ConcreteEnv, DatabaseRo, Env, EnvInner,
};

//---------------------------------------------------------------------------------------------------- DatabaseReadHandle
/// Read handle to the database.
///
/// This is cheaply [`Clone`]able handle that
/// allows `async`hronously reading from the database.
///
/// Calling [`tower::Service::call`] with a [`DatabaseReadHandle`] & [`ReadRequest`]
/// will return an `async`hronous channel that can be `.await`ed upon
/// to receive the corresponding [`Response`].
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
    /// TODO: we need this for testing but should we allow it publicly?
    /// Or within `crate`? It allows anyone to start tampering with the
    /// database directly instead of going through `service`.
    #[inline]
    pub const fn env(&self) -> &Arc<ConcreteEnv> {
        &self.env
    }
}

impl tower::Service<ReadRequest> for DatabaseReadHandle {
    type Response = Response;
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
    fn call(&mut self, request: ReadRequest) -> Self::Future {
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
/// 3. [`Response`] is sent
fn map_request(
    env: &ConcreteEnv,               // Access to the database
    request: ReadRequest,            // The request we must fulfill
    response_sender: ResponseSender, // The channel we must send the response back to
) {
    use ReadRequest as R;

    /* TODO: pre-request handling, run some code for each request? */

    let response = match request {
        R::BlockExtendedHeader(block) => block_extended_header(env, block),
        R::BlockHash(block) => block_hash(env, block),
        R::BlockExtendedHeaderInRange(range) => block_extended_header_in_range(env, range),
        R::ChainHeight => chain_height(env),
        R::GeneratedCoins => generated_coins(env),
        R::Outputs(map) => outputs(env, map),
        R::NumberOutputsWithAmount(vec) => number_outputs_with_amount(env, vec),
        R::CheckKIsNotSpent(set) => check_k_is_not_spent(env, set),
    };

    if let Err(e) = response_sender.send(response) {
        // TODO: use tracing.
        println!("database reader failed to send response: {e:?}");
    }

    /* TODO: post-request handling, run some code for each request? */
}

//---------------------------------------------------------------------------------------------------- Thread Local
/// `heed`'s transactions and tables are not `Sync`, so we cannot use
/// them with rayon, however, we set a feature such that they are `Send`.
///
/// Thus, before using rayon, we put the tx/table inside a
/// `ThreadLocal` which gives access to those threads.
///
/// <https://github.com/Cuprate/cuprate/pull/113#discussion_r1576762346>
#[inline]
fn thread_local<T: Send>(env: &impl Env) -> ThreadLocal<T> {
    ThreadLocal::with_capacity(env.config().reader_threads.as_threads().get())
}

/// Only `heed` requires the above [`thread_local()`] function,
/// as `redb`'s transactions and tables are `Send + Sync`.
///
/// Thus, wrapping them in `ThreadLocal` is wasteful.
///
/// This macro branches depending on what backend we're using
/// and either returns `ThreadLocal<T>` or the T directly.
///
/// An imaginary signature would look something like:
/// ```ignore
/// fn set_tx_ro_and_tables() -> if heed {
///     (ThreadLocal<TxRo>, ThreadLocal<Tables>) }
/// } else {
///     (TxRo, Tables)
/// };
/// ```
///
/// See [`get_tx_ro_and_tables`] for retrieving the output.
///
/// # Early return
/// Note that this early returns with `?` from whatever scope
/// it was called from if `tx_ro()` or `open_tables()` errors.
///
/// # Example
/// ```ignore
/// // Outside scope, still single threaded.
/// // Set the transaction and tables.
/// let (tx_ro, tables) = set_tx_ro_and_tables!(env, env_inner);
///
/// iter
///     .into_par_iter() // <- we've entered `rayon` scope
///     .map(|_| {
///         // Access the outside scope's `tx_ro` and `tables`.
///         // If needed, this will initialize some `ThreadLocal`'s.
///         let (tx_ro, tables) = get_tx_ro_and_tables!(env_inner, tx_ro, tables);
///
///         /* do rayon stuff */
///     });
/// ```
macro_rules! set_tx_ro_and_tables {
    ($env:ident, $env_inner:ident) => {{
        cfg_if::cfg_if! {
            if #[cfg(all(feature = "redb", not(feature = "heed")))] {
                let tx_ro = $env_inner.tx_ro()?;
                let tables = $env_inner.open_tables(tx_ro)?;
                (tx_ro, tables)
            } else {
                (thread_local($env), thread_local($env))
            }
        }
    }};
}

/// Access the values set with [`set_tx_ro_and_tables`].
macro_rules! get_tx_ro_and_tables {
    ($env_inner:ident, $tx_ro:ident, $tables:ident) => {{
        cfg_if::cfg_if! {
            if #[cfg(all(feature = "redb", not(feature = "heed")))] {
                ($tx_ro, $tables)
            } else {
                let tx_ro = $tx_ro.get_or_try(|| $env_inner.tx_ro())?;
                let tables = $tables.get_or_try(|| $env_inner.open_tables(tx_ro))?;
                (tx_ro, tables)
            }
        }
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

// TODO: implement multi-transaction read atomicity.
// <https://github.com/Cuprate/cuprate/pull/113#discussion_r1576874589>.

/// [`ReadRequest::BlockExtendedHeader`].
#[inline]
fn block_extended_header(env: &ConcreteEnv, block_height: BlockHeight) -> ResponseResult {
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;
    let tables = env_inner.open_tables(&tx_ro)?;

    Ok(Response::BlockExtendedHeader(
        get_block_extended_header_from_height(&block_height, &tables)?,
    ))
}

/// [`ReadRequest::BlockHash`].
#[inline]
fn block_hash(env: &ConcreteEnv, block_height: BlockHeight) -> ResponseResult {
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;
    let table_block_infos = env_inner.open_db_ro::<BlockInfos>(&tx_ro)?;

    Ok(Response::BlockHash(
        get_block_info(&block_height, &table_block_infos)?.block_hash,
    ))
}

/// [`ReadRequest::BlockExtendedHeaderInRange`].
#[inline]
fn block_extended_header_in_range(
    env: &ConcreteEnv,
    range: std::ops::Range<BlockHeight>,
) -> ResponseResult {
    let env_inner = env.env_inner();
    let (tx_ro, tables) = set_tx_ro_and_tables!(env, env_inner);

    let vec = range
        .into_par_iter()
        .map(|block_height| {
            let (tx_ro, tables) = get_tx_ro_and_tables!(env_inner, tx_ro, tables);
            get_block_extended_header_from_height(&block_height, tables)
        })
        .collect::<Result<Vec<ExtendedBlockHeader>, RuntimeError>>()?;

    Ok(Response::BlockExtendedHeaderInRange(vec))
}

/// [`ReadRequest::ChainHeight`].
#[inline]
fn chain_height(env: &ConcreteEnv) -> ResponseResult {
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;
    let table_block_heights = env_inner.open_db_ro::<BlockHeights>(&tx_ro)?;
    let table_block_infos = env_inner.open_db_ro::<BlockInfos>(&tx_ro)?;

    let top_height = top_block_height(&table_block_heights)?;
    let block_hash = get_block_info(&top_height, &table_block_infos)?.block_hash;

    Ok(Response::ChainHeight(top_height, block_hash))
}

/// [`ReadRequest::GeneratedCoins`].
#[inline]
fn generated_coins(env: &ConcreteEnv) -> ResponseResult {
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;
    let table_block_heights = env_inner.open_db_ro::<BlockHeights>(&tx_ro)?;
    let table_block_infos = env_inner.open_db_ro::<BlockInfos>(&tx_ro)?;

    let top_height = top_block_height(&table_block_heights)?;

    Ok(Response::GeneratedCoins(cumulative_generated_coins(
        &top_height,
        &table_block_infos,
    )?))
}

/// [`ReadRequest::Outputs`].
#[inline]
fn outputs(env: &ConcreteEnv, map: HashMap<Amount, HashSet<AmountIndex>>) -> ResponseResult {
    let env_inner = env.env_inner();
    let (tx_ro, tables) = set_tx_ro_and_tables!(env, env_inner);

    // -> Result<(AmountIndex, OutputOnChain), RuntimeError>
    let inner_map = |amount, amount_index| {
        let (tx_ro, tables) = get_tx_ro_and_tables!(env_inner, tx_ro, tables);

        let pre_rct_output_id = PreRctOutputId {
            amount,
            amount_index,
        };
        let output = get_output(&pre_rct_output_id, tables.outputs())?;

        //---  Map `Output` -> `OutputOnChain`

        // FIXME: implement lookup table for common values:
        // <https://github.com/monero-project/monero/blob/c8214782fb2a769c57382a999eaf099691c836e7/src/ringct/rctOps.cpp#L322>
        let commitment = ED25519_BASEPOINT_POINT + H() * Scalar::from(amount);

        let time_lock = if output
            .output_flags
            .contains(OutputFlags::NON_ZERO_UNLOCK_TIME)
        {
            // TODO: how to recover the timelock height/time?
            todo!()
        } else {
            Timelock::None
        };

        let key = CompressedEdwardsY::from_slice(&output.key)
            .map(|y| y.decompress())
            .unwrap_or(None);

        let output_on_chain = OutputOnChain {
            height: u64::from(output.height),
            time_lock,
            key,
            commitment,
        };

        Ok((amount_index, output_on_chain))
    };

    let map = map
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

    Ok(Response::Outputs(map))
}

/// [`ReadRequest::NumberOutputsWithAmount`].
#[inline]
fn number_outputs_with_amount(env: &ConcreteEnv, amounts: Vec<Amount>) -> ResponseResult {
    let env_inner = env.env_inner();
    let (tx_ro, tables) = set_tx_ro_and_tables!(env, env_inner);

    let map = amounts
        .into_par_iter()
        .map(|amount| {
            let (tx_ro, tables) = get_tx_ro_and_tables!(env_inner, tx_ro, tables);

            match tables.num_outputs().get(&amount) {
                // INVARIANT: #[cfg] @ lib.rs asserts `usize == u64`
                #[allow(clippy::cast_possible_truncation)]
                Ok(count) => Ok((amount, count as usize)),
                Err(RuntimeError::KeyNotFound) => Ok((amount, 0)),
                Err(e) => Err(e),
            }
        })
        .collect::<Result<HashMap<Amount, usize>, RuntimeError>>()?;

    Ok(Response::NumberOutputsWithAmount(map))
}

/// [`ReadRequest::CheckKIsNotSpent`].
#[inline]
fn check_k_is_not_spent(env: &ConcreteEnv, key_images: HashSet<KeyImage>) -> ResponseResult {
    let env_inner = env.env_inner();
    let (tx_ro, tables) = set_tx_ro_and_tables!(env, env_inner);

    let key_image_exists = |key_image| {
        let (tx_ro, tables) = get_tx_ro_and_tables!(env_inner, tx_ro, tables);
        key_image_exists(&key_image, tables.key_images())
    };

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
        None | Some(Ok(false)) => Ok(Response::CheckKIsNotSpent(true)), // Key image was NOT found.
        Some(Ok(true)) => Ok(Response::CheckKIsNotSpent(false)),        // Key image was found.
        Some(Err(e)) => Err(e), // A database error occurred.
    }
}
