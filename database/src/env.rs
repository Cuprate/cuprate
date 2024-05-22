//! Abstracted database environment; `trait Env`.

//---------------------------------------------------------------------------------------------------- Import
use std::num::NonZeroUsize;

use crate::{
    config::Config,
    database::{DatabaseIter, DatabaseRo, DatabaseRw},
    error::{InitError, RuntimeError},
    resize::ResizeAlgorithm,
    table::Table,
    tables::{call_fn_on_all_tables_or_early_return, TablesIter, TablesMut},
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
/// 1. `ConcreteEnv`
/// 2. `'env`
/// 3. `'tx`
/// 4. `'db`
///
/// As in:
/// - open database tables only live as long as...
/// - transactions which only live as long as the...
/// - environment ([`EnvInner`])
pub trait Env: Sized {
    //------------------------------------------------ Constants
    /// Does the database backend need to be manually
    /// resized when the memory-map is full?
    ///
    /// # Invariant
    /// If this is `false`, that means this [`Env`]
    /// must _never_ return a [`RuntimeError::ResizeNeeded`].
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
    type EnvInner<'env>: EnvInner<'env, Self::TxRo<'env>, Self::TxRw<'env>>
    where
        Self: 'env;

    /// The read-only transaction type of the backend.
    type TxRo<'env>: TxRo<'env> + 'env
    where
        Self: 'env;

    /// The read/write transaction type of the backend.
    type TxRw<'env>: TxRw<'env> + 'env
    where
        Self: 'env;

    //------------------------------------------------ Required
    /// Open the database environment, using the passed [`Config`].
    ///
    /// # Invariants
    /// This function **must** create all tables listed in [`crate::tables`].
    ///
    /// The rest of the functions depend on the fact
    /// they already exist, or else they will panic.
    ///
    /// # Errors
    /// This will error if the database could not be opened.
    ///
    /// This is the only [`Env`] function that will return
    /// an [`InitError`] instead of a [`RuntimeError`].
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
    fn sync(&self) -> Result<(), RuntimeError>;

    /// Resize the database's memory map to a
    /// new (bigger) size using a [`ResizeAlgorithm`].
    ///
    /// By default, this function will use the `ResizeAlgorithm` in [`Env::config`].
    ///
    /// If `resize_algorithm` is `Some`, that will be used instead.
    ///
    /// This function returns the _new_ memory map size in bytes.
    ///
    /// # Invariant
    /// This function _must_ be re-implemented if [`Env::MANUAL_RESIZE`] is `true`.
    ///
    /// Otherwise, this function will panic with `unreachable!()`.
    #[allow(unused_variables)]
    fn resize_map(&self, resize_algorithm: Option<ResizeAlgorithm>) -> NonZeroUsize {
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
        // SAFETY: as we are only accessing the metadata of
        // the file and not reading the bytes, it should be
        // fine even with a memory mapped file being actively
        // written to.
        Ok(std::fs::File::open(&self.config().db_file)?
            .metadata()?
            .len())
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRo
/// Document errors when opening tables in [`EnvInner`].
macro_rules! doc_table_error {
    () => {
        r"# Errors
This will only return [`RuntimeError::Io`] if it errors.

As all tables are created upon [`Env::open`],
this function will never error because a table doesn't exist."
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
pub trait EnvInner<'env, Ro, Rw>
where
    Self: 'env,
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
{
    /// Create a read-only transaction.
    ///
    /// # Errors
    /// This will only return [`RuntimeError::Io`] if it errors.
    fn tx_ro(&'env self) -> Result<Ro, RuntimeError>;

    /// Create a read/write transaction.
    ///
    /// # Errors
    /// This will only return [`RuntimeError::Io`] if it errors.
    fn tx_rw(&'env self) -> Result<Rw, RuntimeError>;

    /// Open a database in read-only mode.
    ///
    /// The returned value can have [`DatabaseRo`]
    /// & [`DatabaseIter`] functions called on it.
    ///
    /// This will open the database [`Table`]
    /// passed as a generic to this function.
    ///
    /// ```rust,ignore
    /// let db = env.open_db_ro::<Table>(&tx_ro);
    /// //  ^                     ^
    /// // database table       table metadata
    /// //                      (name, key/value type)
    /// ```
    ///
    #[doc = doc_table_error!()]
    fn open_db_ro<T: Table>(
        &self,
        tx_ro: &Ro,
    ) -> Result<impl DatabaseRo<T> + DatabaseIter<T>, RuntimeError>;

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
    #[doc = doc_table_error!()]
    fn open_db_rw<T: Table>(&self, tx_rw: &Rw) -> Result<impl DatabaseRw<T>, RuntimeError>;

    /// Open all tables in read/iter mode.
    ///
    /// This calls [`EnvInner::open_db_ro`] on all database tables
    /// and returns a structure that allows access to all tables.
    ///
    #[doc = doc_table_error!()]
    fn open_tables(&self, tx_ro: &Ro) -> Result<impl TablesIter, RuntimeError> {
        call_fn_on_all_tables_or_early_return! {
            Self::open_db_ro(self, tx_ro)
        }
    }

    /// Open all tables in read-write mode.
    ///
    /// This calls [`EnvInner::open_db_rw`] on all database tables
    /// and returns a structure that allows access to all tables.
    ///
    #[doc = doc_table_error!()]
    fn open_tables_mut(&self, tx_rw: &Rw) -> Result<impl TablesMut, RuntimeError> {
        call_fn_on_all_tables_or_early_return! {
            Self::open_db_rw(self, tx_rw)
        }
    }

    /// Clear all `(key, value)`'s from a database table.
    ///
    /// This will delete all key and values in the passed
    /// `T: Table`, but the table itself will continue to exist.
    ///
    /// Note that this operation is tied to `tx_rw`, as such this
    /// function's effects can be aborted using [`TxRw::abort`].
    ///
    #[doc = doc_table_error!()]
    fn clear_db<T: Table>(&self, tx_rw: &mut Rw) -> Result<(), RuntimeError>;
}
