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
use futures::{channel::oneshot, ready};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::PollSemaphore;

use cuprate_helper::asynch::InfallibleOneshotReceiver;
use cuprate_types::{
    service::{ReadRequest, Response},
    ExtendedBlockHeader,
};

use crate::{
    config::ReaderThreads,
    constants::DATABASE_CORRUPT_MSG,
    error::RuntimeError,
    ops::block::{get_block_extended_header_from_height, get_block_info},
    service::types::{ResponseReceiver, ResponseResult, ResponseSender},
    tables::{BlockInfos, Tables},
    types::{Amount, AmountIndex, BlockHeight, KeyImage},
    ConcreteEnv, Env, EnvInner,
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
    env: Arc<RwLock<ConcreteEnv>>,
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
    pub(super) fn init(env: &Arc<RwLock<ConcreteEnv>>, reader_threads: ReaderThreads) -> Self {
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

    /// TODO
    #[inline]
    pub const fn env(&self) -> &Arc<RwLock<ConcreteEnv>> {
        &self.env
    }

    /// TODO
    #[inline]
    pub const fn semaphore(&self) -> &PollSemaphore {
        &self.semaphore
    }

    /// TODO
    #[inline]
    pub const fn permit(&self) -> &Option<OwnedSemaphorePermit> {
        &self.permit
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
        self.pool
            .spawn(move || map_request(permit, env, request, response_sender));

        InfallibleOneshotReceiver::from(receiver)
    }
}

//---------------------------------------------------------------------------------------------------- Request Mapping
// This function maps [`Request`]s to function calls
// executed by the rayon DB reader threadpool.

#[allow(clippy::needless_pass_by_value)] // TODO: fix me
/// Map [`Request`]'s to specific database handler functions.
///
/// This is the main entrance into all `Request` handler functions.
/// The basic structure is:
/// 1. `Request` is mapped to a handler function
/// 2. Handler function is called
/// 3. [`Response`] is sent
fn map_request(
    _permit: OwnedSemaphorePermit, // Permit for this request, dropped at end of function
    env: Arc<RwLock<ConcreteEnv>>, // Access to the database
    request: ReadRequest,          // The request we must fulfill
    response_sender: ResponseSender, // The channel we must send the response back to
) {
    use ReadRequest as R;

    /* TODO: pre-request handling, run some code for each request? */
    let env = env.read().expect(DATABASE_CORRUPT_MSG);

    let response = match request {
        R::BlockExtendedHeader(block) => block_extended_header(&env, block),
        R::BlockHash(block) => block_hash(&env, block),
        R::BlockExtendedHeaderInRange(range) => block_extended_header_in_range(&env, range),
        R::ChainHeight => chain_height(&env),
        R::GeneratedCoins => generated_coins(&env),
        R::Outputs(map) => outputs(&env, map),
        R::NumberOutputsWithAmount(vec) => number_outputs_with_amount(&env, vec),
        R::CheckKIsNotSpent(set) => check_k_is_not_spent(&env, set),
    };

    if let Err(e) = response_sender.send(response) {
        // TODO: use tracing.
        println!("database reader failed to send response: {e:?}");
    }

    /* TODO: post-request handling, run some code for each request? */
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

    // This iterator will early return as `Err` if there's even 1 error.
    let vec = range
        .into_par_iter()
        .map(|block_height| {
            let tx_ro = env_inner.tx_ro()?;
            let tables = env_inner.open_tables(&tx_ro)?;
            get_block_extended_header_from_height(&block_height, &tables)
        })
        .collect::<Result<Vec<ExtendedBlockHeader>, RuntimeError>>()?;

    Ok(Response::BlockExtendedHeaderInRange(vec))
}

/// [`ReadRequest::ChainHeight`].
#[inline]
fn chain_height(env: &ConcreteEnv) -> ResponseResult {
    todo!()
}

/// [`ReadRequest::GeneratedCoins`].
#[inline]
fn generated_coins(env: &ConcreteEnv) -> ResponseResult {
    todo!()
}

/// [`ReadRequest::Outputs`].
#[inline]
#[allow(clippy::needless_pass_by_value)] // TODO: remove me
fn outputs(env: &ConcreteEnv, map: HashMap<Amount, HashSet<AmountIndex>>) -> ResponseResult {
    todo!()
}

/// [`ReadRequest::NumberOutputsWithAmount`].
/// TODO
#[inline]
#[allow(clippy::needless_pass_by_value)] // TODO: remove me
fn number_outputs_with_amount(env: &ConcreteEnv, vec: Vec<Amount>) -> ResponseResult {
    todo!()
}

/// [`ReadRequest::CheckKIsNotSpent`].
#[inline]
#[allow(clippy::needless_pass_by_value)] // TODO: remove me
fn check_k_is_not_spent(env: &ConcreteEnv, set: HashSet<KeyImage>) -> ResponseResult {
    todo!()
}
