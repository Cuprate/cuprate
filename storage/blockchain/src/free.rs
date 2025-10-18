//! General free functions (related to the database).

//---------------------------------------------------------------------------------------------------- Import
use crate::{config::Config, database::BlockchainDatabase, tables::OpenTables};
use cuprate_database::{ConcreteEnv, Env, EnvInner, InitError, RuntimeError, TxRw};
use cuprate_linear_tape::LinearTape;

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
pub fn open<E: Env>(config: Config) -> Result<BlockchainDatabase<E>, InitError> {
    // Attempt to open the database environment.
    let env = E::open(config.db_config.clone())?;

    /// Convert runtime errors to init errors.
    ///
    /// INVARIANT:
    /// `cuprate_database`'s functions mostly return the former
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

    let rct_outputs =
        unsafe { LinearTape::open(config.db_config.db_directory().join("rct_outputs.tape")) }?;

    Ok(BlockchainDatabase {
        dynamic_tables: env,
        rct_outputs,
    })
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
