//! General free functions (related to the tx-pool database).

//---------------------------------------------------------------------------------------------------- Import
use sha3::{Digest, Sha3_256};

use cuprate_database::{ConcreteEnv, Env, EnvInner, InitError, RuntimeError, TxRw};

use crate::{config::Config, tables::OpenTables, types::TransactionBlobHash};

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
pub fn open(config: Config) -> Result<ConcreteEnv, InitError> {
    // Attempt to open the database environment.
    let env = <ConcreteEnv as Env>::open(config.db_config)?;

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

            // These errors shouldn't be happening here.
            RuntimeError::KeyExists
            | RuntimeError::KeyNotFound
            | RuntimeError::ResizeNeeded
            | RuntimeError::TableNotFound => unreachable!(),
        }
    }

    // INVARIANT: We must ensure that all tables are created,
    // `cuprate_database` has no way of knowing _which_ tables
    // we want since it is agnostic, so we are responsible for this.
    {
        let env_inner = env.env_inner();
        let tx_rw = env_inner.tx_rw().map_err(runtime_to_init_error)?;

        // Create all tables.
        OpenTables::create_tables(&env_inner, &tx_rw).map_err(runtime_to_init_error)?;

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
    let mut hasher = Sha3_256::new();
    hasher.update(tx_blob);
    hasher.finalize().into()
}
