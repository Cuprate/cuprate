//! TODO

//---------------------------------------------------------------------------------------------------- Import
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use function_name::named;

use cuprate_database::{
    tables::Outputs,
    types::{Output, PreRctOutputId}, DatabaseIter, DatabaseRo, DatabaseRw, Env, EnvInner, TxRw,
};

use cuprate_database_benchmark::tmp_concrete_env;

//---------------------------------------------------------------------------------------------------- Criterion
criterion_group!(benches, put, get, get_range, delete);
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

//---------------------------------------------------------------------------------------------------- Env benchmarks
/// [`DatabaseRw::put`]
#[named]
fn put(c: &mut Criterion) {
    let (env, _tempdir) = tmp_concrete_env();
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

/// [`DatabaseRo::get`]
#[named]
fn get(c: &mut Criterion) {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    table.put(&KEY, &VALUE).unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            let _value: Output = table.get(black_box(&KEY)).unwrap();
        });
    });
}

/// [`DatabaseRo::get_range`]
#[named]
fn get_range(c: &mut Criterion) {
    let (env, _tempdir) = tmp_concrete_env();
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

/// [`DatabaseRw::delete`]
#[named]
fn delete(c: &mut Criterion) {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();
    let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

    let mut key = KEY;
    for _ in 0..100 {
        table.put(&key, &VALUE).unwrap();
        key.amount += 1;
    }

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            table.put(&KEY, &VALUE).unwrap();
            table.delete(&black_box(KEY)).unwrap();
        });
    });
}

// TODO: waiting on PR 102
// /// [`DatabaseRw::take`]
// #[named]
// fn take(c: &mut Criterion) {
//     let (env, _tempdir) = tmp_concrete_env();
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
