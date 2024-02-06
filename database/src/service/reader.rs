//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    error::RuntimeError,
    service::{request::ReadRequest, response::ReadResponse},
};

use std::{
    future::{Future, IntoFuture},
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::oneshot;

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- DatabaseReader
/// TODO
///
/// A struct representing a thread-pool of database
/// readers that handle `tower::Service` requests.
pub struct DatabaseReader {
    /// TODO
    to_readers: crossbeam::channel::Sender<ReadRequest>,
}

//---------------------------------------------------------------------------------------------------- DatabaseReader Impl
impl DatabaseReader {
    /// TODO
    pub(super) fn init() -> Self {
        /* create channels, initialize data, spawn threads, etc */

        // TODO: return some handle, not `DatabaseReader` itself
        todo!()
    }

    /// The `DatabaseReader`'s main function.
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
impl tower::Service<ReadRequest> for DatabaseReader {
    type Response = Result<ReadResponse, RuntimeError>; // TODO: This could be a more specific error?
    type Error = oneshot::error::RecvError; // TODO: always unwrap on channel failure?
    type Future = oneshot::Receiver<Result<ReadResponse, RuntimeError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn call(&mut self, _req: ReadRequest) -> Self::Future {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- Trait Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
