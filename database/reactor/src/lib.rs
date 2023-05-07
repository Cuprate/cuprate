//! # Database Reactor
//! This crate contains the database reactor implementation.

pub mod client;
pub mod message;
pub mod reactor;
pub mod thread;

use std::{pin::Pin, thread::JoinHandle, sync::Arc};

use cuprate_database::error::DBException;
use futures::{channel::{oneshot, mpsc}, Future, FutureExt};
use message::{DatabaseRequest, DatabaseResponse, DatabaseClientRequest};

#[derive(Debug, Clone)]
/// `Databaseclient` is a struct shared across the daemon to interact with database reactor, and therefore the underlying database
pub struct DatabaseClient {
	/// The channel used to send request to the reactor
	db: mpsc::Sender<DatabaseClientRequest>,
	/// Shared handle to the reactor thread to check if the thread is stopped
	reactor_thread: Arc<JoinHandle<()>>
}

/// Implementing Tower service for the database client
impl tower::Service<DatabaseRequest> for DatabaseClient {
    type Response = DatabaseResponse;
    type Error = DBException; // The reactor can sent back to the caller database errors, such as NotFound for example
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

	/// `poll_ready` check if the channel is sempty and therefore is waiting to process a request
    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.db
			.poll_ready(cx)
			.map_err(|_| DBException::Other("closed"))
    }

	/// `call` to send a request to the database
    fn call(&mut self, req: DatabaseRequest) -> Self::Future {
		// Generating result oneshot::channel
        let (tx, rx) = oneshot::channel::<Result<Self::Response, Self::Error>>();
        // get the callers span
        let span = tracing::span::Span::current();
		
        let req = DatabaseClientRequest { req, tx, span };

        match self.db.try_send(req) {
            Err(_e) => {
                // I'm assuming all callers will call `poll_ready` first (which they are supposed to)
                futures::future::ready(Err(DBException::Other("closed"))).boxed()
            }
            Ok(()) => async move {
                rx.await
                    .expect("Database Reactor will not drop requests until completed")
            }
            .boxed(),
        }
    }
}