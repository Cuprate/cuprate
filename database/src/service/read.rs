//! Database read thread-pool definitions and logic.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    sync::Arc,
    task::{Context, Poll},
};

use crossbeam::channel::Receiver;

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
/// Either our [Response], or a database error occurred.
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
type ResponseSend = tokio::sync::oneshot::Sender<ResponseResult>;

//---------------------------------------------------------------------------------------------------- DatabaseReadHandle
/// Read handle to the database.
///
/// This is cheaply [`Clone`]able handle that
/// allows `async`hronously reading from the database.
///
/// Calling [`tower::Service::call`] with a [`DatabaseReadHandle`] & [`ReadRequest`]
/// will return an `async`hronous channel that can be `.await`ed upon
/// to receive the corresponding [`Response`].
#[derive(Clone, Debug)]
pub struct DatabaseReadHandle {
    /// Sender channel to the database read thread-pool.
    ///
    /// We provide the response channel for the thread-pool.
    pub(super) sender: crossbeam::channel::Sender<(ReadRequest, ResponseSend)>,
}

impl tower::Service<ReadRequest> for DatabaseReadHandle {
    type Response = Response;
    type Error = RuntimeError;
    type Future = ResponseRecv;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    #[inline]
    fn call(&mut self, _req: ReadRequest) -> Self::Future {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseReader
/// Database reader thread.
///
/// This struct essentially represents a thread.
///
/// Each reader thread is spawned with access to this struct (self).
pub(super) struct DatabaseReader {
    /// Receiver side of the database request channel.
    ///
    /// Any caller can send some requests to this channel.
    /// They send them alongside another `Response` channel,
    /// which we will eventually send to.
    ///
    /// We (the database reader thread) are not responsible
    /// for creating this channel, the caller provides it.
    ///
    /// SOMEDAY: this struct itself could cache a return channel
    /// instead of creating a new `oneshot` each request.
    receiver: Receiver<(ReadRequest, ResponseSend)>,

    /// Access to the database.
    db: Arc<ConcreteEnv>,
}

impl Drop for DatabaseReader {
    fn drop(&mut self) {
        // INVARIANT: we set the thread name when spawning it.
        let thread_name = std::thread::current().name().unwrap();

        // TODO: log that this thread has exited?
    }
}

impl DatabaseReader {
    /// Initialize the `DatabaseReader` thread-pool.
    ///
    /// This spawns `N` amount of `DatabaseReader`'s
    /// attached to `db` and returns a handle to the pool.
    ///
    /// Should be called _once_ per actual database.
    #[cold]
    #[inline(never)] // Only called once.
    pub(super) fn init(db: &Arc<ConcreteEnv>, reader_threads: ReaderThreads) -> DatabaseReadHandle {
        // Initialize `Request/Response` channels.
        let (sender, receiver) = crossbeam::channel::unbounded();

        // How many reader threads to spawn?
        let reader_count = reader_threads.as_threads();

        // Spawn pool of readers.
        for i in 0..reader_count.get() {
            let receiver = receiver.clone();
            let db = Arc::clone(db);

            std::thread::Builder::new()
                .name(format!("cuprate_helper::service::read::DatabaseReader{i}"))
                .spawn(move || {
                    let this = Self { receiver, db };
                    Self::main(this);
                })
                .unwrap();
        }

        // Return a handle to the pool and channels to
        // allow clean shutdown of all reader threads.
        DatabaseReadHandle { sender }
    }

    /// The `DatabaseReader`'s main function.
    ///
    /// Each thread just loops in this function.
    #[cold]
    #[inline(never)] // Only called once.
    fn main(self) {
        // 1. Hang on request channel
        // 2. Map request to some database function
        // 3. Execute that function, get the result
        // 4. Return the result via channel
        loop {
            // Database requests.
            let Ok((request, response_send)) = self.receiver.recv() else {
                // If this receive errors, it means that the channel is empty
                // and disconnected, meaning the other side (all senders) have
                // been dropped. This means "shutdown", and we return here to
                // exit the thread.
                return;
            };

            // Map [`Request`]'s to specific database functions.
            match request {
                ReadRequest::Example1 => self.example_handler_1(response_send),
                ReadRequest::Example2(_x) => self.example_handler_2(response_send),
                ReadRequest::Example3(_x) => self.example_handler_3(response_send),
            }
        }
    }

    /// TODO
    #[inline]
    fn example_handler_1(&self, response_send: ResponseSend) {
        let db_result = todo!();
        response_send.send(db_result).unwrap();
    }

    /// TODO
    #[inline]
    fn example_handler_2(&self, response_send: ResponseSend) {
        let db_result = todo!();
        response_send.send(db_result).unwrap();
    }

    /// TODO
    #[inline]
    fn example_handler_3(&self, response_send: ResponseSend) {
        let db_result = todo!();
        response_send.send(db_result).unwrap();
    }
}
