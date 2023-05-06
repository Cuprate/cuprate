use std::{thread::JoinHandle, sync::{atomic::AtomicBool, Arc}};

use cuprate_database::database::{Database, Interface};
use futures::{channel::mpsc::{Sender, Receiver, self}, SinkExt};

use crate::message::DatabaseClientRequest;

/// A Thread executing all Write operations in the database.
pub struct WriteThread {
	handle: JoinHandle<()>,
	tx: Sender<DatabaseClientRequest>,
}

impl WriteThread {

	/// Start the write thread with the given shared pointer to the database
	pub(crate) fn start<D: for<'reactor> Database<'reactor>>(db: Arc<D>) -> WriteThread {
		
		// Generating channels
		let (tx,rx) = mpsc::channel::<DatabaseClientRequest>(0);
		let handle = std::thread::spawn(move || {

			// Moving pointer and receiver
			let (rx, db) = (rx, db);

			write_thread(rx, db)
		});
		
		WriteThread { handle, tx }
	}

	pub(crate) fn stop(&self) -> Result<(), ()> {

		todo!()
	}
}

fn write_thread<D: for<'thread> Database<'thread>>(rx: Receiver<DatabaseClientRequest>, db: Arc<D>) {
	let mut interface = Interface::frodfazfdazfm(db);
}

pub struct ReadThread {
	handle: JoinHandle<()>,
	tx: Sender<DatabaseClientRequest>,
	status: Arc<AtomicBool>,
}