//! Implementation of `trait Env` for `heed`.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    fmt::Debug,
    ops::Deref,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use heed::{DatabaseOpenOptions, EnvFlags, EnvOpenOptions};

use crate::{
    backend::heed::{
        database::{HeedTableRo, HeedTableRw},
        storable::StorableHeed,
        types::HeedDb,
    },
    config::{Config, SyncMode},
    database::{DatabaseRo, DatabaseRw},
    env::{Env, EnvInner},
    error::{InitError, RuntimeError},
    resize::ResizeAlgorithm,
    table::Table,
};

//---------------------------------------------------------------------------------------------------- Consts
/// TODO
const PANIC_MSG_MISSING_TABLE: &str =
    "cuprate_database::Env should uphold the invariant that all tables are already created";

//---------------------------------------------------------------------------------------------------- ConcreteEnv
/// A strongly typed, concrete database environment, backed by `heed`.
pub struct ConcreteEnv {
    /// The actual database environment.
    ///
    /// # Why `RwLock`?
    /// We need mutual exclusive access to the environment for resizing.
    ///
    /// Using 2 atomics for mutual exclusion was considered:
    /// - `currently_resizing: AtomicBool`
    /// - `reader_count: AtomicUsize`
    ///
    /// This is how `monerod` does it:
    /// <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L354-L355>
    ///
    /// `currently_resizing` would be set to `true` on resizes and
    /// `reader_count` would be spinned on until 0, at which point
    /// we are safe to resize.
    ///
    /// Although, 3 atomic operations (check atomic bool, reader_count++, reader_count--)
    /// turns out to be roughly as expensive as acquiring a non-contended `RwLock`,
    /// the CPU sleeping instead of spinning is much better too.
    ///
    /// # `unwrap()`
    /// This will be [`unwrap()`]ed everywhere.
    ///
    /// If lock is poisoned, we want all of Cuprate to panic.
    env: RwLock<heed::Env>,

    /// The configuration we were opened with
    /// (and in current use).
    pub(super) config: Config,
}

impl Drop for ConcreteEnv {
    fn drop(&mut self) {
        // INVARIANT: drop(ConcreteEnv) must sync.
        //
        // TODO:
        // "if the environment has the MDB_NOSYNC flag set the flushes will be omitted,
        // and with MDB_MAPASYNC they will be asynchronous."
        // <http://www.lmdb.tech/doc/group__mdb.html#ga85e61f05aa68b520cc6c3b981dba5037>
        //
        // We need to do `mdb_env_set_flags(&env, MDB_NOSYNC|MDB_ASYNCMAP, 0)`
        // to clear the no sync and async flags such that the below `self.sync()`
        // _actually_ synchronously syncs.
        if let Err(e) = crate::Env::sync(self) {
            // TODO: log error?
        }

        // TODO: log that we are dropping the database.

        // TODO: use tracing.
        // <https://github.com/LMDB/lmdb/blob/b8e54b4c31378932b69f1298972de54a565185b1/libraries/liblmdb/lmdb.h#L49-L61>
        let result = self.env.read().unwrap().clear_stale_readers();
        match result {
            Ok(n) => println!("LMDB stale readers cleared: {n}"),
            Err(e) => println!("LMDB stale reader clear error: {e:?}"),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Env Impl
impl Env for ConcreteEnv {
    const MANUAL_RESIZE: bool = true;
    const SYNCS_PER_TX: bool = false;
    type EnvInner<'env> = RwLockReadGuard<'env, heed::Env>;
    type TxRo<'tx> = heed::RoTxn<'tx>;
    type TxRw<'tx> = heed::RwTxn<'tx>;

    #[cold]
    #[inline(never)] // called once.
    #[allow(clippy::items_after_statements)]
    fn open(config: Config) -> Result<Self, InitError> {
        // <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L1324>

        // Map our `Config` sync mode to the LMDB environment flags.
        //
        // <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L1324>
        let flags = match config.sync_mode {
            SyncMode::Safe => EnvFlags::empty(),
            SyncMode::Async => EnvFlags::MAP_ASYNC,
            SyncMode::Fast => EnvFlags::NO_SYNC | EnvFlags::WRITE_MAP | EnvFlags::MAP_ASYNC,
            // TODO: dynamic syncs are not implemented.
            SyncMode::FastThenSafe | SyncMode::Threshold(_) => unimplemented!(),
        };

        let mut env_open_options = EnvOpenOptions::new();

        // Set the memory map size to
        // (current disk size) + (a bit of leeway)
        // to account for empty databases where we
        // need to write same tables.
        #[allow(clippy::cast_possible_truncation)] // only 64-bit targets
        let disk_size_bytes = match std::fs::File::open(&config.db_file) {
            Ok(file) => file.metadata()?.len() as usize,
            // The database file doesn't exist, 0 bytes.
            Err(io_err) if io_err.kind() == std::io::ErrorKind::NotFound => 0,
            Err(io_err) => return Err(io_err.into()),
        };
        // Add leeway space.
        let memory_map_size = crate::resize::fixed_bytes(disk_size_bytes, 1_000_000 /* 1MB */);
        env_open_options.map_size(memory_map_size.get());

        // Set the max amount of database tables.
        // We know at compile time how many tables there are.
        // TODO: ...how many?
        env_open_options.max_dbs(32);

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
        // TODO: This behavior is from `monerod`:
        // <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L1324>
        // I believe this could be adjusted percentage-wise so very high
        // thread PCs can benefit from something like (cuprated + anything that uses the DB in the future).
        // For now:
        // - No other program using our DB exists
        // - Almost no-one has a 126+ thread CPU
        #[allow(clippy::cast_possible_truncation)] // no-one has `u32::MAX`+ threads
        let reader_threads = config.reader_threads.as_threads().get() as u32;
        env_open_options.max_readers(if reader_threads < 110 {
            126
        } else {
            reader_threads + 16
        });

        // Create the database directory if it doesn't exist.
        std::fs::create_dir_all(config.db_directory())?;
        // Open the environment in the user's PATH.
        let env = env_open_options.open(config.db_directory())?;

        // TODO: Open/create tables with certain flags
        // <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L1324>
        // `heed` creates the database if it didn't exist.
        // <https://docs.rs/heed/0.20.0-alpha.9/src/heed/env.rs.html#223-229>

        /// Function that creates the tables based off the passed `T: Table`.
        fn create_table<T: Table>(
            env: &heed::Env,
            tx_rw: &mut heed::RwTxn<'_>,
        ) -> Result<(), InitError> {
            println!("create_table(): {}", T::NAME); // TODO: use tracing.

            DatabaseOpenOptions::new(env)
                .name(<T as Table>::NAME)
                .types::<StorableHeed<<T as Table>::Key>, StorableHeed<<T as Table>::Value>>()
                .create(tx_rw)?;
            Ok(())
        }

        use crate::tables::{
            BlockBlobs, BlockHeights, BlockInfoV1s, BlockInfoV2s, BlockInfoV3s, KeyImages,
            NumOutputs, Outputs, PrunableHashes, PrunableTxBlobs, PrunedTxBlobs, RctOutputs,
            TxHeights, TxIds, TxUnlockTime,
        };

        let mut tx_rw = env.write_txn()?;
        create_table::<BlockBlobs>(&env, &mut tx_rw)?;
        create_table::<BlockHeights>(&env, &mut tx_rw)?;
        create_table::<BlockInfoV1s>(&env, &mut tx_rw)?;
        create_table::<BlockInfoV2s>(&env, &mut tx_rw)?;
        create_table::<BlockInfoV3s>(&env, &mut tx_rw)?;
        create_table::<KeyImages>(&env, &mut tx_rw)?;
        create_table::<NumOutputs>(&env, &mut tx_rw)?;
        create_table::<Outputs>(&env, &mut tx_rw)?;
        create_table::<PrunableHashes>(&env, &mut tx_rw)?;
        create_table::<PrunableTxBlobs>(&env, &mut tx_rw)?;
        create_table::<PrunedTxBlobs>(&env, &mut tx_rw)?;
        create_table::<RctOutputs>(&env, &mut tx_rw)?;
        create_table::<TxHeights>(&env, &mut tx_rw)?;
        create_table::<TxIds>(&env, &mut tx_rw)?;
        create_table::<TxUnlockTime>(&env, &mut tx_rw)?;

        // TODO: Set dupsort and comparison functions for certain tables
        // <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L1324>

        // INVARIANT: this should never return `ResizeNeeded` due to adding
        // some tables since we added some leeway to the memory map above.
        tx_rw.commit()?;

        Ok(Self {
            env: RwLock::new(env),
            config,
        })
    }

    fn config(&self) -> &Config {
        &self.config
    }

    fn sync(&self) -> Result<(), RuntimeError> {
        Ok(self.env.read().unwrap().force_sync()?)
    }

    fn resize_map(&self, resize_algorithm: Option<ResizeAlgorithm>) {
        let resize_algorithm = resize_algorithm.unwrap_or_else(|| self.config().resize_algorithm);

        let current_size_bytes = self.current_map_size();
        let new_size_bytes = resize_algorithm.resize(current_size_bytes).get();

        // SAFETY:
        // Resizing requires that we have
        // exclusive access to the database environment.
        // Our `heed::Env` is wrapped within a `RwLock`,
        // and we have a WriteGuard to it, so we're safe.
        //
        // <http://www.lmdb.tech/doc/group__mdb.html#gaa2506ec8dab3d969b0e609cd82e619e5>
        unsafe {
            // INVARIANT: `resize()` returns a valid `usize` to resize to.
            self.env.write().unwrap().resize(new_size_bytes).unwrap();
        }
    }

    #[inline]
    fn current_map_size(&self) -> usize {
        self.env.read().unwrap().info().map_size
    }

    #[inline]
    fn env_inner(&self) -> Self::EnvInner<'_> {
        self.env.read().unwrap()
    }
}

//---------------------------------------------------------------------------------------------------- EnvInner Impl
impl<'env> EnvInner<'env, heed::RoTxn<'env>, heed::RwTxn<'env>> for RwLockReadGuard<'env, heed::Env>
where
    Self: 'env,
{
    #[inline]
    fn tx_ro(&'env self) -> Result<heed::RoTxn<'env>, RuntimeError> {
        Ok(self.read_txn()?)
    }

    #[inline]
    fn tx_rw(&'env self) -> Result<heed::RwTxn<'env>, RuntimeError> {
        Ok(self.write_txn()?)
    }

    #[inline]
    fn open_db_ro<T: Table>(
        &self,
        tx_ro: &heed::RoTxn<'env>,
    ) -> Result<impl DatabaseRo<T>, RuntimeError> {
        // Open up a read-only database using our table's const metadata.
        Ok(HeedTableRo {
            db: self
                .open_database(tx_ro, Some(T::NAME))?
                .expect(PANIC_MSG_MISSING_TABLE),
            tx_ro,
        })
    }

    #[inline]
    fn open_db_rw<'tx, T: Table>(
        &self,
        tx_rw: &'tx mut heed::RwTxn<'env>,
    ) -> Result<impl DatabaseRw<'env, 'tx, T>, RuntimeError> {
        // Open up a read/write database using our table's const metadata.
        Ok(HeedTableRw {
            db: self
                .open_database(tx_rw, Some(T::NAME))?
                .expect(PANIC_MSG_MISSING_TABLE),
            tx_rw,
        })
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
