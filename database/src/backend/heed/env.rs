//! Implementation of `trait Env` for `heed`.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    ops::Deref,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use heed::{DatabaseOpenOptions, EnvFlags, EnvOpenOptions};

use crate::{
    backend::heed::{
        database::{HeedTableRo, HeedTableRw},
        types::HeedDb,
    },
    config::{Config, SyncMode},
    database::{DatabaseRo, DatabaseRw},
    env::Env,
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
    type EnvInner = heed::Env;
    type TxRoInput = heed::Env;
    type TxRwInput = heed::Env;
    type TxRo<'env> = heed::RoTxn<'env>;
    type TxRw<'env> = heed::RwTxn<'env>;

    #[cold]
    #[inline(never)] // called once.
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

        // Set the memory map size to at least the current disk size.
        let disk_size_bytes = std::fs::File::open(&config.db_file)?.metadata()?.len();
        #[allow(clippy::cast_possible_truncation)] // only 64-bit targets
        env_open_options.map_size(disk_size_bytes as usize);

        // Set the max amount of database tables.
        // We know at compile time how many tables there are.
        // TODO: ...how many?
        env_open_options.max_dbs(todo!());

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

        // TODO: Open/create tables with certain flags
        // <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L1324>
        // `heed` creates the database if it didn't exist.
        // <https://docs.rs/heed/0.20.0-alpha.9/src/heed/env.rs.html#223-229>

        // TODO: Set dupsort and comparison functions for certain tables
        // <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L1324>

        todo!()
    }

    fn create_tables(
        &self,
        env: &Self::EnvInner,
        tx_rw: &mut Self::TxRw<'_>,
    ) -> Result<(), RuntimeError> {
        use crate::tables::{TestTable, TestTable2};

        // These wonderful fully qualified types are
        // brought to you by trait collisions.

        DatabaseOpenOptions::new(env)
            .name(TestTable::NAME)
            .types::<<TestTable as Table>::Key, <TestTable as Table>::Value>()
            .create(tx_rw)?;

        DatabaseOpenOptions::new(env)
            .name(TestTable::NAME)
            .types::<<TestTable2 as Table>::Key, <TestTable2 as Table>::Value>()
            .create(tx_rw)?;

        todo!()
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
        let new_size_bytes = resize_algorithm.resize(current_size_bytes);

        // SAFETY:
        // Resizing requires that we have
        // exclusive access to the database environment.
        // Our `heed::Env` is wrapped within a `RwLock`,
        // and we have a WriteGuard to it, so we're safe.
        //
        // <http://www.lmdb.tech/doc/group__mdb.html#gaa2506ec8dab3d969b0e609cd82e619e5>
        unsafe {
            // INVARIANT: `resize()` returns a valid `usize` to resize to.
            self.env
                .write()
                .unwrap()
                .resize(new_size_bytes.get())
                .unwrap();
        }
    }

    #[inline]
    fn current_map_size(&self) -> usize {
        self.env.read().unwrap().info().map_size
    }

    #[inline]
    fn env_inner(&self) -> impl Deref<Target = Self::EnvInner> {
        self.env.read().unwrap()
    }

    #[inline]
    fn tx_ro_input(&self) -> impl Deref<Target = Self::TxRoInput> {
        self.env.read().unwrap()
    }

    #[inline]
    fn tx_rw_input(&self) -> impl Deref<Target = Self::TxRwInput> {
        self.env.read().unwrap()
    }

    #[inline]
    fn tx_ro(env: &Self::TxRoInput) -> Result<Self::TxRo<'_>, RuntimeError> {
        Ok(env.read_txn()?)
    }

    #[inline]
    fn tx_rw(env: &Self::TxRwInput) -> Result<Self::TxRw<'_>, RuntimeError> {
        Ok(env.write_txn()?)
    }

    #[inline]
    fn open_db_ro<'tx, T: Table>(
        env: &Self::EnvInner,
        tx_ro: &'tx Self::TxRo<'tx>,
    ) -> Result<impl DatabaseRo<'tx, T>, RuntimeError> {
        // Open up a read-only database using our table's const metadata.
        //
        // The actual underlying type `heed` sees is
        // something similar to `key: [u8], value: [u8]`.
        // See: `crate::backend::heed::{types, storable}` for more detail.
        //
        // With that said, we are still type safe as we are
        // passing around and using `<T: Table>`'s metadata
        // as the types, rather than raw bytes. This gets
        // extended to the table/database type as well,
        // as that also has `T: Table`.
        #[allow(clippy::type_complexity)]
        let result: Result<std::option::Option<HeedDb<T::Key, T::Value>>, heed::Error> =
            env.open_database(tx_ro, Some(T::NAME));

        match result {
            Ok(Some(db)) => Ok(HeedTableRo { db, tx_ro }),
            Err(e) => Err(e.into()),

            // INVARIANT: Every table should be created already.
            Ok(None) => panic!("{PANIC_MSG_MISSING_TABLE}"),
        }
    }

    #[inline]
    fn open_db_rw<'tx, T: Table>(
        env: &Self::EnvInner,
        tx_rw: &'tx mut Self::TxRw<'tx>,
    ) -> Result<impl DatabaseRw<'tx, T>, RuntimeError> {
        // Open up a read/write database using our table's const metadata.
        //
        // Everything said above with `open_db_ro()` applies here as well.
        #[allow(clippy::type_complexity)]
        let result: Result<std::option::Option<HeedDb<T::Key, T::Value>>, heed::Error> =
            env.open_database(tx_rw, Some(T::NAME));

        match result {
            Ok(Some(db)) => Ok(HeedTableRw { db, tx_rw }),
            Err(e) => Err(e.into()),

            // INVARIANT: Every table should be created already.
            Ok(None) => panic!("{PANIC_MSG_MISSING_TABLE}"),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
