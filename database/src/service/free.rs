//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    service::read::DatabaseReader,
    service::write::DatabaseWriter,
    service::{DatabaseReadHandle, DatabaseWriteHandle},
    ConcreteDatabase,
};

use std::sync::{Arc, OnceLock};

//---------------------------------------------------------------------------------------------------- const/static
/// TODO
static DATABASE_HANDLES: OnceLock<(DatabaseReadHandle, DatabaseWriteHandle)> = OnceLock::new();

//---------------------------------------------------------------------------------------------------- Init
/// Initialize the database thread pool, and return read/write handles to it.
pub fn init() -> &'static (DatabaseReadHandle, DatabaseWriteHandle) {
    DATABASE_HANDLES.get_or_init(||{
        // Initialize the database itself.
        let db: ConcreteDatabase = todo!();
        let db = Arc::new(db); // TODO: should be &'static ?

        // Spawn the `Reader/Writer` thread pools.
        let readers = DatabaseReader::init(&db);
        let writers = DatabaseWriter::init(&db);

        // Return the handles to those pools.
        (readers, writers)
    })
}

///
pub fn db_read() -> &'static DatabaseReadHandle {
    &init().0
}

///
pub fn db_write() -> &'static DatabaseWriteHandle {
    &init().1
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
