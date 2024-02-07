//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    error::RuntimeError,
    service::{request::WriteRequest, response::WriteResponse},
    ConcreteDatabase,
};

use std::{
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::oneshot;

//---------------------------------------------------------------------------------------------------- Types
/// TODO
type Response = Result<WriteResponse, RuntimeError>;

/// TODO
type ResponseRecv = tokio::sync::oneshot::Receiver<Response>;

/// TODO
type ResponseSend = tokio::sync::oneshot::Sender<Response>;

//---------------------------------------------------------------------------------------------------- DatabaseWriteHandle
/// TODO
///
/// A handle to the `DatabaseWriterPool`.
///
/// Crates outside `cuprate-database` will be interacting with
/// this opaque struct to send/receive write requests/responses.
#[derive(Clone, Debug)]
pub struct DatabaseWriteHandle {
    /// TODO
    pub(super) sender: crossbeam::channel::Sender<(WriteRequest, ResponseSend)>,
}

impl tower::Service<WriteRequest> for DatabaseWriteHandle {
    type Response = Result<WriteResponse, RuntimeError>; // TODO: This could be a more specific error?
    type Error = oneshot::error::RecvError; // TODO: always unwrap on channel failure?
    type Future = ResponseRecv;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn call(&mut self, _req: WriteRequest) -> Self::Future {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseWriter
/// TODO
///
/// A struct representing the thread-pool (maybe just 2 threads).
/// that handles `tower::Service` write requests.
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
pub(super) struct DatabaseWriter {
    /// TODO
    receiver: crossbeam::channel::Receiver<(WriteRequest, ResponseSend)>,

    /// TODO: either `Arc` or `&'static` after `Box::leak`
    db: Arc<ConcreteDatabase>,
}

impl DatabaseWriter {
    /// TODO
    pub(super) fn init(db: &Arc<ConcreteDatabase>) -> DatabaseWriteHandle {
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
            let db = Arc::clone(db);

            std::thread::spawn(move || {
                let this = Self { receiver, db };

                Self::main(this);
            });
        }

        // Return a handle to the pool.
        DatabaseWriteHandle { sender }
    }

    /// The `DatabaseWriter`'s main function.
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

            self.request_to_db_function(request, response_send);
        }
    }

    /// Map [`Request`]'s to specific database functions.
    fn request_to_db_function(&mut self, request: WriteRequest, response_send: ResponseSend) {
        match request {
            WriteRequest::Example1 => self.example_handler_1(response_send),
            WriteRequest::Example2(_x) => self.example_handler_2(response_send),
            WriteRequest::Example3(_x) => self.example_handler_3(response_send),
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
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
