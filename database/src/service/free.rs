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
/// Once the returned handles are [`Drop::drop`]ed, the reader
/// thread-pool and writer thread will exit automatically.
///
/// # Errors
/// This will forward the error if [`Env::open`] failed.
//
// INVARIANT:
// `cuprate_database` depends on the fact that this is the only
// function that hands out the handles. After that, they can be
// cloned, however they must eventually be dropped and shouldn't
// be leaked.
//
// As the reader thread-pool and writer thread both rely on the
// disconnection (drop) of these channels for shutdown behavior,
// leaking these handles could cause data to not get flushed to disk.
pub fn init(config: Config) -> Result<(DatabaseReadHandle, DatabaseWriteHandle), InitError> {
    let reader_threads = config.reader_threads;

    // Initialize the database itself.
    let db: Arc<ConcreteEnv> = Arc::new(ConcreteEnv::open(config)?);

    // Spawn the Reader thread pool and Writer.
    let readers = DatabaseReadHandle::init(&db, reader_threads);
    let writer = DatabaseWriteHandle::init(db);

    Ok((readers, writer))
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
