//! Abstracted database environment; `trait Env`.

//---------------------------------------------------------------------------------------------------- Import
use std::num::NonZeroUsize;

use crate::{
    config::Config,
    database::{DatabaseIter, DatabaseRo, DatabaseRw},
    error::{DbResult, InitError},
    resize::ResizeAlgorithm,
    table::Table,
    transaction::{TxRo, TxRw},
};

//---------------------------------------------------------------------------------------------------- Env
/// Database environment abstraction.
///
/// Essentially, the functions that can be called on [`ConcreteEnv`](crate::ConcreteEnv).
///
/// # `Drop`
/// Objects that implement [`Env`] _should_ probably
/// [`Env::sync`] in their drop implementations,
/// although, no invariant relies on this (yet).
///
/// # Lifetimes
/// The lifetimes associated with `Env` have a sequential flow:
/// ```text
/// Env -> Tx -> Database
/// ```
///
/// As in:
/// - open database tables only live as long as...
/// - transactions which only live as long as the...
/// - database environment
pub trait Env: Sized {
    //------------------------------------------------ Constants
    /// Does the database backend need to be manually
    /// resized when the memory-map is full?
    ///
    /// # Invariant
    /// If this is `false`, that means this [`Env`]
    /// must _never_ return a [`crate::RuntimeError::ResizeNeeded`].
    ///
    /// If this is `true`, [`Env::resize_map`] & [`Env::current_map_size`]
    /// _must_ be re-implemented, as it just panics by default.
    const MANUAL_RESIZE: bool;

    /// Does the database backend forcefully sync/flush
    /// to disk on every transaction commit?
    ///
    /// This is used as an optimization.
    const SYNCS_PER_TX: bool;

    //------------------------------------------------ Types
    /// The struct representing the actual backend's database environment.
    ///
    /// This is used as the `self` in [`EnvInner`] functions, so whatever
    /// this type is, is what will be accessible from those functions.
    ///
    // # HACK
    // For `heed`, this is just `heed::Env`, for `redb` this is
    // `(redb::Database, redb::Durability)` as each transaction
    // needs the sync mode set during creation.
    type EnvInner<'env>: EnvInner<'env>
    where
        Self: 'env;

    /// The read-only transaction type of the backend.
    type TxRo<'env>: TxRo<'env>
    where
        Self: 'env;

    /// The read/write transaction type of the backend.
    type TxRw<'env>: TxRw<'env>
    where
        Self: 'env;

    //------------------------------------------------ Required
    /// Open the database environment, using the passed [`Config`].
    ///
    /// # Invariants
    /// This function does not create any tables.
    ///
    /// You must create all possible tables with [`EnvInner::create_db`]
    /// before attempting to open any.
    ///
    /// # Errors
    /// This will error if the database file could not be opened.
    ///
    /// This is the only [`Env`] function that will return
    /// an [`InitError`] instead of a [`crate::RuntimeError`].
    fn open(config: Config) -> Result<Self, InitError>;

    /// Return the [`Config`] that this database was [`Env::open`]ed with.
    fn config(&self) -> &Config;

    /// Fully sync the database caches to disk.
    ///
    /// # Invariant
    /// This must **fully** and **synchronously** flush the database data to disk.
    ///
    /// I.e., after this function returns, there must be no doubts
    /// that the data isn't synced yet, it _must_ be synced.
    ///
    // FIXME: either this invariant or `sync()` itself will most
    // likely be removed/changed after `SyncMode` is finalized.
    ///
    /// # Errors
    /// If there is a synchronization error, this should return an error.
    fn sync(&self) -> DbResult<()>;

    /// Resize the database's memory map to a
    /// new (bigger) size using a [`ResizeAlgorithm`].
    ///
    /// By default, this function will use the `ResizeAlgorithm` in [`Env::config`].
    ///
    /// - If `resized_by_another_process` is `true`, `0` will
    ///   be passed to the internal resize function, see:
    //    <http://www.lmdb.tech/doc/group__mdb.html#gad7ea55da06b77513609efebd44b26920>
    /// - If `resize_algorithm` is `Some`, that will be used instead
    /// - This function returns the _new_ memory map size in bytes
    ///
    /// # Invariant
    /// This function _must_ be re-implemented if [`Env::MANUAL_RESIZE`] is `true`.
    ///
    /// Otherwise, this function will panic with `unreachable!()`.
    #[expect(unused_variables)]
    fn resize_map(
        &self,
        resize_algorithm: Option<ResizeAlgorithm>,
        resized_by_another_process: bool,
    ) -> NonZeroUsize {
        unreachable!()
    }

    /// What is the _current_ size of the database's memory map in bytes?
    ///
    /// # Invariant
    /// 1. This function _must_ be re-implemented if [`Env::MANUAL_RESIZE`] is `true`.
    /// 2. This function must be accurate, as [`Env::resize_map()`] may depend on it.
    fn current_map_size(&self) -> usize {
        unreachable!()
    }

    /// Return the [`Env::EnvInner`].
    ///
    /// # Locking behavior
    /// When using the `heed` backend, [`Env::EnvInner`] is a
    /// `RwLockReadGuard`, i.e., calling this function takes a
    /// read lock on the `heed::Env`.
    ///
    /// Be aware of this, as other functions (currently only
    /// [`Env::resize_map`]) will take a _write_ lock.
    fn env_inner(&self) -> Self::EnvInner<'_>;

    //------------------------------------------------ Provided
    /// Return the amount of actual of bytes the database is taking up on disk.
    ///
    /// This is the current _disk_ value in bytes, not the memory map.
    ///
    /// # Errors
    /// This will error if either:
    ///
    /// - [`std::fs::File::open`]
    /// - [`std::fs::File::metadata`]
    ///
    /// failed on the database file on disk.
    fn disk_size_bytes(&self) -> std::io::Result<u64> {
        // We have the direct PATH to the file,
        // no need to use backend-specific functions.
        //
        // INVARIANT: as we are only accessing the metadata of
        // the file and not reading the bytes, it should be
        // fine even with a memory mapped file being actively
        // written to.
        Ok(std::fs::File::open(&self.config().db_file)?
            .metadata()?
            .len())
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRo
/// Document the INVARIANT that the `heed` backend
/// must use [`EnvInner::create_db`] when initially
/// opening/creating tables.
macro_rules! doc_heed_create_db_invariant {
    () => {
        r"The first time you open/create tables, you _must_ use [`EnvInner::create_db`]
to set the proper flags / [`Key`](crate::Key) comparison for the `heed` backend.

Subsequent table opens will follow the flags/ordering, but only if
[`EnvInner::create_db`] was the _first_ function to open/create it."
    };
}

/// The inner [`Env`] type.
///
/// This type is created with [`Env::env_inner`] and represents
/// the type able to generate transactions and open tables.
///
/// # Locking behavior
/// As noted in `Env::env_inner`, this is a `RwLockReadGuard`
/// when using the `heed` backend, be aware of this and do
/// not hold onto an `EnvInner` for a long time.
///
/// # Tables
/// Note that when opening tables with [`EnvInner::open_db_ro`],
/// they must be created first or else it will return error.
///
/// See [`EnvInner::create_db`] for creating tables.
///
/// # Invariant
#[doc = doc_heed_create_db_invariant!()]
pub trait EnvInner<'env> {
    /// The read-only transaction type of the backend.
    ///
    /// `'tx` is the lifetime of the transaction itself.
    type Ro<'tx>: TxRo<'tx>;
    /// The read-write transaction type of the backend.
    ///
    /// `'tx` is the lifetime of the transaction itself.
    type Rw<'tx>: TxRw<'tx>;

    /// Create a read-only transaction.
    ///
    /// # Errors
    /// This will only return [`crate::RuntimeError::Io`] if it errors.
    fn tx_ro(&self) -> DbResult<Self::Ro<'_>>;

    /// Create a read/write transaction.
    ///
    /// # Errors
    /// This will only return [`crate::RuntimeError::Io`] if it errors.
    fn tx_rw(&self) -> DbResult<Self::Rw<'_>>;

    /// Open a database in read-only mode.
    ///
    /// The returned value can have [`DatabaseRo`]
    /// & [`DatabaseIter`] functions called on it.
    ///
    /// This will open the database [`Table`]
    /// passed as a generic to this function.
    ///
    /// ```rust
    /// # use cuprate_database::{
    /// #     ConcreteEnv,
    /// #     config::ConfigBuilder,
    /// #     Env, EnvInner,
    /// #     DatabaseRo, DatabaseRw, TxRo, TxRw,
    /// # };
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let tmp_dir = tempfile::tempdir()?;
    /// # let db_dir = tmp_dir.path().to_owned();
    /// # let config = ConfigBuilder::new(db_dir.into()).build();
    /// # let env = ConcreteEnv::open(config)?;
    /// #
    /// # struct Table;
    /// # impl cuprate_database::Table for Table {
    /// #     const NAME: &'static str = "table";
    /// #     type Key = u8;
    /// #     type Value = u64;
    /// # }
    /// #
    /// # let env_inner = env.env_inner();
    /// # let tx_rw = env_inner.tx_rw()?;
    /// # env_inner.create_db::<Table>(&tx_rw)?;
    /// # TxRw::commit(tx_rw);
    /// #
    /// # let tx_ro = env_inner.tx_ro()?;
    /// let db = env_inner.open_db_ro::<Table>(&tx_ro);
    /// //  ^                           ^
    /// // database table             table metadata
    /// //                            (name, key/value type)
    /// # Ok(()) }
    /// ```
    ///
    /// # Errors
    /// This will only return [`crate::RuntimeError::Io`] on normal errors.
    ///
    /// If the specified table is not created upon before this function is called,
    /// this will return [`crate::RuntimeError::TableNotFound`].
    ///
    /// # Invariant
    #[doc = doc_heed_create_db_invariant!()]
    fn open_db_ro<T: Table>(
        &self,
        tx_ro: &Self::Ro<'_>,
    ) -> DbResult<impl DatabaseRo<T> + DatabaseIter<T>>;

    /// Open a database in read/write mode.
    ///
    /// All [`DatabaseRo`] functions are also callable
    /// with the returned [`DatabaseRw`] structure.
    ///
    /// Note that [`DatabaseIter`] functions are _not_
    /// available to [`DatabaseRw`] structures.
    ///
    /// This will open the database [`Table`]
    /// passed as a generic to this function.
    ///
    /// # Errors
    /// This will only return [`crate::RuntimeError::Io`] on errors.
    ///
    /// # Invariant
    #[doc = doc_heed_create_db_invariant!()]
    fn open_db_rw<T: Table>(&self, tx_rw: &Self::Rw<'_>) -> DbResult<impl DatabaseRw<T>>;

    /// Create a database table.
    ///
    /// This will create the database [`Table`] passed as a generic to this function.
    ///
    /// # Errors
    /// This will only return [`crate::RuntimeError::Io`] on errors.
    ///
    /// # Invariant
    #[doc = doc_heed_create_db_invariant!()]
    fn create_db<T: Table>(&self, tx_rw: &Self::Rw<'_>) -> DbResult<()>;

    /// Clear all `(key, value)`'s from a database table.
    ///
    /// This will delete all key and values in the passed
    /// `T: Table`, but the table itself will continue to exist.
    ///
    /// Note that this operation is tied to `tx_rw`, as such this
    /// function's effects can be aborted using [`TxRw::abort`].
    ///
    /// # Errors
    /// This will return [`crate::RuntimeError::Io`] on normal errors.
    ///
    /// If the specified table is not created upon before this function is called,
    /// this will return [`crate::RuntimeError::TableNotFound`].
    fn clear_db<T: Table>(&self, tx_rw: &mut Self::Rw<'_>) -> DbResult<()>;
}
