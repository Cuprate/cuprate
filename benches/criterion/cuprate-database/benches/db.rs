//! Database operations.
//!
//! This module tests the functions from:
//! - [`cuprate_database::DatabaseRo`]
//! - [`cuprate_database::DatabaseRw`]
//! - [`cuprate_database::DatabaseIter`]

#![allow(unused_crate_dependencies, unused_attributes)]
#![expect(clippy::significant_drop_tightening)]

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
    name = benches;
    config = Criterion::default();
    targets =
    // `DatabaseRo`
    ro_get,
    ro_len,
    ro_first,
    ro_last,
    ro_is_empty,
    ro_contains,

    // `DatabaseRo` with a `TxRw`
    rw_get,
    rw_len,
    rw_first,
    rw_last,
    rw_is_empty,
    rw_contains,

    // `DatabaseIter`
    get_range,
    iter,
    keys,
    values,

    // `DatabaseRw`
    put,
    delete,
    pop_first,
    pop_last,
    take,
}
criterion_main!(benches);

//---------------------------------------------------------------------------------------------------- DatabaseRo
// Read-only table operations.
// This uses `TxRw + TablesMut` briefly to insert values, then
// uses `TxRo + Tables` for the actual operation.
//
// See further below for using `TxRw + TablesMut` on the same operations.

/// [`DatabaseRo::get`]
#[named]
fn ro_get(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    let env_inner = env.env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            let _: Output = table.get(black_box(&KEY)).unwrap();
        });
    });
}

/// [`DatabaseRo::len`]
#[named]
fn ro_len(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    let env_inner = env.env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            black_box(table.len()).unwrap();
        });
    });
}

/// [`DatabaseRo::first`]
#[named]
fn ro_first(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    let env_inner = env.env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            let (_, _): (PreRctOutputId, Output) = black_box(table.first()).unwrap();
        });
    });
}

/// [`DatabaseRo::last`]
#[named]
fn ro_last(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    let env_inner = env.env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            let (_, _): (PreRctOutputId, Output) = black_box(table.last()).unwrap();
        });
    });
}

/// [`DatabaseRo::is_empty`]
#[named]
fn ro_is_empty(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    let env_inner = env.env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            black_box(table.is_empty()).unwrap();
        });
    });
}

/// [`DatabaseRo::contains`]
#[named]
fn ro_contains(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    let env_inner = env.env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            table.contains(black_box(&KEY)).unwrap();
        });
    });
}

//---------------------------------------------------------------------------------------------------- DatabaseRo (TxRw)
// These are the same benchmarks as above, but it uses a
// `TxRw` and a `TablesMut` instead to ensure our read/write tables
// using read operations perform the same as normal read-only tables.

/// [`DatabaseRw::get`]
#[named]
fn rw_get(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    let env_inner = env.env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            let _: Output = table.get(black_box(&KEY)).unwrap();
        });
    });
}

/// [`DatabaseRw::len`]
#[named]
fn rw_len(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    let env_inner = env.env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            black_box(table.len()).unwrap();
        });
    });
}

/// [`DatabaseRw::first`]
#[named]
fn rw_first(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    let env_inner = env.env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            let (_, _): (PreRctOutputId, Output) = black_box(table.first()).unwrap();
        });
    });
}

/// [`DatabaseRw::last`]
#[named]
fn rw_last(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    let env_inner = env.env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            let (_, _): (PreRctOutputId, Output) = black_box(table.last()).unwrap();
        });
    });
}

/// [`DatabaseRw::is_empty`]
#[named]
fn rw_is_empty(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    let env_inner = env.env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            black_box(table.is_empty()).unwrap();
        });
    });
}

/// [`DatabaseRw::contains`]
#[named]
fn rw_contains(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value();
    let env_inner = env.env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            table.contains(black_box(&KEY)).unwrap();
        });
    });
}

//---------------------------------------------------------------------------------------------------- DatabaseIter
/// [`DatabaseIter::get_range`]
#[named]
fn get_range(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value_100();
    let env_inner = env.env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            let range = table.get_range(black_box(..)).unwrap();
            for result in range {
                let _: Output = black_box(result.unwrap());
            }
        });
    });
}

/// [`DatabaseIter::iter`]
#[named]
fn iter(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value_100();
    let env_inner = env.env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            let iter = black_box(table.iter()).unwrap();
            for result in iter {
                let _: (PreRctOutputId, Output) = black_box(result.unwrap());
            }
        });
    });
}

/// [`DatabaseIter::keys`]
#[named]
fn keys(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value_100();
    let env_inner = env.env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            let keys = black_box(table.keys()).unwrap();
            for result in keys {
                let _: PreRctOutputId = black_box(result.unwrap());
            }
        });
    });
}

/// [`DatabaseIter::values`]
#[named]
fn values(c: &mut Criterion) {
    let env = TmpEnv::new().with_key_value_100();
    let env_inner = env.env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            let values = black_box(table.values()).unwrap();
            for result in values {
                let _: Output = black_box(result.unwrap());
            }
        });
    });
}

//---------------------------------------------------------------------------------------------------- DatabaseRw
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

/// [`DatabaseRw::take`]
#[named]
fn take(c: &mut Criterion) {
    let env = TmpEnv::new();
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
