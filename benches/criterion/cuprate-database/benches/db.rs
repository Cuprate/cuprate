//! Database operations.
//!
//! This module tests the functions from:
//! - [`cuprate_database::DatabaseRo`]
//! - [`cuprate_database::DatabaseRw`]
//! - [`cuprate_database::DatabaseIter`]
//!
//! There are 2 flavors of (read-only) benchmarks:
//! - Single threaded that uses [`TmpEnv::new`]
//! - Multi threaded that uses [`TmpEnv::new_all_threads`]
//!
//! They benchmark the same thing, just with different
//! amount of threads. This is done as 1 "inner" function
//! that contains the logic and 2 others to setup the [`TmpEnv`]
//! with different amounts of threads, e.g.:
//! - [`ro_get`] (inner benchmark logic)
//! - [`ro_get_single_thread`] (just calls `ro_get` with 1 thread)
//! - [`ro_get_multi_thread`] (just calls `ro_get` with all threads)
//!
//! Writes are single-threaded, so they only use [`TmpEnv::new`].

#![expect(clippy::significant_drop_tightening, clippy::needless_pass_by_value)]

// TODO
use cuprate_helper as _;
use tempfile as _;

use std::time::Instant;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use function_name::named;

use cuprate_blockchain::{
    tables::Outputs,
    types::{Output, PreRctOutputId},
};
use cuprate_database::{DatabaseIter, DatabaseRo, DatabaseRw, Env, EnvInner};

use cuprate_criterion_database::{TmpEnv, KEY, VALUE};

criterion_group! {
    benches,

    // `DatabaseRo`
    ro_get_single_thread,
    ro_get_multi_thread,
    ro_len_single_thread,
    ro_len_multi_thread,
    ro_first_single_thread,
    ro_first_multi_thread,
    ro_last_single_thread,
    ro_last_multi_thread,
    ro_is_empty_single_thread,
    ro_is_empty_multi_thread,
    ro_contains_single_thread,
    ro_contains_multi_thread,

    // `DatabaseRo` with a `TxRw`
    rw_get_single_thread,
    rw_get_multi_thread,
    rw_len_single_thread,
    rw_len_multi_thread,
    rw_first_single_thread,
    rw_first_multi_thread,
    rw_last_single_thread,
    rw_last_multi_thread,
    rw_is_empty_single_thread,
    rw_is_empty_multi_thread,
    rw_contains_single_thread,
    rw_contains_multi_thread,

    // `DatabaseIter`
    get_range_single_thread,
    get_range_multi_thread,
    iter_single_thread,
    iter_multi_thread,
    keys_single_thread,
    keys_multi_thread,
    values_single_thread,
    values_multi_thread,

    // `DatabaseRw`
    put,
    delete,
    pop_first,
    pop_last,
    take,
}

criterion_main!(benches);

//---------------------------------------------------------------------------------------------------- DatabaseRo::get
// Read-only table operations.
// This uses `TxRw + TablesMut` briefly to insert values, then
// uses `TxRo + Tables` for the actual operation.
//
// See further below for using `TxRw + TablesMut` on the same operations.

fn ro_get(env: TmpEnv, name: &'static str, c: &mut Criterion) {
    let env_inner = env.env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(name, |b| {
        b.iter(|| {
            let _value: Output = table.get(black_box(&KEY)).unwrap();
        });
    });
}

/// [`DatabaseRo::get`] (single-thread).
#[named]
fn ro_get_single_thread(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    ro_get(env, function_name!(), c);
}

/// [`DatabaseRo::get`] (multi-thread).
#[named]
fn ro_get_multi_thread(c: &mut Criterion) {
    let env = TmpEnv::new_all_threads().with_key_value();
    ro_get(env, function_name!(), c);
}

//---------------------------------------------------------------------------------------------------- DatabaseRo::len
fn ro_len(env: TmpEnv, name: &'static str, c: &mut Criterion) {
    let env_inner = env.env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(name, |b| {
        b.iter(|| {
            black_box(table.len()).unwrap();
        });
    });
}

/// [`DatabaseRo::len`] (single-thread).
#[named]
fn ro_len_single_thread(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    ro_len(env, function_name!(), c);
}

/// [`DatabaseRo::len`] (multi-thread).
#[named]
fn ro_len_multi_thread(c: &mut Criterion) {
    let env = TmpEnv::new_all_threads().with_key_value();
    ro_len(env, function_name!(), c);
}

//---------------------------------------------------------------------------------------------------- DatabaseRo::first
fn ro_first(env: TmpEnv, name: &'static str, c: &mut Criterion) {
    let env_inner = env.env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(name, |b| {
        b.iter(|| {
            let (_, _): (PreRctOutputId, Output) = black_box(table.first()).unwrap();
        });
    });
}

/// [`DatabaseRo::first`] (single-thread).
#[named]
fn ro_first_single_thread(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    ro_first(env, function_name!(), c);
}

/// [`DatabaseRo::first`] (multi-thread).
#[named]
fn ro_first_multi_thread(c: &mut Criterion) {
    let env = TmpEnv::new_all_threads().with_key_value();
    ro_first(env, function_name!(), c);
}

//---------------------------------------------------------------------------------------------------- DatabaseRo::last
/// [`DatabaseRo::last`]
fn ro_last(env: TmpEnv, name: &'static str, c: &mut Criterion) {
    let env_inner = env.env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(name, |b| {
        b.iter(|| {
            let (_, _): (PreRctOutputId, Output) = black_box(table.last()).unwrap();
        });
    });
}

/// [`DatabaseRo::last`] (single-thread).
#[named]
fn ro_last_single_thread(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    ro_last(env, function_name!(), c);
}

/// [`DatabaseRo::last`] (multi-thread).
#[named]
fn ro_last_multi_thread(c: &mut Criterion) {
    let env = TmpEnv::new_all_threads().with_key_value();
    ro_last(env, function_name!(), c);
}

//---------------------------------------------------------------------------------------------------- DatabaseRo::is_empty
/// [`DatabaseRo::is_empty`]
fn ro_is_empty(env: TmpEnv, name: &'static str, c: &mut Criterion) {
    let env_inner = env.env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(name, |b| {
        b.iter(|| {
            black_box(table.is_empty()).unwrap();
        });
    });
}

/// [`DatabaseRo::is_empty`] (single-thread).
#[named]
fn ro_is_empty_single_thread(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    ro_is_empty(env, function_name!(), c);
}

/// [`DatabaseRo::is_empty`] (multi-thread).
#[named]
fn ro_is_empty_multi_thread(c: &mut Criterion) {
    let env = TmpEnv::new_all_threads().with_key_value();
    ro_is_empty(env, function_name!(), c);
}

//---------------------------------------------------------------------------------------------------- DatabaseRo::contains
/// [`DatabaseRo::contains`]
fn ro_contains(env: TmpEnv, name: &'static str, c: &mut Criterion) {
    let env_inner = env.env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(name, |b| {
        b.iter(|| {
            table.contains(black_box(&KEY)).unwrap();
        });
    });
}

/// [`DatabaseRo::contains`] (single-thread).
#[named]
fn ro_contains_single_thread(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    ro_contains(env, function_name!(), c);
}

/// [`DatabaseRo::contains`] (multi-thread).
#[named]
fn ro_contains_multi_thread(c: &mut Criterion) {
    let env = TmpEnv::new_all_threads().with_key_value();
    ro_contains(env, function_name!(), c);
}

//---------------------------------------------------------------------------------------------------- DatabaseRw::get
// These are the same benchmarks as above, but it uses a
// `TxRw` and a `TablesMut` instead to ensure our read/write tables
// using read operations perform the same as normal read-only tables.

fn rw_get(env: TmpEnv, name: &'static str, c: &mut Criterion) {
    let env_inner = env.env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    c.bench_function(name, |b| {
        b.iter(|| {
            let _value: Output = table.get(black_box(&KEY)).unwrap();
        });
    });
}

/// [`DatabaseRw::get`] (single-thread).
#[named]
fn rw_get_single_thread(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    rw_get(env, function_name!(), c);
}

/// [`DatabaseRw::get`] (multi-thread).
#[named]
fn rw_get_multi_thread(c: &mut Criterion) {
    let env = TmpEnv::new_all_threads().with_key_value();
    rw_get(env, function_name!(), c);
}

//---------------------------------------------------------------------------------------------------- DatabaseRw::len
fn rw_len(env: TmpEnv, name: &'static str, c: &mut Criterion) {
    let env_inner = env.env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    c.bench_function(name, |b| {
        b.iter(|| {
            black_box(table.len()).unwrap();
        });
    });
}

/// [`DatabaseRw::len`] (single-thread).
#[named]
fn rw_len_single_thread(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    rw_len(env, function_name!(), c);
}

/// [`DatabaseRw::len`] (multi-thread).
#[named]
fn rw_len_multi_thread(c: &mut Criterion) {
    let env = TmpEnv::new_all_threads().with_key_value();
    rw_len(env, function_name!(), c);
}

//---------------------------------------------------------------------------------------------------- DatabaseRw::first
fn rw_first(env: TmpEnv, name: &'static str, c: &mut Criterion) {
    let env_inner = env.env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    c.bench_function(name, |b| {
        b.iter(|| {
            let (_, _): (PreRctOutputId, Output) = black_box(table.first()).unwrap();
        });
    });
}

/// [`DatabaseRw::first`] (single-thread).
#[named]
fn rw_first_single_thread(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    rw_first(env, function_name!(), c);
}

/// [`DatabaseRw::first`] (multi-thread).
#[named]
fn rw_first_multi_thread(c: &mut Criterion) {
    let env = TmpEnv::new_all_threads().with_key_value();
    rw_first(env, function_name!(), c);
}

//---------------------------------------------------------------------------------------------------- DatabaseRw::last
fn rw_last(env: TmpEnv, name: &'static str, c: &mut Criterion) {
    let env_inner = env.env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    c.bench_function(name, |b| {
        b.iter(|| {
            let (_, _): (PreRctOutputId, Output) = black_box(table.last()).unwrap();
        });
    });
}

/// [`DatabaseRw::last`] (single-thread).
#[named]
fn rw_last_single_thread(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    rw_last(env, function_name!(), c);
}

/// [`DatabaseRw::last`] (multi-thread).
#[named]
fn rw_last_multi_thread(c: &mut Criterion) {
    let env = TmpEnv::new_all_threads().with_key_value();
    rw_last(env, function_name!(), c);
}

//---------------------------------------------------------------------------------------------------- DatabaseRw::is_empty
fn rw_is_empty(env: TmpEnv, name: &'static str, c: &mut Criterion) {
    let env_inner = env.env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    c.bench_function(name, |b| {
        b.iter(|| {
            black_box(table.is_empty()).unwrap();
        });
    });
}

/// [`DatabaseRw::is_empty`] (single-thread).
#[named]
fn rw_is_empty_single_thread(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    rw_is_empty(env, function_name!(), c);
}

/// [`DatabaseRw::is_empty`] (multi-thread).
#[named]
fn rw_is_empty_multi_thread(c: &mut Criterion) {
    let env = TmpEnv::new_all_threads().with_key_value();
    rw_is_empty(env, function_name!(), c);
}

//---------------------------------------------------------------------------------------------------- DatabaseRw::contains
fn rw_contains(env: TmpEnv, name: &'static str, c: &mut Criterion) {
    let env_inner = env.env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    c.bench_function(name, |b| {
        b.iter(|| {
            table.contains(black_box(&KEY)).unwrap();
        });
    });
}

/// [`DatabaseRw::contains`] (single-thread).
#[named]
fn rw_contains_single_thread(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    rw_contains(env, function_name!(), c);
}

/// [`DatabaseRw::contains`] (multi-thread).
#[named]
fn rw_contains_multi_thread(c: &mut Criterion) {
    let env = TmpEnv::new_all_threads().with_key_value();
    rw_contains(env, function_name!(), c);
}

//---------------------------------------------------------------------------------------------------- DatabaseIter::get_range
fn get_range(env: TmpEnv, name: &'static str, c: &mut Criterion) {
    let env_inner = env.env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(name, |b| {
        b.iter(|| {
            let range = table.get_range(black_box(..)).unwrap();
            for result in range {
                let _: Output = black_box(result.unwrap());
            }
        });
    });
}

/// [`DatabaseIter::get_range`] (single-thread).
#[named]
fn get_range_single_thread(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value_100();
    get_range(env, function_name!(), c);
}

/// [`DatabaseIter::get_range`] (multi-thread).
#[named]
fn get_range_multi_thread(c: &mut Criterion) {
    let env = TmpEnv::new_all_threads().with_key_value_100();
    get_range(env, function_name!(), c);
}

//---------------------------------------------------------------------------------------------------- DatabaseIter::iter
fn iter(env: TmpEnv, name: &'static str, c: &mut Criterion) {
    let env_inner = env.env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(name, |b| {
        b.iter(|| {
            let iter = black_box(table.iter()).unwrap();
            for result in iter {
                let _: (PreRctOutputId, Output) = black_box(result.unwrap());
            }
        });
    });
}

/// [`DatabaseIter::iter`] (single-thread).
#[named]
fn iter_single_thread(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value_100();
    iter(env, function_name!(), c);
}

/// [`DatabaseIter::iter`] (multi-thread).
#[named]
fn iter_multi_thread(c: &mut Criterion) {
    let env = TmpEnv::new_all_threads().with_key_value_100();
    iter(env, function_name!(), c);
}

//---------------------------------------------------------------------------------------------------- DatabaseIter::keys
/// [`DatabaseRo::keys`]
fn keys(env: TmpEnv, name: &'static str, c: &mut Criterion) {
    let env_inner = env.env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(name, |b| {
        b.iter(|| {
            let keys = black_box(table.keys()).unwrap();
            for result in keys {
                let _: PreRctOutputId = black_box(result.unwrap());
            }
        });
    });
}

/// [`DatabaseIter::keys`] (single-thread).
#[named]
fn keys_single_thread(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value_100();
    keys(env, function_name!(), c);
}

/// [`DatabaseIter::iter`] (multi-thread).
#[named]
fn keys_multi_thread(c: &mut Criterion) {
    let env = TmpEnv::new_all_threads().with_key_value_100();
    keys(env, function_name!(), c);
}

//---------------------------------------------------------------------------------------------------- DatabaseIter::values
/// [`DatabaseRo::values`]
fn values(env: TmpEnv, name: &'static str, c: &mut Criterion) {
    let env_inner = env.env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(name, |b| {
        b.iter(|| {
            let values = black_box(table.values()).unwrap();
            for result in values {
                let _: Output = black_box(result.unwrap());
            }
        });
    });
}

/// [`DatabaseIter::values`] (single-thread).
#[named]
fn values_single_thread(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value_100();
    values(env, function_name!(), c);
}

/// [`DatabaseIter::iter`] (multi-thread).
#[named]
fn values_multi_thread(c: &mut Criterion) {
    let env = TmpEnv::new_all_threads().with_key_value_100();
    values(env, function_name!(), c);
}

//---------------------------------------------------------------------------------------------------- DatabaseRw::put
/// [`DatabaseRw::put`]
#[named]
fn put(c: &mut Criterion) {
    let env = TmpEnv::new();
    let env_inner = env.env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    let mut key = KEY;

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            table.put(black_box(&key), black_box(&VALUE)).unwrap();
            key.amount += 1;
        });
    });
}

//---------------------------------------------------------------------------------------------------- DatabaseRw::delete
/// [`DatabaseRw::delete`]
#[named]
fn delete(c: &mut Criterion) {
    let env = TmpEnv::new();
    let env_inner = env.env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    let mut key = KEY;

    c.bench_function(function_name!(), |b| {
        b.iter_custom(|iters| {
            for _ in 0..iters {
                table.put(&key, &VALUE).unwrap();
                key.amount += 1;
            }

            key = KEY;

            let start = Instant::now();
            for _ in 0..iters {
                table.delete(&key).unwrap();
                key.amount += 1;
            }
            start.elapsed()
        });
    });
}

//---------------------------------------------------------------------------------------------------- DatabaseRw::pop_first
/// [`DatabaseRw::pop_first`]
#[named]
fn pop_first(c: &mut Criterion) {
    let env = TmpEnv::new();
    let env_inner = env.env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    let mut key = KEY;

    c.bench_function(function_name!(), |b| {
        b.iter_custom(|iters| {
            for _ in 0..iters {
                table.put(&key, &VALUE).unwrap();
                key.amount += 1;
            }

            key = KEY;

            let start = Instant::now();
            for _ in 0..iters {
                table.pop_first().unwrap();
                key.amount += 1;
            }
            start.elapsed()
        });
    });
}

//---------------------------------------------------------------------------------------------------- DatabaseRw::pop_last
/// [`DatabaseRw::pop_last`]
#[named]
fn pop_last(c: &mut Criterion) {
    let env = TmpEnv::new();
    let env_inner = env.env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    let mut key = KEY;

    c.bench_function(function_name!(), |b| {
        b.iter_custom(|iters| {
            for _ in 0..iters {
                table.put(&key, &VALUE).unwrap();
                key.amount += 1;
            }

            key = KEY;

            let start = Instant::now();
            for _ in 0..iters {
                table.pop_last().unwrap();
                key.amount += 1;
            }
            start.elapsed()
        });
    });
}

//---------------------------------------------------------------------------------------------------- DatabaseRw::take
/// [`DatabaseRw::take`]
#[named]
fn take(c: &mut Criterion) {
    let env = TmpEnv::new_all_threads();
    let env_inner = env.env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            table.put(&KEY, &VALUE).unwrap();
            let _: Output = black_box(table.take(&black_box(KEY)).unwrap());
        });
    });
}
