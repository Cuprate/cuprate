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

#![allow(
    clippy::items_after_statements,
    clippy::significant_drop_tightening,
    clippy::cast_possible_truncation
)]

//---------------------------------------------------------------------------------------------------- Import
use std::borrow::{Borrow, Cow};

use crate::{
    config::{Config, SyncMode},
    database::{DatabaseIter, DatabaseRo, DatabaseRw},
    env::{Env, EnvInner},
    error::{InitError, RuntimeError},
    resize::ResizeAlgorithm,
    storable::StorableVec,
    table::Table,
    tables::{
        BlockBlobs, BlockHeights, BlockInfos, KeyImages, NumOutputs, Outputs, PrunableHashes,
        PrunableTxBlobs, PrunedTxBlobs, RctOutputs, TxBlobs, TxHeights, TxIds, TxOutputs,
        TxUnlockTime,
    },
    tests::tmp_concrete_env,
    transaction::{TxRo, TxRw},
    types::{
        Amount, AmountIndex, AmountIndices, BlockBlob, BlockHash, BlockHeight, BlockInfo, KeyImage,
        Output, OutputFlags, PreRctOutputId, PrunableBlob, PrunableHash, PrunedBlob, RctOutput,
        TxBlob, TxHash, TxId, UnlockTime,
    },
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

/// Open (and verify) that all database tables
/// exist already after calling [`Env::open`].
#[test]
fn open_db() {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let tx_rw = env_inner.tx_rw().unwrap();

    // Open all tables in read-only mode.
    // This should be updated when tables are modified.
    env_inner.open_db_ro::<BlockBlobs>(&tx_ro).unwrap();
    env_inner.open_db_ro::<BlockHeights>(&tx_ro).unwrap();
    env_inner.open_db_ro::<BlockInfos>(&tx_ro).unwrap();
    env_inner.open_db_ro::<KeyImages>(&tx_ro).unwrap();
    env_inner.open_db_ro::<NumOutputs>(&tx_ro).unwrap();
    env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();
    env_inner.open_db_ro::<PrunableHashes>(&tx_ro).unwrap();
    env_inner.open_db_ro::<PrunableTxBlobs>(&tx_ro).unwrap();
    env_inner.open_db_ro::<PrunedTxBlobs>(&tx_ro).unwrap();
    env_inner.open_db_ro::<RctOutputs>(&tx_ro).unwrap();
    env_inner.open_db_ro::<TxBlobs>(&tx_ro).unwrap();
    env_inner.open_db_ro::<TxHeights>(&tx_ro).unwrap();
    env_inner.open_db_ro::<TxIds>(&tx_ro).unwrap();
    env_inner.open_db_ro::<TxOutputs>(&tx_ro).unwrap();
    env_inner.open_db_ro::<TxUnlockTime>(&tx_ro).unwrap();
    TxRo::commit(tx_ro).unwrap();

    // Open all tables in read/write mode.
    env_inner.open_db_rw::<BlockBlobs>(&tx_rw).unwrap();
    env_inner.open_db_rw::<BlockHeights>(&tx_rw).unwrap();
    env_inner.open_db_rw::<BlockInfos>(&tx_rw).unwrap();
    env_inner.open_db_rw::<KeyImages>(&tx_rw).unwrap();
    env_inner.open_db_rw::<NumOutputs>(&tx_rw).unwrap();
    env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();
    env_inner.open_db_rw::<PrunableHashes>(&tx_rw).unwrap();
    env_inner.open_db_rw::<PrunableTxBlobs>(&tx_rw).unwrap();
    env_inner.open_db_rw::<PrunedTxBlobs>(&tx_rw).unwrap();
    env_inner.open_db_rw::<RctOutputs>(&tx_rw).unwrap();
    env_inner.open_db_rw::<TxBlobs>(&tx_rw).unwrap();
    env_inner.open_db_rw::<TxHeights>(&tx_rw).unwrap();
    env_inner.open_db_rw::<TxIds>(&tx_rw).unwrap();
    env_inner.open_db_rw::<TxOutputs>(&tx_rw).unwrap();
    env_inner.open_db_rw::<TxUnlockTime>(&tx_rw).unwrap();
    TxRw::commit(tx_rw).unwrap();
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
    let page_size = crate::resize::page_size();
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
    } else {
        let (env, _tempdir) = tmp_concrete_env();
        env.resize_map(None);
    }
}

#[test]
#[should_panic = "unreachable"]
fn non_manual_resize_2() {
    if ConcreteEnv::MANUAL_RESIZE {
        unreachable!();
    } else {
        let (env, _tempdir) = tmp_concrete_env();
        env.current_map_size();
    }
}

/// Test all `DatabaseR{o,w}` operations.
#[test]
#[allow(clippy::too_many_lines)]
fn db_read_write() {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    /// The (1st) key.
    const KEY: PreRctOutputId = PreRctOutputId {
        amount: 1,
        amount_index: 123,
    };
    /// The expected value.
    const VALUE: Output = Output {
        key: [35; 32],
        height: 45_761_798,
        output_flags: OutputFlags::empty(),
        tx_idx: 2_353_487,
    };
    /// How many `(key, value)` pairs will be inserted.
    const N: u64 = 100;

    /// Assert 2 `Output`'s are equal, and that accessing
    /// their fields don't result in an unaligned panic.
    fn assert_same(output: Output) {
        assert_eq!(output, VALUE);
        assert_eq!(output.key, VALUE.key);
        assert_eq!(output.height, VALUE.height);
        assert_eq!(output.output_flags, VALUE.output_flags);
        assert_eq!(output.tx_idx, VALUE.tx_idx);
    }

    assert!(table.is_empty().unwrap());

    // Insert keys.
    let mut key = KEY;
    for i in 0..N {
        table.put(&key, &VALUE).unwrap();
        key.amount += 1;
    }

    assert_eq!(table.len().unwrap(), N);

    // Assert the first/last `(key, value)`s are there.
    {
        assert!(table.contains(&KEY).unwrap());
        let get: Output = table.get(&KEY).unwrap();
        assert_same(get);

        let first: Output = table.first().unwrap().1;
        assert_same(first);

        let last: Output = table.last().unwrap().1;
        assert_same(last);
    }

    // Commit transactions, create new ones.
    drop(table);
    TxRw::commit(tx_rw).unwrap();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table_ro = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    // Assert the whole range is there.
    {
        let range = table_ro.get_range(..).unwrap();
        let mut i = 0;
        for result in range {
            let value: Output = result.unwrap();
            assert_same(value);

            i += 1;
        }
        assert_eq!(i, N);
    }

    // `get_range()` tests.
    let mut key = KEY;
    key.amount += N;
    let range = KEY..key;

    // Assert count is correct.
    assert_eq!(
        N as usize,
        table_ro.get_range(range.clone()).unwrap().count()
    );

    // Assert each returned value from the iterator is owned.
    {
        let mut iter = table_ro.get_range(range.clone()).unwrap();
        let value: Output = iter.next().unwrap().unwrap(); // 1. take value out
        drop(iter); // 2. drop the `impl Iterator + 'a`
        assert_same(value); // 3. assert even without the iterator, the value is alive
    }

    // Assert each value is the same.
    {
        let mut iter = table_ro.get_range(range).unwrap();
        for _ in 0..N {
            let value: Output = iter.next().unwrap().unwrap();
            assert_same(value);
        }
    }

    // Assert `update()` works.
    {
        const HEIGHT: u32 = 999;

        assert_ne!(table.get(&KEY).unwrap().height, HEIGHT);

        table
            .update(&KEY, |mut value| {
                value.height = HEIGHT;
                Some(value)
            })
            .unwrap();

        assert_eq!(table.get(&KEY).unwrap().height, HEIGHT);
    }

    // Assert deleting works.
    {
        table.delete(&KEY).unwrap();
        let value = table.get(&KEY);
        assert!(!table.contains(&KEY).unwrap());
        assert!(matches!(value, Err(RuntimeError::KeyNotFound)));
        // Assert the other `(key, value)` pairs are still there.
        let mut key = KEY;
        key.amount += N - 1; // we used inclusive `0..N`
        let value = table.get(&key).unwrap();
        assert_same(value);
    }

    // Assert `take()` works.
    {
        let mut key = KEY;
        key.amount += 1;
        let value = table.take(&key).unwrap();
        assert_eq!(value, VALUE);

        let get = table.get(&KEY);
        assert!(!table.contains(&key).unwrap());
        assert!(matches!(get, Err(RuntimeError::KeyNotFound)));

        // Assert the other `(key, value)` pairs are still there.
        key.amount += 1;
        let value = table.get(&key).unwrap();
        assert_same(value);
    }

    drop(table);
    TxRw::commit(tx_rw).unwrap();

    // Assert `clear_db()` works.
    {
        let mut tx_rw = env_inner.tx_rw().unwrap();
        env_inner.clear_db::<Outputs>(&mut tx_rw).unwrap();
        let table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();
        assert!(table.is_empty().unwrap());
        for n in 0..N {
            let mut key = KEY;
            key.amount += n;
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

//---------------------------------------------------------------------------------------------------- Table Tests
/// Test multiple tables and their key + values.
///
/// Each one of these tests:
/// - Opens a specific table
/// - Essentially does the `db_read_write` test
macro_rules! test_tables {
    ($(
        $table:ident,    // Table type
        $key_type:ty =>  // Key (type)
        $value_type:ty,  // Value (type)
        $key:expr =>     // Key (the value)
        $value:expr,     // Value (the value)
    )* $(,)?) => { paste::paste! { $(
        // Test function's name is the table type in `snake_case`.
        #[test]
        fn [<$table:snake>]() {
            // Open the database env and table.
            let (env, _tempdir) = tmp_concrete_env();
            let env_inner = env.env_inner();
            let mut tx_rw = env_inner.tx_rw().unwrap();
            let mut table = env_inner.open_db_rw::<$table>(&mut tx_rw).unwrap();

            /// The expected key.
            const KEY: $key_type = $key;
            // The expected value.
            let value: $value_type = $value;

            // Assert a passed value is equal to the const value.
            let assert_eq = |v: &$value_type| {
                assert_eq!(v, &value);
            };

            // Insert the key.
            table.put(&KEY, &value).unwrap();
            // Assert key is there.
            {
                let value: $value_type = table.get(&KEY).unwrap();
                assert_eq(&value);
            }

            assert!(table.contains(&KEY).unwrap());
            assert_eq!(table.len().unwrap(), 1);

            // Commit transactions, create new ones.
            drop(table);
            TxRw::commit(tx_rw).unwrap();
            let mut tx_rw = env_inner.tx_rw().unwrap();
            let tx_ro = env_inner.tx_ro().unwrap();
            let mut table = env_inner.open_db_rw::<$table>(&tx_rw).unwrap();
            let table_ro = env_inner.open_db_ro::<$table>(&tx_ro).unwrap();

            // Assert `get_range()` works.
            {
                let range = KEY..;
                assert_eq!(1, table_ro.get_range(range.clone()).unwrap().count());
                let mut iter = table_ro.get_range(range).unwrap();
                let value = iter.next().unwrap().unwrap();
                assert_eq(&value);
            }

            // Assert deleting works.
            {
                table.delete(&KEY).unwrap();
                let value = table.get(&KEY);
                assert!(matches!(value, Err(RuntimeError::KeyNotFound)));
                assert!(!table.contains(&KEY).unwrap());
                assert_eq!(table.len().unwrap(), 0);
            }

            table.put(&KEY, &value).unwrap();

            // Assert `clear_db()` works.
            {
                drop(table);
                env_inner.clear_db::<$table>(&mut tx_rw).unwrap();
                let table = env_inner.open_db_rw::<$table>(&mut tx_rw).unwrap();
                let value = table.get(&KEY);
                assert!(matches!(value, Err(RuntimeError::KeyNotFound)));
                assert!(!table.contains(&KEY).unwrap());
                assert_eq!(table.len().unwrap(), 0);
            }
        }
    )*}};
}

// Notes:
// - Keep this sorted A-Z (by table name)
test_tables! {
    BlockBlobs, // Table type
    BlockHeight => BlockBlob, // Key type => Value type
    123 => StorableVec(vec![1,2,3,4,5,6,7,8]), // Actual key => Actual value

    BlockHeights,
    BlockHash => BlockHeight,
    [32; 32] => 123,

    BlockInfos,
    BlockHeight => BlockInfo,
    123 => BlockInfo {
        timestamp: 1,
        cumulative_generated_coins: 123,
        weight: 321,
        cumulative_difficulty: 111,
        block_hash: [54; 32],
        cumulative_rct_outs: 2389,
        long_term_weight: 2389,
    },

    KeyImages,
    KeyImage => (),
    [32; 32] => (),

    NumOutputs,
    Amount => AmountIndex,
    123 => 123,

    TxBlobs,
    TxId => TxBlob,
    123 => StorableVec(vec![1,2,3,4,5,6,7,8]),

    TxIds,
    TxHash => TxId,
    [32; 32] => 123,

    TxHeights,
    TxId => BlockHeight,
    123 => 123,

    TxOutputs,
    TxId => AmountIndices,
    123 => StorableVec(vec![1,2,3,4,5,6,7,8]),

    TxUnlockTime,
    TxId => UnlockTime,
    123 => 123,

    Outputs,
    PreRctOutputId => Output,
    PreRctOutputId {
        amount: 1,
        amount_index: 2,
    } => Output {
        key: [1; 32],
        height: 1,
        output_flags: OutputFlags::empty(),
        tx_idx: 3,
    },

    PrunedTxBlobs,
    TxId => PrunedBlob,
    123 => StorableVec(vec![1,2,3,4,5,6,7,8]),

    PrunableTxBlobs,
    TxId => PrunableBlob,
    123 => StorableVec(vec![1,2,3,4,5,6,7,8]),

    PrunableHashes,
    TxId => PrunableHash,
    123 => [32; 32],

    RctOutputs,
    AmountIndex => RctOutput,
    123 => RctOutput {
        key: [1; 32],
        height: 1,
        output_flags: OutputFlags::empty(),
        tx_idx: 3,
        commitment: [3; 32],
    },
}
