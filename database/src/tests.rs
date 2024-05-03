//! Utilities for `cuprate_database` testing, and some tests.
//!
//! These types/fn's are only:
//! - enabled on #[cfg(test)]
//! - only used internally

//---------------------------------------------------------------------------------------------------- Import
use std::fmt::Debug;

use pretty_assertions::assert_eq;

use crate::{
    config::ConfigBuilder,
    tables::{Tables, TablesIter, TablesMut},
    ConcreteEnv, DatabaseIter, DatabaseRo, DatabaseRw, Env, EnvInner, TxRw,
};

//---------------------------------------------------------------------------------------------------- Struct
/// Named struct to assert the length of all tables.
///
/// This is a struct with fields instead of a function
/// so that callers can name arguments, otherwise the call-site
/// is a little confusing, i.e. `assert_table_len(0, 25, 1, 123)`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct AssertTableLen {
    pub(crate) block_infos: u64,
    pub(crate) block_blobs: u64,
    pub(crate) block_heights: u64,
    pub(crate) key_images: u64,
    pub(crate) num_outputs: u64,
    pub(crate) pruned_tx_blobs: u64,
    pub(crate) prunable_hashes: u64,
    pub(crate) outputs: u64,
    pub(crate) prunable_tx_blobs: u64,
    pub(crate) rct_outputs: u64,
    pub(crate) tx_blobs: u64,
    pub(crate) tx_ids: u64,
    pub(crate) tx_heights: u64,
    pub(crate) tx_unlock_time: u64,
}

impl AssertTableLen {
    /// Assert the length of all tables.
    pub(crate) fn assert(self, tables: &impl Tables) {
        let other = Self {
            block_infos: tables.block_infos().len().unwrap(),
            block_blobs: tables.block_blobs().len().unwrap(),
            block_heights: tables.block_heights().len().unwrap(),
            key_images: tables.key_images().len().unwrap(),
            num_outputs: tables.num_outputs().len().unwrap(),
            pruned_tx_blobs: tables.pruned_tx_blobs().len().unwrap(),
            prunable_hashes: tables.prunable_hashes().len().unwrap(),
            outputs: tables.outputs().len().unwrap(),
            prunable_tx_blobs: tables.prunable_tx_blobs().len().unwrap(),
            rct_outputs: tables.rct_outputs().len().unwrap(),
            tx_blobs: tables.tx_blobs().len().unwrap(),
            tx_ids: tables.tx_ids().len().unwrap(),
            tx_heights: tables.tx_heights().len().unwrap(),
            tx_unlock_time: tables.tx_unlock_time().len().unwrap(),
        };

        assert_eq!(self, other);
    }
}

//---------------------------------------------------------------------------------------------------- fn
/// Create an `Env` in a temporarily directory.
/// The directory is automatically removed after the `TempDir` is dropped.
///
/// FIXME: changing this to `-> impl Env` causes lifetime errors...
pub(crate) fn tmp_concrete_env() -> (ConcreteEnv, tempfile::TempDir) {
    let tempdir = tempfile::tempdir().unwrap();
    let config = ConfigBuilder::new()
        .db_directory(tempdir.path().into())
        .low_power()
        .build();
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

//---------------------------------------------------------------------------------------------------- Tests
/// Assert that `key`'s in database tables are sorted in
/// an ordered B-Tree fashion, i.e. `min_value -> max_value`.
#[test]
fn tables_are_sorted() {
    let (env, _tmp) = tmp_concrete_env();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut tables_mut = env_inner.open_tables_mut(&tx_rw).unwrap();

    // Insert `{5, 4, 3, 2, 1, 0}`, assert each new
    // number inserted is the minimum `first()` value.
    for key in (0..6).rev() {
        tables_mut.num_outputs_mut().put(&key, &123).unwrap();
        let (first, _) = tables_mut.num_outputs_mut().first().unwrap();
        assert_eq!(first, key);
    }

    drop(tables_mut);
    TxRw::commit(tx_rw).unwrap();
    let tx_rw = env_inner.tx_rw().unwrap();

    // Assert iterators are ordered.
    {
        let tx_ro = env_inner.tx_ro().unwrap();
        let tables = env_inner.open_tables(&tx_ro).unwrap();
        let t = tables.num_outputs_iter();
        let iter = t.iter().unwrap();
        let keys = t.keys().unwrap();
        for ((i, iter), key) in (0..6).zip(iter).zip(keys) {
            let (iter, _) = iter.unwrap();
            let key = key.unwrap();
            assert_eq!(i, iter);
            assert_eq!(iter, key);
        }
    }

    let mut tables_mut = env_inner.open_tables_mut(&tx_rw).unwrap();
    let t = tables_mut.num_outputs_mut();

    // Assert the `first()` values are the minimum, i.e. `{0, 1, 2}`
    for key in 0..3 {
        let (first, _) = t.first().unwrap();
        assert_eq!(first, key);
        t.delete(&key).unwrap();
    }

    // Assert the `last()` values are the maximum, i.e. `{5, 4, 3}`
    for key in (3..6).rev() {
        let (last, _) = tables_mut.num_outputs_mut().last().unwrap();
        assert_eq!(last, key);
        tables_mut.num_outputs_mut().delete(&key).unwrap();
    }
}
