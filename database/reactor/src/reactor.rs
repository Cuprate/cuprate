use std::{thread::JoinHandle, path::PathBuf, sync::{RwLock, Arc}};

use futures::{channel::{oneshot, mpsc::{self, Receiver}}, Future, FutureExt};
use cuprate_database::{database::{Database, Interface}, error::DBException};
use libmdbx::{NoWriteMap, WriteMap};
use tracing::{span, Level, event, Span};
use crate::{message::{DatabaseRequest, DatabaseResponse, DatabaseClientRequest}, DatabaseClient, thread::{WriteThread, ReadThread}};


/// Actual database reactor struct being used in the reactor thread
pub struct DatabaseReactor {
	/// Channel for receiving clients request 
	client: mpsc::Receiver<DatabaseClientRequest>,
	/// Access to WriteThread
	write_thread: Option<WriteThread>,
	/// Vector of ReadThread
	read_threads: Vec<ReadThread>,
	/// The number of write being performed in the database
	write_count: u64,
}

/// In-memory cache of database.
pub struct ReactorCache {
	/// Cache of Blockchain's height
	ChainHeight: RwLock<u64>,
	/// Cache of Core Sync Data, used when connecting with other peers
	CoreSyncData: RwLock<()>, // RwLock<CoreSyncData>
}

impl DatabaseReactor {

	/// Start the reactor thread and its underlying database
	pub fn init(path: PathBuf, id: &'static str, num_thread: u64, write_count: u64) -> Result<DatabaseClient, std::io::Error> {

		let dbreactor_span = span!(Level::TRACE, "DatabaseReactor");
		let _guard = dbreactor_span.enter();

		event!(Level::INFO, "Starting database reactor...");

		let (dbclient_tx, dbclient_rx) = mpsc::channel::<DatabaseClientRequest>(num_thread as usize);
		event!(Level::TRACE, "DBClientReq channel done");

		drop(_guard);
		let builder = std::thread::Builder::new().name("DatabaseReactor".to_string());
		let reactor_thread = builder.spawn(move || {
			let rx = dbclient_rx;
			let span = dbreactor_span;
			
			let mut reactor = DatabaseReactor::new(rx, num_thread, write_count);
			match id {
				"mdbx" => reactor.reactor_thread::<libmdbx::Database<WriteMap>>(path),
				_ => unreachable!()
			}
		})?;

		Ok(DatabaseClient {
			db: dbclient_tx,
			reactor_thread: Arc::new(reactor_thread)
		})
	}

	fn new(rx: Receiver<DatabaseClientRequest>, num_thread: u64, write_count: u64) -> Self {
		DatabaseReactor { 
			client: rx, 
			write_thread: None, 
			read_threads: Vec::with_capacity(num_thread as usize), 
			mm_size: 0, 
			write_count 
		}
	}

	fn reactor_thread<D: for<'reactor> Database<'reactor>>(&mut self, path: PathBuf) {
		// Re-entering DatabaseReactor thread
		let span = Span::current();
		let _guard = span.enter();

		event!(Level::TRACE, "Started reactor thread...");

		// Opening the database. If the database don't open we can't start the node
		match <D as Database>::open(path) {
			Err(err) => {
				event!(Level::ERROR, "Failed to open database: {}", err);
				std::process::exit(0);
			}
			Ok(db) => {
				event!(Level::TRACE, "Database::open() successful");

				// Checking if this is a valid database
				if let Err(err) = &db.check_all_tables_exist() {
					event!(Level::ERROR, "Database tables aren't present, It must be corrupted: {}", err);
					std::process::exit(0);
				}

				match Interface::from(Arc::new(db)) {
					Ok(interface) => {
						event!(Level::TRACE, "Started first interface")

						// start thread and loop
					}
					Err(err) => {
						event!(Level::ERROR, "Can't start database interface: {}", err);
						std::process::exit(0)
					}
				}
			}
		}
	}
}