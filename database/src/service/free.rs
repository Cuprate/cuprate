//! General free functions used (related to `cuprate_database::service`).

//---------------------------------------------------------------------------------------------------- Import
use std::sync::Arc;

use crate::{
    config::Config,
    error::InitError,
    service::{
        read::DatabaseReader, write::DatabaseWriter, DatabaseReadHandle, DatabaseWriteHandle,
    },
    ConcreteEnv, Env,
};

//---------------------------------------------------------------------------------------------------- Init
#[cold]
#[inline(never)] // Only called once (?)
/// Initialize a database & thread-pool, and return a read/write handle to it.
///
/// The returned handles are cheaply [`Clone`]able.
///
/// TODO: add blocking behavior docs.
///
/// # Errors
/// TODO
pub fn init(config: Config) -> Result<(DatabaseReadHandle, DatabaseWriteHandle), InitError> {
    let reader_threads = config.reader_threads;

    // Initialize the database itself.
    let db: Arc<ConcreteEnv> = Arc::new(ConcreteEnv::open(config)?);

    // Spawn the Reader thread pool and Writer.
    let readers = DatabaseReader::init(&db, reader_threads);
    let writers = DatabaseWriter::init(&db);

    // Return the handles to those pools.
    Ok((readers, writers))
}

#[cold]
#[inline(never)] // Only called once.
/// # TODO
/// This is pretty much happens, but this function can be deleted.
/// Document how shutdown works, maybe in `service` module docs.
///
/// //------------------------------------------------------------
///
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
pub fn shutdown(db: ConcreteEnv) {
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

    drop(db);
    todo!();
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
