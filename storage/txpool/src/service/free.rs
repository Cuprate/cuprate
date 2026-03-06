use std::sync::Arc;

use rayon::ThreadPool;

use crate::{
    error::TxPoolError,
    service::{TxpoolReadHandle, TxpoolWriteHandle},
    txpool::TxpoolDatabase,
};

//---------------------------------------------------------------------------------------------------- Init
#[cold]
#[inline(never)] // Only called once (?)
/// Initialise a database and return a read/write handle to it.
///
/// Once the returned handles are [`Drop::drop`]ed, the reader
/// thread-pool and writer thread will exit automatically.
///
/// # Errors
/// This will forward the error if the opening failed.
pub fn init_with_pool(
    database: fjall::Database,
    pool: Arc<ThreadPool>,
) -> Result<(TxpoolReadHandle, TxpoolWriteHandle), TxPoolError> {
    let database = Arc::new(TxpoolDatabase::open_with_database(database)?);

    // Spawn the Reader thread pool and Writer.
    let readers = TxpoolReadHandle {
        txpool: Arc::clone(&database),
        pool: Arc::clone(&pool),
    };
    let writer = TxpoolWriteHandle {
        txpool: Arc::clone(&database),
        pool,
    };

    Ok((readers, writer))
}
