//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    error::RuntimeError,
    service::{request::ReadRequest, response::ReadResponse},
    ConcreteDatabase,
};

use std::{
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::oneshot;

//---------------------------------------------------------------------------------------------------- Types
/// TODO
type Response = Result<ReadResponse, RuntimeError>;

/// TODO
type ResponseRecv = tokio::sync::oneshot::Receiver<Response>;

/// TODO
type ResponseSend = tokio::sync::oneshot::Sender<Response>;

//---------------------------------------------------------------------------------------------------- DatabaseReadHandle
/// TODO
///
/// A struct representing a thread-pool of database
/// readers that handle `tower::Service` requests.
#[derive(Clone, Debug)]
pub struct DatabaseReadHandle {
    /// TODO
    pub(super) sender: crossbeam::channel::Sender<(ReadRequest, ResponseSend)>,
}

impl tower::Service<ReadRequest> for DatabaseReadHandle {
    type Response = Response; // TODO: This could be a more specific error?
    type Error = oneshot::error::RecvError; // TODO: always unwrap on channel failure?
    type Future = ResponseRecv;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn call(&mut self, _req: ReadRequest) -> Self::Future {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseReader Impl
/// TODO
pub(super) struct DatabaseReader {
    /// TODO
    receiver: crossbeam::channel::Receiver<(ReadRequest, ResponseSend)>,

    /// TODO: either `Arc` or `&'static` after `Box::leak`
    db: Arc<ConcreteDatabase>,
}

impl DatabaseReader {
    /// TODO
    pub(super) fn init(db: &Arc<ConcreteDatabase>) -> DatabaseReadHandle {
        // Initalize `Request/Response` channels.
        let (sender, receiver) = crossbeam::channel::unbounded();

        // TODO: slightly _less_ readers per thread may be more ideal.
        // We could account for the writer count as well such that
        // readers + writers == total_thread_count
        let readers = cuprate_helper::thread::threads().get();

        // Spawn pool of readers.
        for _ in 0..readers {
            let receiver = receiver.clone();
            let db = Arc::clone(db);

            std::thread::spawn(move || {
                let this = Self { receiver, db };

                Self::main(this);
            });
        }

        // Return a handle to the pool.
        DatabaseReadHandle { sender }
    }

    /// The `DatabaseReader`'s main function.
    fn main(mut self) {
        loop {
            // 1. Hang on request channel
            // 2. Map request to some database function
            // 3. Execute that function, get the result
            // 4. Return the result via channel
            let (request, response_send) = match self.receiver.recv() {
                Ok((r, c)) => (r, c),
                Err(e) => {
                    // TODO: what to do with this channel error?
                    todo!();
                }
            };

            self.request_to_db_function(request, response_send);
        }
    }

    /// Map [`Request`]'s to specific database functions.
    fn request_to_db_function(&mut self, request: ReadRequest, response_send: ResponseSend) {
        match request {
            ReadRequest::Example1 => self.example_handler_1(response_send),
            ReadRequest::Example2(_x) => self.example_handler_2(response_send),
            ReadRequest::Example3(_x) => self.example_handler_3(response_send),
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

//---------------------------------------------------------------------------------------------------- Trait Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
