//! Implementation of `trait Env` for `heed`.

//---------------------------------------------------------------------------------------------------- Import
use std::sync::RwLock;

use crate::{
    backend::heed::database::{HeedTableRo, HeedTableRw},
    // backend::heed::types::HeedDb,
    config::Config,
    database::{DatabaseRead, DatabaseWrite},
    env::Env,
    error::{InitError, RuntimeError},
    resize::ResizeAlgorithm,
    table::Table,
};

//---------------------------------------------------------------------------------------------------- Env
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
    config: Config,
}

impl Drop for ConcreteEnv {
    fn drop(&mut self) {
        if let Err(e) = self.sync() {
            // TODO: log error?
        }

        // TODO: log that we are dropping the database.
    }
}

//---------------------------------------------------------------------------------------------------- Env Impl
impl Env for ConcreteEnv {
    const MANUAL_RESIZE: bool = true;
    const SYNCS_PER_TX: bool = false;
    type TxRo<'db> = heed::RoTxn<'db>;
    type TxRw<'db> = heed::RwTxn<'db>;

    #[cold]
    #[inline(never)] // called once.
    fn open(config: Config) -> Result<Self, InitError> {
        // INVARIANT:
        // We must open LMDB using `heed::EnvOpenOptions::max_readers`
        // and input whatever is in `config.reader_threads` or else
        // LMDB will start throwing errors if there are >126 readers.
        // <http://www.lmdb.tech/doc/group__mdb.html#gae687966c24b790630be2a41573fe40e2>
        //
        // We should also leave reader slots for other processes, e.g. `xmrblocks`.
        // <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L1372>

        // <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L1324>
        todo!()
    }

    fn config(&self) -> &Config {
        &self.config
    }

    fn sync(&self) -> Result<(), RuntimeError> {
        todo!()
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

    fn current_map_size(&self) -> usize {
        self.env.read().unwrap().info().map_size
    }

    #[inline]
    fn ro_tx(&self) -> Result<Self::TxRo<'_>, RuntimeError> {
        todo!()
    }

    #[inline]
    fn rw_tx(&self) -> Result<Self::TxRw<'_>, RuntimeError> {
        todo!()
    }

    #[cold]
    #[inline(never)] // called infrequently?.
    fn create_tables_if_needed<T: Table>(
        &self,
        tx_write: &mut Self::TxRw<'_>,
    ) -> Result<(), RuntimeError> {
        todo!()
    }

    #[inline]
    fn open_db_read<T: Table>(
        &self,
        tx_read: &Self::TxRo<'_>,
    ) -> Result<impl DatabaseRead<T>, RuntimeError> {
        let tx: HeedTableRo<T> = todo!();
        Ok(tx)
    }

    #[inline]
    fn open_db_write<T: Table>(
        &self,
        tx_write: &Self::TxRw<'_>,
    ) -> Result<impl DatabaseWrite<T>, RuntimeError> {
        let tx: HeedTableRw<T> = todo!();
        Ok(tx)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
