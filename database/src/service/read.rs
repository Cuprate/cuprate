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
/// The read response from the database reader thread.
///
/// This is an `Err` when the database itself errors
/// (e.g, get() object doesn't exist).
type Response = Result<ReadResponse, RuntimeError>;

/// The `Receiver` channel that receives the read response.
///
/// This is owned by the caller (the reader)
/// who `.await`'s for the response.
type ResponseRecv = tokio::sync::oneshot::Receiver<Response>;

/// The `Sender` channel for the response.
///
/// The database reader thread uses this to send
/// the database result to the caller.
type ResponseSend = tokio::sync::oneshot::Sender<Response>;

//---------------------------------------------------------------------------------------------------- DatabaseReadHandle
/// Read handle to the database.
///
/// This is cheaply [`Clone`]able handle that
/// allows `async`hronously reading from the database.
///
/// Calling [`DatabaseReadHandle::call`] with a [`ReadRequest`]
/// will return an `async`hronous channel that can be `.await`ed upon
/// to receive the corresponding [`ReadResponse`].
#[derive(Clone, Debug)]
pub struct DatabaseReadHandle {
    /// Sender channel to the database read thread-pool.
    ///
    /// We provide the response channel for the thread-pool.
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
    receiver: crossbeam::channel::Receiver<(ReadRequest, ResponseSend)>,

    /// TODO: either `Arc` or `&'static` after `Box::leak`
    /// Access to the database.
    db: Arc<ConcreteDatabase>,
}

impl DatabaseReader {
    /// Initialize the `DatabaseReader` thread-pool.
    ///
    /// This spawns `N` amount of `DatabaseReader`'s
    /// attached to `db` and returns a handle to the pool.
    ///
    /// Should be called _once_ per actual database.
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
    ///
    /// Each thread just loops in this function.
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
