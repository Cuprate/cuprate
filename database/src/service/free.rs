//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    service::read::DatabaseReader,
    service::write::DatabaseWriter,
    service::{DatabaseReadHandle, DatabaseWriteHandle},
    ConcreteDatabase,
};

use std::sync::OnceLock;

//---------------------------------------------------------------------------------------------------- const/static
/// Read/write handles to the single, global program database.
///
/// [`init()`] will initialize this [`OnceLock`], and store
/// the initialized database's handles inside it.
///
/// Not accessible publically, outside crates use one of:
/// - [`init()`]
/// - [`db_read()`]
/// - [`db_write()`]
static DATABASE_HANDLES: OnceLock<(DatabaseReadHandle, DatabaseWriteHandle)> = OnceLock::new();

//---------------------------------------------------------------------------------------------------- Init
#[cold]
#[inline(never)] // Only called once.
/// Initialize the database, the thread-pool, and return a read/write handle to it.
///
/// This initializes a [`OnceLock`] containing the [`ConcreteDatabase`],
/// meaning there is only 1 database per program.
///
/// Calling this function again will return handles to the same database.
pub fn init() -> &'static (DatabaseReadHandle, DatabaseWriteHandle) {
    DATABASE_HANDLES.get_or_init(|| {
        // Initialize the database itself.
        let db: ConcreteDatabase = todo!();
        // Leak it, the database lives forever.
        //
        // TODO: there's probably shutdown code we have to run.
        // Leaking may not be viable, or atleast, we need to
        // be able to run destructors.
        let db: &'static ConcreteDatabase = Box::leak(Box::new(db));

        // Spawn the `Reader/Writer` thread pools.
        let readers = DatabaseReader::init(db);
        let writers = DatabaseWriter::init(db);

        // Return the handles to those pools.
        (readers, writers)
    })
}

#[inline]
/// Acquire a read handle to the single global database.
///
/// This returns a `static` read handle to
/// the database initialized in [`init()`].
///
/// This function will initialize the database if not already initialized.
pub fn db_read() -> &'static DatabaseReadHandle {
    &init().0
}

#[inline]
/// Acquire a write handle to the single global database.
///
/// This returns a `static` write handle to
/// the database initialized in [`init()`].
///
/// This function will initialize the database if not already initialized.
pub fn db_write() -> &'static DatabaseWriteHandle {
    &init().1
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
