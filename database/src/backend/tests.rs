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

#![allow(clippy::items_after_statements, clippy::significant_drop_tightening)]

//---------------------------------------------------------------------------------------------------- Import
use std::borrow::{Borrow, Cow};

use crate::{
    config::{Config, SyncMode},
    database::{DatabaseRo, DatabaseRw},
    env::{Env, EnvInner},
    error::{InitError, RuntimeError},
    resize::ResizeAlgorithm,
    storable::StorableVec,
    table::Table,
    tables::{
        BlockBlobs, BlockHeights, BlockInfoV1s, BlockInfoV2s, BlockInfoV3s, KeyImages, NumOutputs,
        Outputs, PrunableHashes, PrunableTxBlobs, PrunedTxBlobs, RctOutputs, TxHeights, TxIds,
        TxUnlockTime,
    },
    transaction::{TxRo, TxRw},
    types::{
        Amount, AmountIndex, AmountIndices, BlockBlob, BlockHash, BlockHeight, BlockInfoV1,
        BlockInfoV2, BlockInfoV3, KeyImage, Output, PreRctOutputId, PrunableBlob, PrunableHash,
        PrunedBlob, RctOutput, TxHash, TxId, UnlockTime,
    },
    ConcreteEnv,
};

//---------------------------------------------------------------------------------------------------- Tests
/// Create an `Env` in a temporarily directory.
/// The directory is automatically removed after the `TempDir` is dropped.
///
/// TODO: changing this to `-> impl Env` causes lifetime errors...
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
fn open_db() {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let mut tx_rw = env_inner.tx_rw().unwrap();

    // Open all tables in read-only mode.
    // This should be updated when tables are modified.
    env_inner.open_db_ro::<BlockBlobs>(&tx_ro).unwrap();
    env_inner.open_db_ro::<BlockHeights>(&tx_ro).unwrap();
    env_inner.open_db_ro::<BlockInfoV1s>(&tx_ro).unwrap();
    env_inner.open_db_ro::<BlockInfoV2s>(&tx_ro).unwrap();
    env_inner.open_db_ro::<BlockInfoV3s>(&tx_ro).unwrap();
    env_inner.open_db_ro::<KeyImages>(&tx_ro).unwrap();
    env_inner.open_db_ro::<NumOutputs>(&tx_ro).unwrap();
    env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();
    env_inner.open_db_ro::<PrunableHashes>(&tx_ro).unwrap();
    env_inner.open_db_ro::<PrunableTxBlobs>(&tx_ro).unwrap();
    env_inner.open_db_ro::<PrunedTxBlobs>(&tx_ro).unwrap();
    env_inner.open_db_ro::<RctOutputs>(&tx_ro).unwrap();
    env_inner.open_db_ro::<TxHeights>(&tx_ro).unwrap();
    env_inner.open_db_ro::<TxIds>(&tx_ro).unwrap();
    env_inner.open_db_ro::<TxUnlockTime>(&tx_ro).unwrap();
    TxRo::commit(tx_ro).unwrap();

    // Open all tables in read/write mode.
    env_inner.open_db_rw::<BlockBlobs>(&mut tx_rw).unwrap();
    env_inner.open_db_rw::<BlockHeights>(&mut tx_rw).unwrap();
    env_inner.open_db_rw::<BlockInfoV1s>(&mut tx_rw).unwrap();
    env_inner.open_db_rw::<BlockInfoV2s>(&mut tx_rw).unwrap();
    env_inner.open_db_rw::<BlockInfoV3s>(&mut tx_rw).unwrap();
    env_inner.open_db_rw::<KeyImages>(&mut tx_rw).unwrap();
    env_inner.open_db_rw::<NumOutputs>(&mut tx_rw).unwrap();
    env_inner.open_db_rw::<Outputs>(&mut tx_rw).unwrap();
    env_inner.open_db_rw::<PrunableHashes>(&mut tx_rw).unwrap();
    env_inner.open_db_rw::<PrunableTxBlobs>(&mut tx_rw).unwrap();
    env_inner.open_db_rw::<PrunedTxBlobs>(&mut tx_rw).unwrap();
    env_inner.open_db_rw::<RctOutputs>(&mut tx_rw).unwrap();
    env_inner.open_db_rw::<TxHeights>(&mut tx_rw).unwrap();
    env_inner.open_db_rw::<TxIds>(&mut tx_rw).unwrap();
    env_inner.open_db_rw::<TxUnlockTime>(&mut tx_rw).unwrap();
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
fn db_read_write() {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();
    let mut tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&mut tx_rw).unwrap();

    /// The (1st) key.
    const KEY: PreRctOutputId = PreRctOutputId {
        amount: 1,
        amount_index: 123,
    };
    /// The expected value.
    const VALUE: Output = Output {
        key: [35; 32],
        height: 45_761_798,
        output_flags: 0,
        tx_idx: 2_353_487,
    };

    /// Assert 2 `Output`'s are equal, and that accessing
    /// their fields don't result in an unaligned panic.
    fn assert_same(output: Output) {
        assert_eq!(output, VALUE);
        assert_eq!(output.key, VALUE.key);
        assert_eq!(output.height, VALUE.height);
        assert_eq!(output.output_flags, VALUE.output_flags);
        assert_eq!(output.tx_idx, VALUE.tx_idx);
    }

    // Insert `0..100` keys.
    let mut key = KEY;
    for i in 0..100 {
        table.put(&key, &VALUE).unwrap();
        key.amount += 1;
    }

    // Assert the 1st key is there.
    {
        let value: Output = table.get(&KEY).unwrap();
        assert_same(value);
    }

    // Assert the whole range is there.
    {
        let range = table.get_range(..).unwrap();
        let mut i = 0;
        for result in range {
            let value: Output = result.unwrap();
            assert_same(value);

            i += 1;
        }
        assert_eq!(i, 100);
    }

    // Assert `get_range()` works.
    let mut key = KEY;
    key.amount += 100;
    let range = KEY..key;
    assert_eq!(100, table.get_range(range).unwrap().count());

    // Assert deleting works.
    table.delete(&KEY).unwrap();
    let value = table.get(&KEY);
    assert!(matches!(value, Err(RuntimeError::KeyNotFound)));
}

//---------------------------------------------------------------------------------------------------- Table Tests
/// Test multiple tables and their key + values.
///
/// Each one of these tests:
/// - Opens a specific table
/// - Inserts a key + value
/// - Retrieves the key + value
/// - Asserts it is the same
/// - Tests `get_range()`
/// - Tests `delete()`
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

            // Assert `get_range()` works.
            {
                let range = KEY..;
                assert_eq!(1, table.get_range(range).unwrap().count());

                let range = KEY..;
                let mut iter = table.get_range(range).unwrap();
                let value = iter.next().unwrap().unwrap();
                assert_eq(&value);
            }

            // Assert deleting works.
            table.delete(&KEY).unwrap();
            let value = table.get(&KEY);
            assert!(matches!(value, Err(RuntimeError::KeyNotFound)));
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

    BlockInfoV1s,
    BlockHeight => BlockInfoV1,
    123 => BlockInfoV1 {
        timestamp: 1,
        total_generated_coins: 123,
        weight: 321,
        cumulative_difficulty: 111,
        block_hash: [54; 32],
    },

    BlockInfoV2s,
    BlockHeight => BlockInfoV2,
    123 => BlockInfoV2 {
        timestamp: 1,
        total_generated_coins: 123,
        weight: 321,
        cumulative_difficulty: 111,
        cumulative_rct_outs: 2389,
        block_hash: [54; 32],
    },

    BlockInfoV3s,
    BlockHeight => BlockInfoV3,
    123 => BlockInfoV3 {
        timestamp: 1,
        total_generated_coins: 123,
        weight: 321,
        cumulative_difficulty_low: 111,
        cumulative_difficulty_high: 112,
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

    TxIds,
    TxHash => TxId,
    [32; 32] => 123,

    TxHeights,
    TxId => BlockHeight,
    123 => 123,

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
        output_flags: 0,
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
        output_flags: 0,
        tx_idx: 3,
        commitment: [3; 32],
    },
}
