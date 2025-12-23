//! General free functions (related to the database).

use fjall::{CompressionType, KeyspaceCreateOptions};
//---------------------------------------------------------------------------------------------------- Import
use tapes::{MmapFileOpenOption, Tapes};


use crate::{
    config::{linear_tapes_config, Config},
    Blockchain,
};
use crate::error::BlockchainError;

//---------------------------------------------------------------------------------------------------- Free functions
/// Open the blockchain database using the passed [`Config`].
///
/// This calls [`cuprate_database::Env::open`] and prepares the
/// database to be ready for blockchain-related usage, e.g.
/// table creation, table sort order, etc.
///
/// All tables found in [`crate::tables`] will be
/// ready for usage in the returned [`ConcreteEnv`].
///
/// # Errors
/// This will error if:
/// - The database file could not be opened
/// - A write transaction could not be opened
/// - A table could not be created/opened
#[cold]
#[inline(never)] // only called once
pub fn open(config: Config) -> Result<Blockchain, BlockchainError> {


    let fjall_keyspace = fjall::SingleWriterTxDatabase::builder(&config.data_dir).manual_journal_persist(true).max_write_buffer_size(128 * 1024 * 1024).open().unwrap();

      let block_heights_fjall = fjall_keyspace.keyspace("BLOCK_HEIGHTS",|| KeyspaceCreateOptions::default().manual_journal_persist(true).max_memtable_size(128 * 1024 * 1024)).unwrap();
        let key_images_fjall = fjall_keyspace.keyspace("key_images_fjall", || KeyspaceCreateOptions::default().manual_journal_persist(true).max_memtable_size(128 * 1024 * 1024)).unwrap();
        let pre_rct_outputs_fjall = fjall_keyspace.keyspace("pre_rct_outputs_fjall", || KeyspaceCreateOptions::default().manual_journal_persist(true).max_memtable_size(128 * 1024 * 1024)).unwrap();
        let tx_ids_fjall = fjall_keyspace.keyspace("tx_ids_fjall", || KeyspaceCreateOptions::default().manual_journal_persist(true).max_memtable_size(128 * 1024 * 1024)).unwrap();
        let tx_outputs_fjall = fjall_keyspace.keyspace("tx_outputs_fjall", || KeyspaceCreateOptions::default().manual_journal_persist(true).max_memtable_size(128 * 1024 * 1024)).unwrap();


    let tapes = linear_tapes_config(config.data_dir.clone(), config.blob_data_dir);

    let linear_tapes = unsafe {
        Tapes::new(
            tapes,
            MmapFileOpenOption {
                dir: config.data_dir,
            },
            1024 * 1204 * 1024,
        )?
    };

    tracing::debug!("opened db");
    Ok(Blockchain {
        fjall_keyspace,
        linear_tapes,
        block_heights_fjall,
        key_images_fjall,
        pre_rct_outputs_fjall,
        tx_ids_fjall,
        tx_outputs_fjall,
        pre_rct_numb_outputs_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
    })
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
