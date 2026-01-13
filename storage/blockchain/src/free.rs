//! General free functions (related to the database).

use fjall::{CompressionType, KeyspaceCreateOptions};
//---------------------------------------------------------------------------------------------------- Import
use tapes::{Persistence, TapeOpenOptions, Tapes};


use crate::{
    config::Config,
    Blockchain,
};
use crate::database::{BLOCK_INFOS, PRUNABLE_BLOBS, PRUNED_BLOBS, RCT_OUTPUTS, TX_INFOS, V1_PRUNABLE_BLOBS};
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


    let fjall_keyspace = fjall::SingleWriterTxDatabase::builder(&config.data_dir).max_write_buffer_size(Some(124 * 1024 * 1024)).cache_size(2 * 1024 * 1024 * 1024 / 4).open().unwrap();

    let block_heights_fjall = fjall_keyspace.keyspace("BLOCK_HEIGHTS",|| KeyspaceCreateOptions::default().manual_journal_persist(true).max_memtable_size(16 * 1024 * 1024)).unwrap();
    let key_images_fjall = fjall_keyspace.keyspace("key_images_fjall", || KeyspaceCreateOptions::default().manual_journal_persist(true).max_memtable_size(16 * 1024 * 1024)).unwrap();
    let pre_rct_outputs_fjall = fjall_keyspace.keyspace("pre_rct_outputs_fjall", || KeyspaceCreateOptions::default().manual_journal_persist(true).max_memtable_size(16 * 1024 * 1024)).unwrap();
    let tx_ids_fjall = fjall_keyspace.keyspace("tx_ids_fjall", || KeyspaceCreateOptions::default().manual_journal_persist(true).max_memtable_size(16 * 1024 * 1024)).unwrap();
    let tx_outputs_fjall = fjall_keyspace.keyspace("tx_outputs_fjall", || KeyspaceCreateOptions::default().manual_journal_persist(true).max_memtable_size(16 * 1024 * 1024)).unwrap();

    let linear_tapes = Tapes::open(&config.data_dir )?;
    let mut tape_append_tx = linear_tapes.append();
    
    let rct_outputs = tape_append_tx.open_fixed_sized_tape(RCT_OUTPUTS, &TapeOpenOptions {
        top_cache_size: 100 * 1024 * 1024,
        dir: config.data_dir.clone() ,
    })?;
    let tx_infos = tape_append_tx.open_fixed_sized_tape(TX_INFOS, &TapeOpenOptions {
        top_cache_size: 16 * 1024,
        dir: config.data_dir.clone() ,
    })?;
    let block_infos = tape_append_tx.open_fixed_sized_tape(BLOCK_INFOS, &TapeOpenOptions {
        top_cache_size: 16 * 1024,
        dir: config.data_dir.clone() ,
    })?;
    let pruned_blobs = tape_append_tx.open_blob_tape(PRUNED_BLOBS, &TapeOpenOptions {
        top_cache_size:  1024 * 1024,
        dir: config.data_dir.clone() ,
    })?;
    let v1_prunable_blobs = tape_append_tx.open_blob_tape(V1_PRUNABLE_BLOBS, &TapeOpenOptions {
        top_cache_size:  8096,
        dir: config.data_dir.clone() ,
    })?;
    
    let prunable_blobs = (0..8).map(|i| {
        tape_append_tx.open_blob_tape(PRUNABLE_BLOBS[i], &TapeOpenOptions {
            top_cache_size: 8096,
            dir: config.data_dir.clone() ,
        })
    }).collect::<Result<_, _>>()?;

    tape_append_tx.commit(Persistence::SyncAll)?;

    drop(tape_append_tx);

    tracing::debug!("opened db");
    Ok(Blockchain {
        fjall_keyspace,
        linear_tapes,
        block_heights_fjall,
        key_images_fjall,
        pre_rct_outputs_fjall,
        tx_ids_fjall,
        tx_outputs_fjall,
        rct_outputs,
        tx_infos,
        block_infos,
        pruned_blobs,
        v1_prunable_blobs,
        prunable_blobs,
        pre_rct_numb_outputs_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
    })
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
