<<<<<<< HEAD
//! General free functions (related to the database).

//---------------------------------------------------------------------------------------------------- Import
use cuprate_database::{ConcreteEnv, Env, EnvInner, InitError, RuntimeError, TxRw};

use crate::{config::Config, open_tables::OpenTables};

//---------------------------------------------------------------------------------------------------- Free functions
/// TODO
///
/// # Errors
/// TODO
#[cold]
#[inline(never)] // only called once
pub fn open(config: Config) -> Result<ConcreteEnv, InitError> {
    // Attempt to open the database environment.
    let env = <ConcreteEnv as Env>::open(config.db_config)?;

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
        let tx_rw = env_inner.tx_rw();
        let tx_rw = match tx_rw {
            Ok(tx_rw) => tx_rw,
            Err(e) => return Err(runtime_to_init_error(e)),
        };

        // Create all tables.
        if let Err(e) = OpenTables::create_tables(&env_inner, &tx_rw) {
            return Err(runtime_to_init_error(e));
        };

        if let Err(e) = tx_rw.commit() {
            return Err(runtime_to_init_error(e));
        }
    }

    Ok(env)
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
||||||| c837f2f
=======
//! General free functions (related to the database).

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Free functions

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
>>>>>>> main
