//! Utilities for `cuprate_database` testing.
//!
//! These types/fn's are only:
//! - enabled on #[cfg(test)]
//! - only used internally

#![allow(clippy::significant_drop_tightening)]

//---------------------------------------------------------------------------------------------------- Import
use std::{
    fmt::Debug,
    sync::{Arc, OnceLock},
};

use monero_serai::{
    ringct::{RctPrunable, RctSignatures},
    transaction::{Timelock, Transaction, TransactionPrefix},
};

use crate::{
    config::Config, key::Key, storable::Storable, tables::Tables, transaction::TxRo, ConcreteEnv,
    Env, EnvInner,
};

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

/// Assert all the tables in the environment are empty.
pub(crate) fn assert_all_tables_are_empty(env: &ConcreteEnv) {
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let tables = env_inner.open_tables(&tx_ro).unwrap();
    assert!(tables.all_tables_empty().unwrap());
    assert_eq!(crate::ops::tx::get_num_tx(tables.tx_ids()).unwrap(), 0);
}
