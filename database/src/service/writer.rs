//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    error::RuntimeError,
    service::{request::WriteRequest, response::WriteResponse},
};

use std::{
    future::{Future, IntoFuture},
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::oneshot;

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- DatabaseWriters
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
pub struct DatabaseWriter {
    /// TODO
    to_writers: crossbeam::channel::Sender<WriteRequest>,
}

//---------------------------------------------------------------------------------------------------- DatabaseWriter Impl
impl DatabaseWriter {
    /// TODO
    pub(super) fn init() -> Self {
        /* create channels, initialize data, spawn threads, etc */

        // TODO: return some handle, not `DatabaseWriter` itself
        todo!()
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

//---------------------------------------------------------------------------------------------------- `tower::Service`
impl tower::Service<WriteRequest> for DatabaseWriter {
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

//---------------------------------------------------------------------------------------------------- Trait Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
