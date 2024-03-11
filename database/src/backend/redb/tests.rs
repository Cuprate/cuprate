//! Tests for `cuprate_database`, backed by `redb`.

//---------------------------------------------------------------------------------------------------- Import
use std::borrow::{Borrow, Cow};

use crate::{
    config::{Config, SyncMode},
    database::{DatabaseRo, DatabaseRw},
    env::{Env, EnvInner},
    error::{InitError, RuntimeError},
    resize::ResizeAlgorithm,
    table::Table,
    tables::{TestTable, TestTable2},
    transaction::{TxRo, TxRw},
    types::TestType,
    value_guard::ValueGuard,
    ConcreteEnv,
};

//---------------------------------------------------------------------------------------------------- Tests
// FIXME: there is no need to duplicate these tests.
// They can be re-used word-for-word across backends
// since we don't reference any backend specific types
// and only use `cuprate_database`'s traits.
//
// De-duplicate these somehow.

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

    TxRo::commit(env_inner.tx_ro().unwrap()).unwrap();
    TxRw::commit(env_inner.tx_rw().unwrap()).unwrap();
    TxRw::abort(env_inner.tx_rw().unwrap()).unwrap();
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
    TxRo::commit(tx_ro).unwrap();

    // Open all tables in read/write mode.
    env_inner.open_db_rw::<TestTable>(&mut tx_rw).unwrap();
    env_inner.open_db_rw::<TestTable2>(&mut tx_rw).unwrap();
    TxRw::commit(tx_rw).unwrap();
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
        let guard = table.get(&KEY).unwrap();
        let cow: Cow<'_, TestType> = guard.unguard();
        let value: &TestType = cow.as_ref();

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
        for result in range {
            let guard = result.unwrap();
            let cow: Cow<'_, TestType> = guard.unguard();
            let value: &TestType = cow.as_ref();

            assert_eq!(value, &VALUE);
            assert_eq!(value.u, VALUE.u);
            assert_eq!(value.b, VALUE.b);
            assert_eq!(value._pad, VALUE._pad);

            i += 1;
        }
        assert_eq!(i, 100);
    }

    table.delete(&KEY).unwrap();

    let value = table.get(&KEY);
    assert!(matches!(value, Err(RuntimeError::KeyNotFound)));
}
