//! General free functions (related to the database).

use heed::{DatabaseFlags, DefaultComparator, EnvFlags, EnvOpenOptions};
//---------------------------------------------------------------------------------------------------- Import
use cuprate_linear_tapes::LinearTapes;

use crate::database::{
    ALT_BLOCKS_INFO, ALT_BLOCK_BLOBS, ALT_BLOCK_HEIGHTS, ALT_CHAIN_INFOS, ALT_TRANSACTION_BLOBS,
    ALT_TRANSACTION_INFOS, BLOCK_HEIGHTS, KEY_IMAGES, PRE_RCT_OUTPUTS, TX_IDS, TX_OUTPUTS,
};
use crate::{
    config::{linear_tapes_config, Config},
    Blockchain,
};

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
pub fn open(config: Config) -> Result<Blockchain, heed::Error> {
    // Attempt to open the database environment.
    let env = {
        // <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L1324>

        let mut env_open_options = EnvOpenOptions::new();

        // SAFETY: the flags we're setting are 'unsafe'
        // from a data durability perspective, although,
        // the user config wanted this.
        //
        // MAYBE: We may need to open/create tables with certain flags
        // <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L1324>
        // MAYBE: Set comparison functions for certain tables
        // <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L1324>
        unsafe {
            env_open_options.flags(
                EnvFlags::NO_READ_AHEAD
                    | EnvFlags::NO_SYNC
                    | EnvFlags::WRITE_MAP
                    | EnvFlags::MAP_ASYNC,
            );
        }

        // Set the max amount of database tables.
        // We know at compile time how many tables there are.
        // SOMEDAY: ...how many?
        env_open_options.max_dbs(32);

        env_open_options.map_size(30 * 1024 * 1024 * 1024);

        // LMDB documentation:
        // ```
        // Number of slots in the reader table.
        // This value was chosen somewhat arbitrarily. 126 readers plus a
        // couple mutexes fit exactly into 8KB on my development machine.
        // ```
        // <https://github.com/LMDB/lmdb/blob/b8e54b4c31378932b69f1298972de54a565185b1/libraries/liblmdb/mdb.c#L794-L799>
        //
        // So, we're going to be following these rules:
        // - Use at least 126 reader threads
        // - Add 16 extra reader threads if <126
        //
        // FIXME: This behavior is from `monerod`:
        // <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L1324>
        // I believe this could be adjusted percentage-wise so very high
        // thread PCs can benefit from something like (cuprated + anything that uses the DB in the future).
        // For now:
        // - No other program using our DB exists
        // - Almost no-one has a 126+ thread CPU
        let reader_threads =
            u32::try_from(config.reader_threads.as_threads().get()).unwrap_or(u32::MAX);
        env_open_options.max_readers(if reader_threads < 110 {
            126
        } else {
            reader_threads.saturating_add(16)
        });

        // Create the database directory if it doesn't exist.
        std::fs::create_dir_all(&config.data_dir)?;
        // Open the environment in the user's PATH.
        // SAFETY: LMDB uses a memory-map backed file.
        // <https://docs.rs/heed/0.20.0/heed/struct.EnvOpenOptions.html#method.open>
        unsafe { env_open_options.open(&config.data_dir)? }
    };

    {
        let mut rw_tx = env.write_txn()?;

        BLOCK_HEIGHTS
            .set(
                env.database_options()
                    .name("BLOCK_HEIGHTS")
                    .types()
                    .create(&mut rw_tx)?,
            )
            .unwrap();
        KEY_IMAGES
            .set(
                env.database_options()
                    .name("KEY_IMAGES")
                    .types()
                    .flags(DatabaseFlags::DUP_SORT | DatabaseFlags::DUP_FIXED)
                    .create(&mut rw_tx)?,
            )
            .unwrap();
        PRE_RCT_OUTPUTS
            .set(
                env.database_options()
                    .name("PRE_RCT_OUTPUTS")
                    .types()
                    .key_comparator()
                    .dup_sort_comparator()
                    .flags(DatabaseFlags::DUP_SORT | DatabaseFlags::DUP_FIXED)
                    .create(&mut rw_tx)?,
            )
            .unwrap();
        TX_IDS
            .set(
                env.database_options()
                    .name("TX_IDS")
                    .types()
                    .create(&mut rw_tx)?,
            )
            .unwrap();
        TX_OUTPUTS
            .set(
                env.database_options()
                    .name("TX_OUTPUTS")
                    .types()
                    .key_comparator()
                    .create(&mut rw_tx)?,
            )
            .unwrap();
        ALT_CHAIN_INFOS
            .set(
                env.database_options()
                    .name("ALT_CHAIN_INFOS")
                    .types()
                    .create(&mut rw_tx)?,
            )
            .unwrap();
        ALT_BLOCK_HEIGHTS
            .set(
                env.database_options()
                    .name("ALT_BLOCK_HEIGHTS")
                    .types()
                    .create(&mut rw_tx)?,
            )
            .unwrap();
        ALT_BLOCKS_INFO
            .set(
                env.database_options()
                    .name("ALT_BLOCKS_INFO")
                    .types()
                    .create(&mut rw_tx)?,
            )
            .unwrap();
        ALT_BLOCK_BLOBS
            .set(
                env.database_options()
                    .name("ALT_BLOCK_BLOBS")
                    .types()
                    .create(&mut rw_tx)?,
            )
            .unwrap();
        ALT_TRANSACTION_BLOBS
            .set(
                env.database_options()
                    .name("ALT_TRANSACTION_BLOBS")
                    .types()
                    .create(&mut rw_tx)?,
            )
            .unwrap();
        ALT_TRANSACTION_INFOS
            .set(
                env.database_options()
                    .name("ALT_TRANSACTION_INFOS")
                    .types()
                    .create(&mut rw_tx)?,
            )
            .unwrap();

        rw_tx.commit()?;
    }

    let tapes = linear_tapes_config(config.blob_data_dir);

    let linear_tapes = unsafe { LinearTapes::new(tapes, config.data_dir, 1024 * 1204 * 1024)? };

    tracing::debug!("opened db");
    Ok(Blockchain {
        dynamic_tables: env,
        linear_tapes,
    })
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
