//! # Database Reactor
//! This crate contains the database reactor implementation.

pub mod client;
pub mod message;
pub mod reactor;
pub mod thread;

use std::{pin::Pin, thread::JoinHandle, sync::Arc, time::Duration};

use cuprate_database::error::DBException;
use futures::{channel::{oneshot, mpsc}, Future, FutureExt};
use message::{DatabaseRequest, DatabaseResponse, DatabaseClientRequest};
use tower::Service;

#[derive(Debug, Clone)]
/// `Databaseclient` is a struct shared across the daemon to interact with database reactor, and therefore the underlying database
pub struct DatabaseClient {
	/// The channel used to send request to the reactor
	db: mpsc::Sender<DatabaseClientRequest>,
	/// A shared pointer to the reactor thread. Used to check if the reactor shutdowned properly.
	reactor_thread: Arc<JoinHandle<()>>,
}

impl DatabaseClient {

	/// This function send a message to stop the reactor, and check if it shutdowned properly.
	pub async fn shutdown(mut self) -> Result<(), ()> {

		if let DatabaseResponse::Shutdowned = self
			.call(DatabaseRequest::Shutdown)
			.await
			.map_err(|err| {})?
		{
			// A small delay is placed here to let the OS thread shutdown. The upper response is sent just before the end of the thread.
			std::thread::sleep(Duration::from_millis(200));
			if self.reactor_thread.is_finished() {
				return Ok(())
			}
		}
		Err(())
	}
}


impl tower::Service<DatabaseRequest> for DatabaseClient {
    type Response = DatabaseResponse;
    type Error = DBException;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.db
			.poll_ready(cx)
			.map_err(|_| DBException::Other("closed"))
    }

    fn call(&mut self, req: DatabaseRequest) -> Self::Future {
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