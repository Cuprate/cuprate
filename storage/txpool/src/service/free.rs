use std::sync::Arc;

use rayon::ThreadPool;

use crate::error::TxPoolError;
use crate::service::{TxpoolReadHandle, TxpoolWriteHandle};
use crate::txpool::TxpoolDatabase;

//---------------------------------------------------------------------------------------------------- Init
#[cold]
#[inline(never)] // Only called once (?)
/// TODO
pub fn init_with_pool(
    database: fjall::Database,
    pool: Arc<ThreadPool>,
) -> Result<(TxpoolReadHandle, TxpoolWriteHandle), TxPoolError> {
    let database = Arc::new(TxpoolDatabase::open_with_database(database)?);

    // Spawn the Reader thread pool and Writer.
    let readers = TxpoolReadHandle {
        txpool: Arc::clone(&database),
        pool: pool.clone(),
    };
    let writer = TxpoolWriteHandle {
        txpool: Arc::clone(&database),
        pool,
    };

    Ok((readers, writer))
}
