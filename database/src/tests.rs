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
    DatabaseRo, Env, EnvInner,
};

//---------------------------------------------------------------------------------------------------- Struct
/// Named struct to assert the length of all tables.
///
/// This is a struct with fields instead of a function
/// so that callers can name arguments, otherwise the call-site
/// is a little confusing, i.e. `assert_table_len(0, 25, 1, 123)`.
pub(crate) struct AssertTableLen {
    block_infos: u64,
    block_blobs: u64,
    block_heights: u64,
    key_images: u64,
    num_outputs: u64,
    pruned_tx_blobs: u64,
    prunable_hashes: u64,
    outputs: u64,
    prunable_tx_blobs: u64,
    rct_outputs: u64,
    tx_blobs: u64,
    tx_ids: u64,
    tx_heights: u64,
    tx_unlock_time: u64,
}

impl AssertTableLen {
    /// Assert the length of all tables.
    pub(crate) fn assert(self, tables: &impl Tables) {
        for (table_len, self_len) in [
            (tables.block_infos().len(), self.block_infos),
            (tables.block_blobs().len(), self.block_blobs),
            (tables.block_heights().len(), self.block_heights),
            (tables.key_images().len(), self.key_images),
            (tables.num_outputs().len(), self.num_outputs),
            (tables.pruned_tx_blobs().len(), self.pruned_tx_blobs),
            (tables.prunable_hashes().len(), self.prunable_hashes),
            (tables.outputs().len(), self.outputs),
            (tables.prunable_tx_blobs().len(), self.prunable_tx_blobs),
            (tables.rct_outputs().len(), self.rct_outputs),
            (tables.tx_blobs().len(), self.tx_blobs),
            (tables.tx_ids().len(), self.tx_ids),
            (tables.tx_heights().len(), self.tx_heights),
            (tables.tx_unlock_time().len(), self.tx_unlock_time),
        ] {
            assert_eq!(table_len.unwrap(), self_len);
        }
    }
}

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
