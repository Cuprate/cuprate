//! Database write thread-pool definitions and logic.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    sync::Arc,
    task::{Context, Poll},
};

use cuprate_helper::asynch::InfallibleOneshotReceiver;

use crate::{
    error::RuntimeError,
    service::{request::WriteRequest, response::Response},
    ConcreteEnv,
};

//---------------------------------------------------------------------------------------------------- Types
/// The actual type of the response.
///
/// Either our [Response], or a database error occured.
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
/// This is cheaply [`Clone`]able handle that
/// allows `async`hronously writing to the database.
///
/// Calling [`tower::Service::call`] with a [`DatabaseWriteHandle`] & [`WriteRequest`]
/// will return an `async`hronous channel that can be `.await`ed upon
/// to receive the corresponding [`WriteResponse`].
#[derive(Clone, Debug)]
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
/// Database writer thread.
///
/// Each reader thread is spawned with access to this struct (self).
///
pub(super) struct DatabaseWriter {
    /// Receiver side of the database request channel.
    ///
    /// Any caller can send some requests to this channel.
    /// They send them alongside another `Response` channel,
    /// which we will eventually send to.
    receiver: crossbeam::channel::Receiver<(WriteRequest, ResponseSend)>,

    /// TODO: either `Arc` or `&'static` after `Box::leak`
    /// Access to the database.
    db: Arc<ConcreteEnv>,
}

impl DatabaseWriter {
    /// Initialize the `DatabaseWriter` thread-pool.
    ///
    /// This spawns `N` amount of `DatabaseWriter`'s
    /// attached to `db` and returns a handle to the pool.
    ///
    /// The reason this isn't a single thread is to allow a little
    /// more work to be done during the leeway in-between DB operations, e.g:
    ///
    /// ```text
    ///  Request1
    ///     |                                      Request2
    ///     v                                         |
    ///  Writer1                                      v
    ///     |                                      Writer2
    /// doing work                                    |
    ///     |                               waiting on other writer
    /// sending response  --|                         |
    ///     |               |- leeway            doing work
    ///     v             --|                         |
    ///    done                                       |
    ///                                        sending response
    ///                                               |
    ///                                               v
    ///                                              done
    /// ```
    ///
    /// During `leeway`, `Writer1` is:
    /// - busy preparing the message
    /// - creating the response channel
    /// - sending it back to the channel
    /// - ...etc
    ///
    /// During this time, it would be wasteful if `Request2`'s was
    /// just being waited on, so instead, `Writer2` handles that
    /// work while `Writer1` is in-between tasks.
    ///
    /// This leeway is pretty tiny (doesn't take long to allocate a channel
    /// and send a response) so there's only 2 Writers for now (needs testing).
    ///
    /// The database backends themselves will hang on write transactions if
    /// there are other existing ones, so we ourselves don't need locks.
    #[cold]
    #[inline(never)] // Only called once.
    pub(super) fn init(db: &Arc<ConcreteEnv>) -> DatabaseWriteHandle {
        // Initalize `Request/Response` channels.
        let (sender, receiver) = crossbeam::channel::unbounded();

        // TODO: should we scale writers up as we have more threads?
        //
        // The below function causes this [thread-count <-> writer] mapping:
        // <=16t -> 2
        //   32t -> 3
        //   64t -> 6
        //  128t -> 12
        //
        // 3+ writers might harm more than help.
        // Anyone have a 64c/128t CPU to test on...?
        let writers = std::cmp::min(2, cuprate_helper::thread::threads_10().get());

        // Spawn pool of writers.
        for _ in 0..writers {
            let receiver = receiver.clone();
            let db = db.clone();

            std::thread::spawn(move || {
                let this = Self { receiver, db };

                Self::main(this);
            });
        }

        // Return a handle to the pool.
        DatabaseWriteHandle { sender }
    }

    /// The `DatabaseWriter`'s main function.
    /// The `DatabaseReader`'s main function.
    ///
    /// Each thread just loops in this function.
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
                    /* TODO: run shutdown code */
                    Self::shutdown(self);

                    // Return, exiting the thread.
                    return;
                }
            }
        }
    }

    /// TODO
    fn example_handler_1(&mut self, response_send: ResponseSend) {
        let db_result = todo!();
        response_send.send(db_result).unwrap();
    }

    /// TODO
    fn example_handler_2(&mut self, response_send: ResponseSend) {
        let db_result = todo!();
        response_send.send(db_result).unwrap();
    }

    /// TODO
    fn example_handler_3(&mut self, response_send: ResponseSend) {
        let db_result = todo!();
        response_send.send(db_result).unwrap();
    }

    /// TODO
    fn shutdown(self) {
        todo!()
    }
}
