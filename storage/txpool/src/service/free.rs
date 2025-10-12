use std::sync::Arc;

use rayon::ThreadPool;

use cuprate_database::{ConcreteEnv, InitError};

use crate::{
    service::{
        read::{init_read_service, init_read_service_with_pool},
        types::{TxpoolReadHandle, TxpoolWriteHandle},
        write::init_write_service,
    },
    Config,
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
/// This will forward the error if [`crate::open`] failed.
pub fn init(
    config: &Config,
) -> Result<(TxpoolReadHandle, TxpoolWriteHandle, Arc<ConcreteEnv>), InitError> {
    let reader_threads = config.reader_threads;

    // Initialize the database itself.
    let db = Arc::new(crate::open(config)?);

    // Spawn the Reader thread pool and Writer.
    let readers = init_read_service(Arc::clone(&db), reader_threads)?;
    let writer = init_write_service(Arc::clone(&db))?;

    Ok((readers, writer, db))
}

#[cold]
#[inline(never)] // Only called once (?)
/// Initialize a database, and return a read/write handle to it.
///
/// Unlike [`init`] this will not create a thread-pool, instead using
/// the one passed in.
///
/// Once the returned handles are [`Drop::drop`]ed, the reader
/// thread-pool and writer thread will exit automatically.
///
/// # Errors
/// This will forward the error if [`crate::open`] failed.
pub fn init_with_pool(
    config: &Config,
    pool: Arc<ThreadPool>,
) -> Result<(TxpoolReadHandle, TxpoolWriteHandle, Arc<ConcreteEnv>), InitError> {
    // Initialize the database itself.
    let db = Arc::new(crate::open(config)?);

    // Spawn the Reader thread pool and Writer.
    let readers = init_read_service_with_pool(Arc::clone(&db), pool);
    let writer = init_write_service(Arc::clone(&db))?;

    Ok((readers, writer, db))
}
