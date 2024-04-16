//! TODO

//---------------------------------------------------------------------------------------------------- Import
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use cuprate_database::{ConcreteEnv, Env, EnvInner, TxRo, TxRw};

use crate::tmp_concrete_env;

//----------------------------------------------------------------------------------------------------
/// [`Env::open`].
fn open(c: &mut Criterion) {
    c.bench_function("tmp_concrete_env", |b| b.iter(|| tmp_concrete_env()));
}

/// Create and commit read-only transactions.
fn tx_ro(c: &mut Criterion) {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();

    c.bench_function("tx_ro", |b| {
        b.iter(|| {
            let tx_ro = env_inner.tx_ro().unwrap();
            TxRo::commit(tx_ro).unwrap();
        })
    });
}

/// Create and commit read/write transactions.
fn tx_rw(c: &mut Criterion) {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();

    c.bench_function("tx_rw", |b| {
        b.iter(|| {
            let tx_rw = env_inner.tx_rw().unwrap();
            TxRw::commit(tx_rw).unwrap();
        })
    });
}

/// Open all database tables in read-only mode.
fn open_tables(c: &mut Criterion) {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();

    c.bench_function("open_tables", |b| {
        b.iter(|| {
            env_inner.open_tables(&tx_ro).unwrap();
        })
    });
}

/// Open all database tables in read/write mode.
fn open_tables_mut(c: &mut Criterion) {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();
    let mut tx_rw = env_inner.tx_ro().unwrap();

    c.bench_function("open_tables_mut", |b| {
        b.iter(|| {
            env_inner.open_tables_mut(&mut tx_rw).unwrap();
        })
    });
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

    // `get_range()` tests.
    let mut key = KEY;
    key.amount += 100;
    let range = KEY..key;

    // Assert count is correct.
    assert_eq!(100, table.get_range(range.clone()).unwrap().count());

    // Assert each returned value from the iterator is owned.
    {
        let mut iter = table.get_range(range.clone()).unwrap();
        let value: Output = iter.next().unwrap().unwrap(); // 1. take value out
        drop(iter); // 2. drop the `impl Iterator + 'a`
        assert_same(value); // 3. assert even without the iterator, the value is alive
    }

    // Assert each value is the same.
    {
        let mut iter = table.get_range(range).unwrap();
        for _ in 0..100 {
            let value: Output = iter.next().unwrap().unwrap();
            assert_same(value);
        }
    }

    // Assert deleting works.
    table.delete(&KEY).unwrap();
    let value = table.get(&KEY);
    assert!(matches!(value, Err(RuntimeError::KeyNotFound)));
}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
