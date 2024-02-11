//! General free functions used (related to `cuprate_database::service`).

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    service::read::DatabaseReader,
    service::write::DatabaseWriter,
    service::{DatabaseReadHandle, DatabaseWriteHandle},
    ConcreteEnv,
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
/// This initializes a [`OnceLock`] containing the [`ConcreteEnv`],
/// meaning there is only 1 database per program.
///
/// Calling this function again will return handles to the same database.
pub fn init() -> &'static (DatabaseReadHandle, DatabaseWriteHandle) {
    DATABASE_HANDLES.get_or_init(|| {
        // Initialize the database itself.
        let db: ConcreteEnv = todo!();
        // Leak it, the database lives forever.
        //
        // TODO: there's probably shutdown code we have to run.
        // Leaking may not be viable, or atleast, we need to
        // be able to run destructors.
        let db: &'static ConcreteEnv = Box::leak(Box::new(db));

        // Spawn the `Reader/Writer` thread pools.
        let readers = DatabaseReader::init(db);
        let writers = DatabaseWriter::init(db);

        // Return the handles to those pools.
        (readers, writers)
    })
}

/// Sync/flush all data, and shutdown the database thread-pool.
///
/// This function **blocks**, waiting until:
/// 1. All database transactions are complete
/// 2. All data has been flushed to disk
/// 3. All database threads have exited
///
/// The database being shutdown is the one started in [`init()`],
/// aka, the single program global database.
///
/// # TODO
/// Maybe the visibility/access of this function should somehow be
/// limited such that only certain parts of `cuprate` can actually
/// call this function.
///
/// Anyone/everyone being able to shutdown the database seems dangerous.
///
/// Counter-argument: we can just CTRL+F to see who calls this i guess.
pub fn shutdown() {
    // Not sure how this function is going
    // to work on a `&'static` database, but:

    // 1. Send a shutdown message to all database threads, maybe `Request::Shutdown`
    // 2. Wait on barrier until all threads are "ready" (all tx's are done)
    // 3. Writer thread will flush all data to disk
    // 4. All threads exit, 1 of them sends us back an OK
    // 5. We don't need to reclaim ownership of `&'static ConcreteEnv` because...
    //   5a) a bunch of threads have a `&` to it, so this is hard (impossible?)
    //   5b) as along as data is flushed, we can just `std::process::exit`
    //       and there's no need to (manually) drop the actual database

    todo!();
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
