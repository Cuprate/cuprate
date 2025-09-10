//! General free functions (related to the tx-pool database).

use std::borrow::Cow;

use cuprate_database::{
    ConcreteEnv, DatabaseRo, Env, EnvInner, InitError, RuntimeError, StorableStr, TxRw,
};
use cuprate_database::{DatabaseRw, TxRo};

use crate::{
    config::Config,
    tables::{Metadata, OpenTables, TransactionBlobs},
    types::TransactionBlobHash,
};

/// The current version of the database format.
pub const DATABASE_VERSION: StorableStr = StorableStr(Cow::Borrowed("0.1"));

/// The key used to store the database version in the [`Metadata`] table.
pub const VERSION_KEY: StorableStr = StorableStr(Cow::Borrowed("version"));

//---------------------------------------------------------------------------------------------------- Free functions
/// Open the txpool database using the passed [`Config`].
///
/// This calls [`cuprate_database::Env::open`] and prepares the
/// database to be ready for txpool-related usage, e.g.
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
pub fn open(config: &Config) -> Result<ConcreteEnv, InitError> {
    // Attempt to open the database environment.
    let env = <ConcreteEnv as Env>::open(config.db_config.clone())?;

    /// Convert runtime errors to init errors.
    ///
    /// INVARIANT:
    /// [`cuprate_database`]'s functions mostly return the former
    /// so we must convert them. We have knowledge of which errors
    /// makes sense in this functions context so we panic on
    /// unexpected ones.
    fn runtime_to_init_error(runtime: RuntimeError) -> InitError {
        match runtime {
            RuntimeError::Io(io_error) => io_error.into(),
            RuntimeError::KeyNotFound => InitError::InvalidVersion,

            // These errors shouldn't be happening here.
            RuntimeError::KeyExists
            | RuntimeError::ResizeNeeded
            | RuntimeError::ResizedByAnotherProcess
            | RuntimeError::TableNotFound => unreachable!(),
        }
    }

    let fresh_db;

    // INVARIANT: We must ensure that all tables are created,
    // `cuprate_database` has no way of knowing _which_ tables
    // we want since it is agnostic, so we are responsible for this.
    {
        let env_inner = env.env_inner();

        // Store if this DB has been used before by checking if the [`TransactionBlobs`] table exists.
        let tx_ro = env_inner.tx_ro().map_err(runtime_to_init_error)?;
        fresh_db = env_inner.open_db_ro::<TransactionBlobs>(&tx_ro).is_err();
        TxRo::commit(tx_ro).map_err(runtime_to_init_error)?;

        let tx_rw = env_inner.tx_rw().map_err(runtime_to_init_error)?;

        // Create all tables.
        OpenTables::create_tables(&env_inner, &tx_rw).map_err(runtime_to_init_error)?;

        TxRw::commit(tx_rw).map_err(runtime_to_init_error)?;
    }

    {
        let env_inner = env.env_inner();
        let tx_rw = env_inner.tx_rw().map_err(runtime_to_init_error)?;

        let mut metadata = env_inner
            .open_db_rw::<Metadata>(&tx_rw)
            .map_err(runtime_to_init_error)?;

        if fresh_db {
            // If the database is new, add the version.
            metadata
                .put(&VERSION_KEY, &DATABASE_VERSION)
                .map_err(runtime_to_init_error)?;
        }

        let print_version_err = || {
            tracing::error!(
                "The database follows an old format, please delete the database at: {}",
                config.db_config.db_directory().display()
            );
        };

        let version = metadata
            .get(&VERSION_KEY)
            .inspect_err(|_| print_version_err())
            .map_err(runtime_to_init_error)?;

        if version != DATABASE_VERSION {
            // TODO: database migration when stable? This is the tx-pool so is not critical.
            print_version_err();
            return Err(InitError::InvalidVersion);
        }

        drop(metadata);
        TxRw::commit(tx_rw).map_err(runtime_to_init_error)?;
    }

    Ok(env)
}

/// Calculate the transaction blob hash.
///
/// This value is supposed to be quick to compute just based of the tx-blob without needing to parse the tx.
///
/// The exact way the hash is calculated is not stable and is subject to change, as such it should not be exposed
/// as a way to interact with Cuprate externally.
pub fn transaction_blob_hash(tx_blob: &[u8]) -> TransactionBlobHash {
    blake3::hash(tx_blob).into()
}
