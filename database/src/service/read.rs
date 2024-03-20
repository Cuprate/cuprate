//! Database reader thread-pool definitions and logic.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    sync::Arc,
    task::{Context, Poll},
};

use crossbeam::channel::Receiver;

use futures::channel::oneshot;

use cuprate_helper::asynch::InfallibleOneshotReceiver;

use crate::{
    config::ReaderThreads,
    error::RuntimeError,
    service::{request::ReadRequest, response::Response},
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
#[derive(Clone)]
pub struct DatabaseReadHandle {
    /// Handle to the custom `rayon` DB reader thread-pool.
    ///
    /// Requests are "spawn"ed in this thread-pool, and responses
    /// are returned via a channel we (the caller) provide.
    pub(super) pool: Arc<rayon::ThreadPool>,

    /// Access to the database.
    pub(super) db: Arc<ConcreteEnv>,
}

impl DatabaseReadHandle {
    /// Initialize the `DatabaseReader` thread-pool backed by `rayon`.
    ///
    /// This spawns `N` amount of `DatabaseReader`'s
    /// attached to `db` and returns a handle to the pool.
    ///
    /// Should be called _once_ per actual database.
    #[cold]
    #[inline(never)] // Only called once.
    pub(super) fn init(db: &Arc<ConcreteEnv>, reader_threads: ReaderThreads) -> Self {
        // How many reader threads to spawn?
        let reader_count = reader_threads.as_threads();

        // Spawn `rayon` reader threadpool.
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(reader_count.get())
            .thread_name(|i| format!("cuprate_helper::service::read::DatabaseReader{i}"))
            .build()
            .unwrap();

        // Return a handle to the pool.
        Self {
            pool: Arc::new(pool),
            db: Arc::clone(db),
        }
    }

    /// TODO
    #[inline]
    pub const fn db(&self) -> &Arc<ConcreteEnv> {
        &self.db
    }
}

impl tower::Service<ReadRequest> for DatabaseReadHandle {
    type Response = Response;
    type Error = RuntimeError;
    type Future = ResponseReceiver;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Threadpool is always ready as long as this handle is alive.
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: ReadRequest) -> Self::Future {
        // Response channel we `.await` on.
        let (response_sender, receiver) = oneshot::channel();

        // Database thread's state.
        let db_reader = DatabaseReader {
            db: Arc::clone(&self.db),
        };

        // Spawn the request in the rayon DB thread-pool.
        //
        // Note that this uses `self.pool` instead of `rayon::spawn`
        // such that any `rayon` parallel code that runs within
        // the passed closure uses the same `rayon` threadpool.
        //
        // INVARIANT:
        // The below `DatabaseReader` function impl block relies on this behavior.
        self.pool
            .spawn(move || db_reader.map_request(request, response_sender));

        InfallibleOneshotReceiver::from(receiver)
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseReader
/// Database reader thread state.
///
/// This struct represents (some of the) state every
/// `DatabaseReadHandle` rayon thread will have access to.
///
/// Could just be function inputs, although this allows `self`
/// and a `Drop` impl such that each request has the same cleanup routine.
pub(super) struct DatabaseReader {
    /// Access to the database.
    db: Arc<ConcreteEnv>,
    // FIXME:
    // `request` and `response_sender` can't be in here
    // for `drop()` + `send()` taking by value.
}

impl Drop for DatabaseReader {
    fn drop(&mut self) {
        // INVARIANT: we set the thread name when spawning it.
        let thread_name = std::thread::current().name().unwrap();

        // TODO: log that this response has finished?
    }
}

// INVARIANT:
// These functions are called above in `tower::Service::call()`
// using a custom threadpool which means any call to `par_*()` functions
// will be using the custom rayon DB reader thread-pool, not the global one.
//
// All functions in this `impl` block assume that this is the case,
// such that `par_*()` functions will not block the _global_ rayon thread-pool.
impl DatabaseReader {
    #[inline]
    /// Map [`Request`]'s to specific database handler functions.
    ///
    /// This is the main entrance into all `Request` handler functions.
    /// The basic structure is:
    ///
    /// 1. `Request` is mapped to a handler function
    /// 2. Handler function is called
    /// 3. [`Response`] is sent
    /// 4. [`Self`] drop code runs
    fn map_request(
        self,
        request: ReadRequest,            // The request we must fulfill
        response_sender: ResponseSender, // The channel we must send the response back to
    ) {
        match request {
            ReadRequest::Example1 => self.example_handler_1(response_sender),
            ReadRequest::Example2(x) => self.example_handler_2(response_sender, x),
            ReadRequest::Example3(x) => self.example_handler_3(response_sender, x),
        }
    }

    /// TODO
    #[inline]
    #[allow(clippy::unused_self)] // TODO: remove me
    fn example_handler_1(self, response_sender: ResponseSender) {
        let db_result = Ok(Response::Example1);
        response_sender.send(db_result).unwrap();
    }

    /// TODO
    #[inline]
    #[allow(clippy::unused_self)] // TODO: remove me
    fn example_handler_2(self, response_sender: ResponseSender, x: usize) {
        let db_result = Ok(Response::Example2(x));
        response_sender.send(db_result).unwrap();
    }

    /// TODO
    #[inline]
    #[allow(clippy::unused_self)] // TODO: remove me
    fn example_handler_3(self, response_sender: ResponseSender, x: String) {
        let db_result = Ok(Response::Example3(x));
        response_sender.send(db_result).unwrap();
    }
}
