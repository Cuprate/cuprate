//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    error::RuntimeError,
    service::{request::ReadRequest, response::ReadResponse},
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

//---------------------------------------------------------------------------------------------------- DatabaseReadHandle
/// TODO
///
/// A struct representing a thread-pool of database
/// readers that handle `tower::Service` requests.
#[derive(Clone, Debug)]
pub struct DatabaseReadHandle {
    /// TODO
    pub(super) sender: crossbeam::channel::Sender<ReadRequest>,
}

impl tower::Service<ReadRequest> for DatabaseReadHandle {
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

//---------------------------------------------------------------------------------------------------- DatabaseReader Impl
/// TODO
pub(super) struct DatabaseReader {
    /// TODO
    receiver: crossbeam::channel::Receiver<ReadRequest>,

    /// TODO: either `Arc` or `&'static` after `Box::leak`
    db: Arc<ConcreteDatabase>,
}

impl DatabaseReader {
    /// TODO
    pub(super) fn init(db: &Arc<ConcreteDatabase>) -> DatabaseReadHandle {
        // Initalize `Request/Response` channels.
        let (sender, receiver) = crossbeam::channel::unbounded();

        // Spawn pool of readers.
        for _ in 0..2
        /* TODO: how many readers? */
        {
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
            self.request_to_db_function();
        }
    }

    /// Map [`Request`]'s to specific database functions.
    fn request_to_db_function(&mut self) {
        todo!();
    }
}

//---------------------------------------------------------------------------------------------------- Trait Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
