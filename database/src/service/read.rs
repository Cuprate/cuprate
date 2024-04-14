//! Database reader thread-pool definitions and logic.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    sync::Arc,
    task::{Context, Poll},
};

use crossbeam::channel::Receiver;

use futures::{channel::oneshot, ready};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::PollSemaphore;

use cuprate_helper::asynch::InfallibleOneshotReceiver;
use cuprate_types::service::{ReadRequest, Response};

use crate::{
    config::ReaderThreads,
    error::RuntimeError,
    types::{Amount, AmountIndex, BlockHeight, KeyImage},
    ConcreteEnv,
};

//---------------------------------------------------------------------------------------------------- Types
/// The actual type of the response.
///
/// Either our [`Response`], or a database error occurred.
type ResponseResult = Result<Response, RuntimeError>;

/// The `Receiver` channel that receives the read response.
///
/// This is owned by the caller (the reader)
/// who `.await`'s for the response.
///
/// The channel itself should never fail,
/// but the actual database operation might.
type ResponseReceiver = InfallibleOneshotReceiver<ResponseResult>;

/// The `Sender` channel for the response.
///
/// The database reader thread uses this to send
/// the database result to the caller.
type ResponseSender = oneshot::Sender<ResponseResult>;

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
            pool: self.pool.clone(),
            semaphore: self.semaphore.clone(),
            permit: None,
            env: self.env.clone(),
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

    /// TODO
    #[inline]
    pub const fn env(&self) -> &Arc<ConcreteEnv> {
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
        let permit = ready!(self.semaphore.poll_acquire(cx))
            .expect("`self` itself owns the backing semaphore, so it can't be closed.");

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
        let env = Arc::clone(self.env());
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
    env: Arc<ConcreteEnv>,         // Access to the database
    request: ReadRequest,          // The request we must fulfill
    response_sender: ResponseSender, // The channel we must send the response back to
) {
    /* TODO: pre-request handling, run some code for each request? */
    use ReadRequest as R;

    let response = match request {
        R::BlockExtendedHeader(block) => block_extended_header(&env, block),
        R::BlockHash(block) => block_hash(&env, block),
        R::BlockExtendedHeaderInRange(range) => block_extended_header_in_range(&env, range),
        R::ChainHeight => chain_height(&env),
        R::GeneratedCoins => generated_coins(&env),
        R::Outputs(map) => outputs(&env, map),
        R::NumberOutputsWithAmount(vec) => number_outputs_with_amount(&env, vec),
        R::CheckKIsNotSpent(set) => check_k_is_not_spent(&env, set),
        R::BlockBatchInRange(range) => block_batch_in_range(&env, range),
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
fn block_extended_header(env: &Arc<ConcreteEnv>, block_height: BlockHeight) -> ResponseResult {
    todo!()
}

/// [`ReadRequest::BlockHash`].
#[inline]
fn block_hash(env: &Arc<ConcreteEnv>, block_height: BlockHeight) -> ResponseResult {
    todo!()
}

/// [`ReadRequest::BlockExtendedHeaderInRange`].
#[inline]
fn block_extended_header_in_range(
    env: &Arc<ConcreteEnv>,
    range: std::ops::Range<BlockHeight>,
) -> ResponseResult {
    todo!()
}

/// [`ReadRequest::ChainHeight`].
#[inline]
fn chain_height(env: &Arc<ConcreteEnv>) -> ResponseResult {
    todo!()
}

/// [`ReadRequest::GeneratedCoins`].
#[inline]
fn generated_coins(env: &Arc<ConcreteEnv>) -> ResponseResult {
    todo!()
}

/// [`ReadRequest::Outputs`].
#[inline]
#[allow(clippy::needless_pass_by_value)] // TODO: remove me
fn outputs(env: &Arc<ConcreteEnv>, map: HashMap<Amount, HashSet<AmountIndex>>) -> ResponseResult {
    todo!()
}

/// [`ReadRequest::NumberOutputsWithAmount`].
/// TODO
#[inline]
#[allow(clippy::needless_pass_by_value)] // TODO: remove me
fn number_outputs_with_amount(env: &Arc<ConcreteEnv>, vec: Vec<Amount>) -> ResponseResult {
    todo!()
}

/// [`ReadRequest::CheckKIsNotSpent`].
#[inline]
#[allow(clippy::needless_pass_by_value)] // TODO: remove me
fn check_k_is_not_spent(env: &Arc<ConcreteEnv>, set: HashSet<KeyImage>) -> ResponseResult {
    todo!()
}

/// [`ReadRequest::BlockBatchInRange`].
#[inline]
fn block_batch_in_range(env: &Arc<ConcreteEnv>, range: Range<BlockHeight>) -> ResponseResult {
    todo!()
}
