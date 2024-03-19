//! Database read thread-pool definitions and logic.

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
type ResponseRecv = InfallibleOneshotReceiver<ResponseResult>;

/// The `Sender` channel for the response.
///
/// The database reader thread uses this to send
/// the database result to the caller.
type ResponseSend = oneshot::Sender<ResponseResult>;

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
    type Future = ResponseRecv;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Threadpool is always ready as long as this handle is alive.
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: ReadRequest) -> Self::Future {
        // Response channel we `.await` on.
        let (response_send, receiver) = oneshot::channel();

        // Database thread's state.
        let db_reader = DatabaseReader {
            response_send,
            db: Arc::clone(&self.db),
        };

        // Spawn the request in the rayon DB thread-pool.
        self.pool.spawn(move || db_reader.map_request(request));

        InfallibleOneshotReceiver::from(receiver)
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseReader
/// Database reader thread state.
///
/// This struct represents the state every
/// `DatabaseReadHandle` rayon thread will have access to.
///
/// Could just be function inputs, although this allows `self`.
pub(super) struct DatabaseReader {
    /// The channel we must send the response back to.
    response_send: ResponseSend,

    /// Access to the database.
    db: Arc<ConcreteEnv>,
}

impl DatabaseReader {
    #[inline]
    /// Map [`Request`]'s to specific database handler functions.
    fn map_request(self, request: ReadRequest) {
        match request {
            ReadRequest::Example1 => self.example_handler_1(),
            ReadRequest::Example2(_x) => self.example_handler_2(),
            ReadRequest::Example3(_x) => self.example_handler_3(),
        }
    }

    /// TODO
    #[inline]
    fn example_handler_1(self) {
        let db_result = todo!();
        self.response_send.send(db_result).unwrap();
    }

    /// TODO
    #[inline]
    fn example_handler_2(self) {
        let db_result = todo!();
        self.response_send.send(db_result).unwrap();
    }

    /// TODO
    #[inline]
    fn example_handler_3(self) {
        let db_result = todo!();
        self.response_send.send(db_result).unwrap();
    }
}

impl Drop for DatabaseReader {
    fn drop(&mut self) {
        // INVARIANT: we set the thread name when spawning it.
        let thread_name = std::thread::current().name().unwrap();

        // TODO: log that this response has finished?
    }
}
