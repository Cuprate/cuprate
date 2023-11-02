//---------------------------------------------------------------------------------------------------- use
use std::pin::Pin;
use std::future::Future;
use std::task::{Context,Poll};
use std::thread::JoinHandle;
use tokio::sync::*;
use std::path::Path;

//---------------------------------------------------------------------------------------------------- Layer 5 - Service
// The "requests" other Cuprate crates can send.
macro_rules! impl_request {
	(
		$request:ident, // Name of the request struct.
		$response:ty,   // The expected response type.
		$error:ty       // The potential error that might arise when calling.
	) => {
		// #[derive(stuff, ...)]
		//
		// This is just a zero-sized marker struct
		// to indicate the generic request.
		//
		// Could be `Vec<u8>` or `Bytes` instead if
		// we're directly forwarding bytes as requests.
		//
		// Holds onto the `oneshot` channel that
		// we will eventually get a `Response` from.
		struct $request(oneshot::Receiver<Result<$response, $error>>);
	};

	impl<D: Database> tower::Service<()> for AbstractDatabase<D> {
		type Error    = ();
		type Response = ();
		type Future   = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

		fn poll_ready(&mut self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
			Poll::Ready(Ok(()))
		}

		fn call(&mut self, request: $request) -> Self::Future {
			// 1. Create response channel.
			let (tx, rx) = tokio::sync::oneshot::channel::<$response>();

			// 2. Send request to "the thread"
			// tx.send(...);

			// 3. Create future.

			// 4. `await` on that created future...?
		}
	}
}

//---------------------------------------------------------------------------------------------------- Layer 4 - Thread
fn init_db_thread() -> JoinHandle<()> {
	std::thread::spawn(move || {
		// Receive requests.
		//
		// Map the request to a certain F
		// where that F is a function that
		// returns the expected result.
		//
		// Once we get the result, send it
		// over the oneshot channel that
		// was given to us by the requester.
		for request in recv.iter() {
			match request {
				Request1(one_shot) => request_handler_1(one_shot),
				Request2(one_shot) => request_handler_2(one_shot),
				Request3(one_shot) => request_handler_3(one_shot),

				// [...]
			}
		}
	})
}

//---------------------------------------------------------------------------------------------------- Layer 3 - AbstractDatabase
// Temporary fields, just for the gist
struct AbstractDatabase<D: Database> {
	request: SomeChannelType<Receiver<>>,
	database: D,
}

//---------------------------------------------------------------------------------------------------- Layer 2 - Trait
// Generics are temporary, just for the gist
trait Database<K, V> {
	// Required methods.
	fn open<P: AsRef<Path>>(path: P) -> Result<Self, ()>;
	fn get(&self, key: &K) -> Result<V, ()>;

	// Provided (higher-level) methods.
	fn get_block(&self, index: usize) -> Result<Block, ()> {
		// blah blah
		// - open db
		// - find table of blocks
		self.database.get(index) // -> returns corresponding block
	}
}

//---------------------------------------------------------------------------------------------------- Layer 1 - Database
// This code would exist in `database/src/backend/{D}.rs`
// where D is a database name, e.g, `lmdb.rs`.
//
// Each file would implement the above `Database` trait.
//
// If the database allows for more optimal methods,
// the provided methods can be re-implemented.
impl Database for lmdb::Database {
	fn open<P: AsRef<Path>>(path: P) -> Result<Self, ()> {
		// impl open
	}

	fn get(&self, key: &K) -> Result<V, ()> {
		// impl get
	}

	// `get_block()` is already implemented but
	// let's re-impl it with lmdb-specific stuff
	fn get_block(&self, index: usize) -> Result<Block, ()> {
		// better lmdb-specific impl
	}
}