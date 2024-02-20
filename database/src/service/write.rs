//! Database write thread-pool definitions and logic.

//---------------------------------------------------------------------------------------------------- Import
use std::task::{Context, Poll};

use cuprate_helper::asynch::InfallibleOneshotReceiver;

use crate::{
    error::RuntimeError,
    service::{read::DatabaseReaderShutdown, request::WriteRequest, response::Response},
    ConcreteEnv, Env,
};

//---------------------------------------------------------------------------------------------------- Types
/// The actual type of the response.
///
/// Either our [Response], or a database error occurred.
type ResponseResult = Result<Response, RuntimeError>;

/// The `Receiver` channel that receives the write response.
///
/// The channel itself should never fail,
/// but the actual database operation might.
type ResponseRecv = InfallibleOneshotReceiver<ResponseResult>;

/// The `Sender` channel for the response.
type ResponseSend = tokio::sync::oneshot::Sender<ResponseResult>;

//---------------------------------------------------------------------------------------------------- DatabaseWriteHandle
/// Write handle to the database.
///
/// This is handle that allows `async`hronously writing to the database,
/// it is not [`Clone`]able as there is only ever 1 place within Cuprate
/// that writes.
///
/// Calling [`tower::Service::call`] with a [`DatabaseWriteHandle`] & [`WriteRequest`]
/// will return an `async`hronous channel that can be `.await`ed upon
/// to receive the corresponding [`Response`].
#[derive(Debug)]
pub struct DatabaseWriteHandle {
    /// Sender channel to the database write thread-pool.
    ///
    /// We provide the response channel for the thread-pool.
    pub(super) sender: crossbeam::channel::Sender<(WriteRequest, ResponseSend)>,
}

impl tower::Service<WriteRequest> for DatabaseWriteHandle {
    type Response = Response;
    type Error = RuntimeError;
    type Future = ResponseRecv;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    #[inline]
    fn call(&mut self, _req: WriteRequest) -> Self::Future {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseWriter
/// The single database writer thread.
pub(super) struct DatabaseWriter {
    /// Receiver side of the database request channel.
    ///
    /// Any caller can send some requests to this channel.
    /// They send them alongside another `Response` channel,
    /// which we will eventually send to.
    receiver: crossbeam::channel::Receiver<(WriteRequest, ResponseSend)>,

    /// TODO
    readers: DatabaseReaderShutdown,

    /// Access to the database.
    db: ConcreteEnv,
}

impl DatabaseWriter {
    /// Initialize the single `DatabaseWriter` thread.
    #[cold]
    #[inline(never)] // Only called once.
    pub(super) fn init(db: &ConcreteEnv, readers: DatabaseReaderShutdown) -> DatabaseWriteHandle {
        // Initialize `Request/Response` channels.
        let (sender, receiver) = crossbeam::channel::unbounded();

        // Spawn the writer.
        let db = ConcreteEnv::clone(db);
        std::thread::spawn(move || {
            let this = Self {
                receiver,
                readers,
                db,
            };

            Self::main(this);
        });

        // Return a handle to the pool.
        DatabaseWriteHandle { sender }
    }

    /// The `DatabaseWriter`'s main function.
    ///
    /// The writer just loops in this function.
    #[cold]
    #[inline(never)] // Only called once.
    fn main(mut self) {
        loop {
            // 1. Hang on request channel
            // 2. Map request to some database function
            // 3. Execute that function, get the result
            // 4. Return the result via channel
            let (request, response_send) = match self.receiver.recv() {
                Ok(tuple) => tuple,
                Err(e) => {
                    // TODO: what to do with this channel error?
                    todo!();
                }
            };

            // Map [`Request`]'s to specific database functions.
            match request {
                WriteRequest::Example1 => self.example_handler_1(response_send),
                WriteRequest::Example2(_x) => self.example_handler_2(response_send),
                WriteRequest::Example3(_x) => self.example_handler_3(response_send),
                WriteRequest::Shutdown => {
                    // Run shutdown code and exit thread.
                    Self::shutdown(self, response_send);
                    return;
                }
            }
        }
    }

    /// TODO
    #[cold]
    #[inline(never)]
    #[allow(clippy::unused_self)] // `self` must not be dropped.
    fn shutdown(self, response_send: ResponseSend) {
        // Shutdown all the reader threads.
        self.readers.shutdown();

        // Flush the database to disk.
        match self.db.sync() {
            Ok(()) => {
                // Tell the shutdown request that we're done shutting down.
                response_send
                    .send(Ok(Response::ExampleWriteResponse))
                    .unwrap();
            }
            Err(e) => {
                // TODO: what to do if this final sync fails?
                // We might be `SyncMode::Fastest` so this erroring
                // is bad news, it means (potentially) some of our
                // data isn't synced and we might corrupt if we exit here.
                //
                // We could try a few times before failing.
                response_send.send(Err(e)).unwrap();
            }
        }
    }

    /// Resize the database's memory map.
    fn resize_map(&self) {
        // The compiler most likely optimizes out this
        // entire function call if this returns here.
        if !ConcreteEnv::MANUAL_RESIZE {
            return;
        }

        let current_map_size = self.db.current_map_size();
        let new_size_bytes = crate::free::resize_memory_map(current_map_size);

        // INVARIANT:
        // [`Env`]'s that are `MANUAL_RESIZE` are expected to implement
        // their internals such that we have exclusive access when calling
        // this function. _We_ do not handle the exclusion part, `resize_map()`
        // itself does. The `heed` backend does this with `RwLock`.
        //
        // We need mutual exclusion due to:
        // <http://www.lmdb.tech/doc/group__mdb.html#gaa2506ec8dab3d969b0e609cd82e619e5>
        self.db.resize_map(new_size_bytes);
    }

    /// TODO
    #[inline]
    fn example_handler_1(&mut self, response_send: ResponseSend) {
        let db_result = todo!();
        response_send.send(db_result).unwrap();
    }

    /// TODO
    #[inline]
    fn example_handler_2(&mut self, response_send: ResponseSend) {
        let db_result = todo!();
        response_send.send(db_result).unwrap();
    }

    /// TODO
    #[inline]
    fn example_handler_3(&mut self, response_send: ResponseSend) {
        let db_result = todo!();
        response_send.send(db_result).unwrap();
    }
}
