//! Tests for `cuprate_database`, backed by `heed`.

//---------------------------------------------------------------------------------------------------- Import
use std::borrow::Borrow;

use crate::{
    backend::heed::ConcreteEnv,
    config::{Config, SyncMode},
    database::{DatabaseRo, DatabaseRw},
    env::{Env, EnvInner},
    error::{InitError, RuntimeError},
    resize::ResizeAlgorithm,
    table::Table,
    tables::{TestTable, TestTable2},
    types::TestType,
};

//---------------------------------------------------------------------------------------------------- Tests
/// Create a `ConcreteEnv` in a temporarily directory.
/// The directory is automatically removed after the `TempDir` is dropped.
fn tmp_concrete_env() -> (ConcreteEnv, tempfile::TempDir) {
    let tempdir = tempfile::tempdir().unwrap();
    let config = Config::low_power(Some(tempdir.path().into()));
    let env = ConcreteEnv::open(config).unwrap();

    (env, tempdir)
}

/// Simply call [`Env::open`]. If this fails, something is really wrong.
#[test]
fn open() {
    tmp_concrete_env();
}

/// Create database transactions, but don't write any data.
#[test]
fn tx() {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();

    env_inner.tx_ro().unwrap().commit().unwrap();
    env_inner.tx_rw().unwrap().commit().unwrap();
    env_inner.tx_rw().unwrap().abort();
}

/// Open (and verify) that all database tables
/// exist already after calling [`Env::open`].
#[test]
#[allow(clippy::items_after_statements, clippy::significant_drop_tightening)]
fn open_db() {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let mut tx_rw = env_inner.tx_rw().unwrap();

    // Open all tables in read-only mode.
    // This should be updated when tables are modified.
    env_inner.open_db_ro::<TestTable>(&tx_ro).unwrap();
    env_inner.open_db_ro::<TestTable2>(&tx_ro).unwrap();
    tx_ro.commit().unwrap();

    // Open all tables in read/write mode.
    env_inner.open_db_rw::<TestTable>(&mut tx_rw).unwrap();
    env_inner.open_db_rw::<TestTable2>(&mut tx_rw).unwrap();
    tx_rw.commit().unwrap();
}

/// Test all `DatabaseR{o,w}` operations.
#[test]
#[allow(clippy::items_after_statements, clippy::significant_drop_tightening)]
fn db_read_write() {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();
    let mut tx_rw = env_inner.tx_rw().unwrap();

    let k = -999_i64;
    let v = TestType {
        u: 1,
        b: 255,
        _pad: [0; 7],
    };

    {
        env_inner
            .open_db_rw::<TestTable>(&mut tx_rw)
            .unwrap()
            .put(&k, &v)
            .unwrap();
    }

    {
        let table = env_inner.open_db_rw::<TestTable>(&mut tx_rw).unwrap();
        let table_value = table.get(&k).unwrap();
        // assert_eq!(table_value.borrow(), &v);
    }

    tx_rw.commit().unwrap();
}
