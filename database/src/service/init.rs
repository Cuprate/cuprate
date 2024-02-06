//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    error::RuntimeError,
    service::read::DatabaseReader,
    service::write::DatabaseWriter,
    service::{
        DatabaseReadHandle, DatabaseWriteHandle, ReadRequest, ReadResponse, WriteRequest,
        WriteResponse,
    },
    ConcreteDatabase,
};

use std::{
    sync::Arc,
    task::{Context, Poll},
};

//---------------------------------------------------------------------------------------------------- Init
/// TODO
///
/// Initialize the database thread pool, and return read/write handles to it.
pub fn init() -> (DatabaseReadHandle, DatabaseWriteHandle) {
    // Initialize the database itself.
    let db: ConcreteDatabase = todo!();
    let db = Arc::new(db); // TODO: should be &'static ?

    // Spawn the `Reader/Writer` thread pools.
    let readers = DatabaseReader::init(&db);
    let writers = DatabaseWriter::init(&db);

    // Return the handles to those pools.
    (readers, writers)
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
