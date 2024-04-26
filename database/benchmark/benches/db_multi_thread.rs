//! Same as `db.rs` but multi-threaded.
//! TODO: create multi-threaded benchmarks

use std::time::Instant;

//---------------------------------------------------------------------------------------------------- Import
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use function_name::named;

use cuprate_database::{
    tables::Outputs,
    types::{Output, PreRctOutputId},
    DatabaseIter, DatabaseRo, DatabaseRw, Env, EnvInner, TxRw,
};

use cuprate_database_benchmark::tmp_env_all_threads;

//---------------------------------------------------------------------------------------------------- Criterion
criterion_group! {
    benches,
    ro_get,
    ro_len,
    ro_first,
    ro_last,
    ro_is_empty,
    ro_contains,
    rw_get,
    rw_len,
    rw_first,
    rw_last,
    rw_is_empty,
    rw_contains,
    put,
    delete,
    pop_first,
    pop_last,
    get_range,
    iter,
    keys,
    values,
}
criterion_main!(benches);

//---------------------------------------------------------------------------------------------------- Constants
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

//---------------------------------------------------------------------------------------------------- DatabaseRo
// Read-only table operations.
// This uses `TxRw + TablesMut` briefly to insert values, then
// uses `TxRo + Tables` for the actual operation.
//
// See further below for using `TxRw + TablesMut` on the same operations.

/// [`DatabaseRo::get`]
#[named]
fn ro_get(c: &mut Criterion) {
    let (env, _tempdir) = tmp_env_all_threads();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    table.put(&KEY, &VALUE).unwrap();
    drop(table);
    tx_rw.commit().unwrap();

    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            let _value: Output = table.get(black_box(&KEY)).unwrap();
        });
    });
}

/// [`DatabaseRo::len`]
#[named]
fn ro_len(c: &mut Criterion) {
    let (env, _tempdir) = tmp_env_all_threads();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    table.put(&KEY, &VALUE).unwrap();
    drop(table);
    tx_rw.commit().unwrap();

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
    let (env, _tempdir) = tmp_env_all_threads();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    table.put(&KEY, &VALUE).unwrap();
    drop(table);
    tx_rw.commit().unwrap();

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
    let (env, _tempdir) = tmp_env_all_threads();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    table.put(&KEY, &VALUE).unwrap();
    drop(table);
    tx_rw.commit().unwrap();

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
    let (env, _tempdir) = tmp_env_all_threads();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    table.put(&KEY, &VALUE).unwrap();
    drop(table);
    tx_rw.commit().unwrap();

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
    let (env, _tempdir) = tmp_env_all_threads();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    table.put(&KEY, &VALUE).unwrap();
    drop(table);
    tx_rw.commit().unwrap();

    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            table.contains(black_box(&KEY)).unwrap();
        });
    });
}

//---------------------------------------------------------------------------------------------------- DatabaseRo (using TxRw)
// These are the same benchmarks as above, but it uses a
// `TxRw` and a `TablesMut` instead to ensure our read/write tables
// using read operations perform the same as normal read-only tables.

/// [`DatabaseRo::get`]
#[named]
fn rw_get(c: &mut Criterion) {
    let (env, _tempdir) = tmp_env_all_threads();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    table.put(&KEY, &VALUE).unwrap();
    drop(table);
    tx_rw.commit().unwrap();

    let tx_rw = env_inner.tx_rw().unwrap();
    let table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            let _value: Output = table.get(black_box(&KEY)).unwrap();
        });
    });
}

/// [`DatabaseRo::len`]
#[named]
fn rw_len(c: &mut Criterion) {
    let (env, _tempdir) = tmp_env_all_threads();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    table.put(&KEY, &VALUE).unwrap();
    drop(table);
    tx_rw.commit().unwrap();

    let tx_rw = env_inner.tx_rw().unwrap();
    let table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            black_box(table.len()).unwrap();
        });
    });
}

/// [`DatabaseRo::first`]
#[named]
fn rw_first(c: &mut Criterion) {
    let (env, _tempdir) = tmp_env_all_threads();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    table.put(&KEY, &VALUE).unwrap();
    drop(table);
    tx_rw.commit().unwrap();

    let tx_rw = env_inner.tx_rw().unwrap();
    let table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            let (_, _): (PreRctOutputId, Output) = black_box(table.first()).unwrap();
        });
    });
}

/// [`DatabaseRo::last`]
#[named]
fn rw_last(c: &mut Criterion) {
    let (env, _tempdir) = tmp_env_all_threads();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    table.put(&KEY, &VALUE).unwrap();
    drop(table);
    tx_rw.commit().unwrap();

    let tx_rw = env_inner.tx_rw().unwrap();
    let table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            let (_, _): (PreRctOutputId, Output) = black_box(table.last()).unwrap();
        });
    });
}

/// [`DatabaseRo::is_empty`]
#[named]
fn rw_is_empty(c: &mut Criterion) {
    let (env, _tempdir) = tmp_env_all_threads();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    table.put(&KEY, &VALUE).unwrap();
    drop(table);
    tx_rw.commit().unwrap();

    let tx_rw = env_inner.tx_rw().unwrap();
    let table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            black_box(table.is_empty()).unwrap();
        });
    });
}

/// [`DatabaseRo::contains`]
#[named]
fn rw_contains(c: &mut Criterion) {
    let (env, _tempdir) = tmp_env_all_threads();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    table.put(&KEY, &VALUE).unwrap();
    drop(table);
    tx_rw.commit().unwrap();

    let tx_rw = env_inner.tx_rw().unwrap();
    let table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            table.contains(black_box(&KEY)).unwrap();
        });
    });
}

//---------------------------------------------------------------------------------------------------- DatabaseRw
/// [`DatabaseRw::put`]
#[named]
fn put(c: &mut Criterion) {
    let (env, _tempdir) = tmp_env_all_threads();
    let env_inner = env.env_inner();
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
    let (env, _tempdir) = tmp_env_all_threads();
    let env_inner = env.env_inner();
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
    let (env, _tempdir) = tmp_env_all_threads();
    let env_inner = env.env_inner();
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
    let (env, _tempdir) = tmp_env_all_threads();
    let env_inner = env.env_inner();
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

// TODO: waiting on PR 102
// /// [`DatabaseRw::take`]
// #[named]
// fn take(c: &mut Criterion) {
//     let (env, _tempdir) = tmp_env_all_threads();
//     let env_inner = env.env_inner();
//     let tx_rw = env_inner.tx_rw().unwrap();
//     let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

//     let mut key = KEY;
//     for _ in 0..100 {
//         table.put(&key, &VALUE).unwrap();
//         key.amount += 1;
//     }

//     c.bench_function(function_name!(), |b| {
//         b.iter(|| {
//             table.put(&KEY, &VALUE).unwrap();
//             let value: Output = black_box(table.take(&black_box(KEY)).unwrap());
//         });
//     });
// }

//---------------------------------------------------------------------------------------------------- DatabaseIter
/// [`DatabaseRo::get_range`]
#[named]
fn get_range(c: &mut Criterion) {
    let (env, _tempdir) = tmp_env_all_threads();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    let mut key = KEY;
    for _ in 0..100 {
        table.put(&key, &VALUE).unwrap();
        key.amount += 1;
    }

    drop(table);
    TxRw::commit(tx_rw).unwrap();

    let tx_ro = env_inner.tx_ro().unwrap();
    let table = env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            let range = table.get_range(black_box(..)).unwrap();
            for result in range {
                let _value: Output = black_box(result.unwrap());
            }
        });
    });
}

/// [`DatabaseRo::iter`]
#[named]
fn iter(c: &mut Criterion) {
    let (env, _tempdir) = tmp_env_all_threads();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    let mut key = KEY;
    for _ in 0..100 {
        table.put(&key, &VALUE).unwrap();
        key.amount += 1;
    }

    drop(table);
    TxRw::commit(tx_rw).unwrap();

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

/// [`DatabaseRo::keys`]
#[named]
fn keys(c: &mut Criterion) {
    let (env, _tempdir) = tmp_env_all_threads();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    let mut key = KEY;
    for _ in 0..100 {
        table.put(&key, &VALUE).unwrap();
        key.amount += 1;
    }

    drop(table);
    TxRw::commit(tx_rw).unwrap();

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

/// [`DatabaseRo::values`]
#[named]
fn values(c: &mut Criterion) {
    let (env, _tempdir) = tmp_env_all_threads();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    let mut key = KEY;
    for _ in 0..100 {
        table.put(&key, &VALUE).unwrap();
        key.amount += 1;
    }

    drop(table);
    TxRw::commit(tx_rw).unwrap();

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
