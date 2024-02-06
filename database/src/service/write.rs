//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    error::RuntimeError,
    service::{request::WriteRequest, response::WriteResponse},
    ConcreteDatabase,
};

use std::{
    future::{Future, IntoFuture},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::oneshot;

//---------------------------------------------------------------------------------------------------- Constants

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
    pub(super) sender: crossbeam::channel::Sender<WriteRequest>,
}

impl tower::Service<WriteRequest> for DatabaseWriteHandle {
    type Response = Result<WriteResponse, RuntimeError>; // TODO: This could be a more specific error?
    type Error = oneshot::error::RecvError; // TODO: always unwrap on channel failure?
    type Future = oneshot::Receiver<Result<WriteResponse, RuntimeError>>;

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
    receiver: crossbeam::channel::Receiver<WriteRequest>,

    /// TODO: either `Arc` or `&'static` after `Box::leak`
    db: Arc<ConcreteDatabase>,
}

impl DatabaseWriter {
    /// TODO
    pub(super) fn init(db: &Arc<ConcreteDatabase>) -> DatabaseWriteHandle {
        // Initalize `Request/Response` channels.
        let (sender, receiver) = crossbeam::channel::unbounded();

        // Spawn pool of writers.
        for _ in 0..2
        /* TODO: how many writers? */
        {
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
            self.request_to_db_function();
        }
    }

    /// Map [`Request`]'s to specific database functions.
    fn request_to_db_function(&mut self) {
        todo!();
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
