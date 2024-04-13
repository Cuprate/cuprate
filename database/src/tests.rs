//! Utilities for `cuprate_database` testing.
//!
//! These types/fn's are only:
//! - enabled on #[cfg(test)]
//! - only used internally

//---------------------------------------------------------------------------------------------------- Import
use std::{fmt::Debug, sync::OnceLock};

use monero_serai::{
    ringct::{RctPrunable, RctSignatures},
    transaction::{Timelock, Transaction, TransactionPrefix},
};

use crate::{config::Config, key::Key, storable::Storable, ConcreteEnv, Env};

//---------------------------------------------------------------------------------------------------- fn
/// Create an `Env` in a temporarily directory.
/// The directory is automatically removed after the `TempDir` is dropped.
///
/// FIXME: changing this to `-> impl Env` causes lifetime errors...
pub(crate) fn tmp_concrete_env() -> (ConcreteEnv, tempfile::TempDir) {
    let tempdir = tempfile::tempdir().unwrap();
    let config = Config::low_power(Some(tempdir.path().into()));
    let env = ConcreteEnv::open(config).unwrap();

    (env, tempdir)
}
