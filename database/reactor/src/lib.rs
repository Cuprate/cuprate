//! This crate contains the database reactor implementation.

pub mod client;
pub mod message;
pub mod reactor;
pub mod thread;

use std::{pin::Pin, thread::JoinHandle, path::{Path, PathBuf}, sync::Arc};

use cuprate_database::error::DBException;
use futures::{channel::{oneshot, mpsc}, Future, FutureExt};
use message::{DatabaseRequest, DatabaseResponse, DatabaseClientRequest};

#[derive(Debug, Clone)]
pub struct DatabaseClient {
	db: mpsc::Sender<DatabaseClientRequest>,
	reactor_thread: Arc<JoinHandle<()>>,
}

impl DatabaseClient {

	pub fn stop_reactor() -> Result<(), ()> {
		todo!()
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