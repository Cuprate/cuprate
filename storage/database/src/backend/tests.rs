//! Tests for `cuprate_database`'s backends.
//!
//! These tests are fully trait-based, meaning there
//! is no reference to `backend/`-specific types.
//!
//! As such, which backend is tested is
//! dependant on the feature flags used.
//!
//! | Feature flag  | Tested backend |
//! |---------------|----------------|
//! | Only `redb`   | `redb`
//! | Anything else | `heed`
//!
//! `redb`, and it only must be enabled for it to be tested.

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    database::{DatabaseIter, DatabaseRo, DatabaseRw},
    env::{Env, EnvInner},
    error::RuntimeError,
    resize::ResizeAlgorithm,
    tests::{tmp_concrete_env, TestTable},
    transaction::{TxRo, TxRw},
    ConcreteEnv,
};

//---------------------------------------------------------------------------------------------------- Tests
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

/// Test [`Env::open`] and creating/opening tables.
#[test]
fn open_db() {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();

    // Create table.
    {
        let tx_rw = env_inner.tx_rw().unwrap();
        env_inner.create_db::<TestTable>(&tx_rw).unwrap();
        TxRw::commit(tx_rw).unwrap();
    }

    let tx_ro = env_inner.tx_ro().unwrap();
    let tx_rw = env_inner.tx_rw().unwrap();

    // Open table in read-only mode.
    env_inner.open_db_ro::<TestTable>(&tx_ro).unwrap();
    TxRo::commit(tx_ro).unwrap();

    // Open table in read/write mode.
    env_inner.open_db_rw::<TestTable>(&tx_rw).unwrap();
    TxRw::commit(tx_rw).unwrap();
}

/// Assert that opening a read-only table before creating errors.
#[test]
fn open_ro_uncreated_table() {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();

    // Open uncreated table.
    let error = env_inner.open_db_ro::<TestTable>(&tx_ro);
    assert!(matches!(error, Err(RuntimeError::TableNotFound)));
}

/// Assert that opening a read/write table before creating is OK.
#[test]
fn open_rw_uncreated_table() {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();

    // Open uncreated table.
    let _table = env_inner.open_db_rw::<TestTable>(&tx_rw).unwrap();
}

/// Assert that opening a read-only table after creating is OK.
#[test]
fn open_ro_created_table() {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();

    // Assert uncreated table errors.
    {
        let tx_ro = env_inner.tx_ro().unwrap();
        let error = env_inner.open_db_ro::<TestTable>(&tx_ro);
        assert!(matches!(error, Err(RuntimeError::TableNotFound)));
    }

    // Create table.
    {
        let tx_rw = env_inner.tx_rw().unwrap();
        env_inner.create_db::<TestTable>(&tx_rw).unwrap();
        TxRw::commit(tx_rw).unwrap();
    }

    // Assert created table is now OK.
    let tx_ro = env_inner.tx_ro().unwrap();
    let _table = env_inner.open_db_ro::<TestTable>(&tx_ro).unwrap();
}

/// Test `Env` resizes.
#[test]
fn resize() {
    // This test is only valid for `Env`'s that need to resize manually.
    if !ConcreteEnv::MANUAL_RESIZE {
        return;
    }

    let (env, _tempdir) = tmp_concrete_env();

    // Resize by the OS page size.
    let page_size = *crate::resize::PAGE_SIZE;
    let old_size = env.current_map_size();
    env.resize_map(Some(ResizeAlgorithm::FixedBytes(page_size)));

    // Assert it resized exactly by the OS page size.
    let new_size = env.current_map_size();
    assert_eq!(new_size, old_size + page_size.get());
}

/// Test that `Env`'s that don't manually resize.
#[test]
#[should_panic = "unreachable"]
fn non_manual_resize_1() {
    if ConcreteEnv::MANUAL_RESIZE {
        unreachable!();
    }
    let (env, _tempdir) = tmp_concrete_env();
    env.resize_map(None);
}

#[test]
#[should_panic = "unreachable"]
fn non_manual_resize_2() {
    if ConcreteEnv::MANUAL_RESIZE {
        unreachable!();
    }
    let (env, _tempdir) = tmp_concrete_env();
    env.current_map_size();
}

/// Tests that [`EnvInner::clear_db`] will return
/// [`RuntimeError::TableNotFound`] if the table doesn't exist.
#[test]
fn clear_db_table_not_found() {
    let (env, _tmpdir) = tmp_concrete_env();
    let env_inner = env.env_inner();
    let mut tx_rw = env_inner.tx_rw().unwrap();
    let err = env_inner.clear_db::<TestTable>(&mut tx_rw).unwrap_err();
    assert!(matches!(err, RuntimeError::TableNotFound));

    env_inner.create_db::<TestTable>(&tx_rw).unwrap();
    env_inner.clear_db::<TestTable>(&mut tx_rw).unwrap();
}

/// Test all `DatabaseR{o,w}` operations.
#[test]
fn db_read_write() {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<TestTable>(&tx_rw).unwrap();

    /// The (1st) key.
    const KEY: u32 = 0;
    /// The expected value.
    const VALUE: u64 = 0;
    /// How many `(key, value)` pairs will be inserted.
    const N: u32 = 100;

    /// Assert a u64 is the same as `VALUE`.
    fn assert_value(value: u64) {
        assert_eq!(value, VALUE);
    }

    assert!(table.is_empty().unwrap());

    // Insert keys.
    let mut key = KEY;
    #[expect(clippy::explicit_counter_loop, reason = "we need the +1 side effect")]
    for _ in 0..N {
        table.put(&key, &VALUE).unwrap();
        key += 1;
    }

    assert_eq!(table.len().unwrap(), u64::from(N));

    // Assert the first/last `(key, value)`s are there.
    {
        assert!(table.contains(&KEY).unwrap());
        let get = table.get(&KEY).unwrap();
        assert_value(get);

        let first = table.first().unwrap().1;
        assert_value(first);

        let last = table.last().unwrap().1;
        assert_value(last);
    }

    // Commit transactions, create new ones.
    drop(table);
    TxRw::commit(tx_rw).unwrap();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table_ro = env_inner.open_db_ro::<TestTable>(&tx_ro).unwrap();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<TestTable>(&tx_rw).unwrap();

    // Assert the whole range is there.
    {
        let range = table_ro.values().unwrap();
        let mut i = 0;
        for result in range {
            let value = result.unwrap();
            assert_value(value);
            i += 1;
        }
        assert_eq!(i, N);
    }

    // iter tests.

    // Assert count is correct.
    assert_eq!(N as usize, table_ro.values().unwrap().count());

    // Assert each returned value from the iterator is owned.
    {
        let mut iter = table_ro.values().unwrap();
        let value = iter.next().unwrap().unwrap(); // 1. take value out
        drop(iter); // 2. drop the `impl Iterator + 'a`
        assert_value(value); // 3. assert even without the iterator, the value is alive
    }

    // Assert each value is the same.
    {
        let mut iter = table_ro.values().unwrap();
        for _ in 0..N {
            let value = iter.next().unwrap().unwrap();
            assert_value(value);
        }
    }

    // Assert `Entry` works.
    {
        const NEW_VALUE: u64 = 999;

        assert_ne!(table.get(&KEY).unwrap(), NEW_VALUE);

        table
            .entry(&KEY)
            .unwrap()
            .and_update(|value| {
                *value = NEW_VALUE;
            })
            .unwrap();

        assert_eq!(table.get(&KEY).unwrap(), NEW_VALUE);
    }

    // Assert deleting works.
    {
        table.delete(&KEY).unwrap();
        let value = table.get(&KEY);
        assert!(!table.contains(&KEY).unwrap());
        assert!(matches!(value, Err(RuntimeError::KeyNotFound)));
        // Assert the other `(key, value)` pairs are still there.
        let mut key = KEY;
        key += N - 1; // we used inclusive `0..N`
        let value = table.get(&key).unwrap();
        assert_value(value);
    }

    // Assert `take()` works.
    {
        let mut key = KEY;
        key += 1;
        let value = table.take(&key).unwrap();
        assert_eq!(value, VALUE);

        let get = table.get(&KEY);
        assert!(!table.contains(&key).unwrap());
        assert!(matches!(get, Err(RuntimeError::KeyNotFound)));

        // Assert the other `(key, value)` pairs are still there.
        key += 1;
        let value = table.get(&key).unwrap();
        assert_value(value);
    }

    drop(table);
    TxRw::commit(tx_rw).unwrap();

    // Assert `clear_db()` works.
    {
        let mut tx_rw = env_inner.tx_rw().unwrap();
        env_inner.clear_db::<TestTable>(&mut tx_rw).unwrap();
        let table = env_inner.open_db_rw::<TestTable>(&tx_rw).unwrap();
        assert!(table.is_empty().unwrap());
        for n in 0..N {
            let mut key = KEY;
            key += n;
            let value = table.get(&key);
            assert!(matches!(value, Err(RuntimeError::KeyNotFound)));
            assert!(!table.contains(&key).unwrap());
        }

        // Reader still sees old value.
        assert!(!table_ro.is_empty().unwrap());

        // Writer sees updated value (nothing).
        assert!(table.is_empty().unwrap());
    }
}

/// Assert that `key`'s in database tables are sorted in
/// an ordered B-Tree fashion, i.e. `min_value -> max_value`.
///
/// And that it is true for integers, e.g. `0` -> `10`.
#[test]
fn tables_are_sorted() {
    let (env, _tmp) = tmp_concrete_env();
    let env_inner = env.env_inner();

    /// Range of keys to insert, `{0, 1, 2 ... 256}`.
    const RANGE: std::ops::Range<u32> = 0..257;

    // Create tables and set flags / comparison flags.
    {
        let tx_rw = env_inner.tx_rw().unwrap();
        env_inner.create_db::<TestTable>(&tx_rw).unwrap();
        TxRw::commit(tx_rw).unwrap();
    }

    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<TestTable>(&tx_rw).unwrap();

    // Insert range, assert each new
    // number inserted is the minimum `last()` value.
    for key in RANGE {
        table.put(&key, &0).unwrap();
        table.contains(&key).unwrap();
        let (first, _) = table.first().unwrap();
        let (last, _) = table.last().unwrap();
        println!("first: {first}, last: {last}, key: {key}");
        assert_eq!(last, key);
    }

    drop(table);
    TxRw::commit(tx_rw).unwrap();
    let tx_rw = env_inner.tx_rw().unwrap();

    // Assert iterators are ordered.
    {
        let tx_ro = env_inner.tx_ro().unwrap();
        let table = env_inner.open_db_ro::<TestTable>(&tx_ro).unwrap();
        let iter = table.iter().unwrap();
        let keys = table.keys().unwrap();
        for ((i, iter), key) in RANGE.zip(iter).zip(keys) {
            let (iter, _) = iter.unwrap();
            let key = key.unwrap();
            assert_eq!(i, iter);
            assert_eq!(iter, key);
        }
    }

    let mut table = env_inner.open_db_rw::<TestTable>(&tx_rw).unwrap();

    // Assert the `first()` values are the minimum, i.e. `{0, 1, 2}`
    for key in [0, 1, 2] {
        let (first, _) = table.first().unwrap();
        assert_eq!(first, key);
        table.delete(&key).unwrap();
    }

    // Assert the `last()` values are the maximum, i.e. `{256, 255, 254}`
    for key in [256, 255, 254] {
        let (last, _) = table.last().unwrap();
        assert_eq!(last, key);
        table.delete(&key).unwrap();
    }
}
