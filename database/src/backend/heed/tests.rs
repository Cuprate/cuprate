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
    env_inner.tx_rw().unwrap().abort().unwrap();
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
#[allow(
    clippy::items_after_statements,
    clippy::significant_drop_tightening,
    clippy::used_underscore_binding
)]
fn db_read_write() {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();
    let mut tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<TestTable>(&mut tx_rw).unwrap();

    const KEY: i64 = 0_i64;
    const VALUE: TestType = TestType {
        u: 1,
        b: 255,
        _pad: [0; 7],
    };

    // Insert `0..100` keys.
    for i in 0..100 {
        table.put(&(KEY + i), &VALUE).unwrap();
    }

    // Assert the 1st key is there.
    {
        let mut guard = None;

        let value = table.get(&KEY, &mut guard).unwrap();
        let value = value.as_ref();
        // Make sure all field accesses are aligned.
        assert_eq!(value, &VALUE);
        assert_eq!(value.u, VALUE.u);
        assert_eq!(value.b, VALUE.b);
        assert_eq!(value._pad, VALUE._pad);
    }

    // Assert the whole range is there.
    {
        let range = table.get_range(..).unwrap();
        let mut i = 0;
        for value in range {
            let value = value.unwrap();
            let value: &TestType = value.as_ref();
            assert_eq!(value, &VALUE);
            assert_eq!(value.u, VALUE.u);
            assert_eq!(value.b, VALUE.b);
            assert_eq!(value._pad, VALUE._pad);
            i += 1;
        }
        assert_eq!(i, 100);
    }

    table.delete(&KEY).unwrap();

    let mut guard = None;

    let value = table.get(&KEY, &mut guard);
    assert!(matches!(value, Err(RuntimeError::KeyNotFound)));
}
