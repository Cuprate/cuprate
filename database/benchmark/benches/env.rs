//! TODO

//---------------------------------------------------------------------------------------------------- Import
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use function_name::named;

use cuprate_database::{
    config::Config,
    resize::{page_size, ResizeAlgorithm},
    tables::Outputs,
    ConcreteEnv, Env, EnvInner, TxRo, TxRw,
};

use cuprate_database_benchmark::tmp_concrete_env;

//---------------------------------------------------------------------------------------------------- Criterion
criterion_group! {
    benches,
    open,
    env_inner,
    tx_ro,
    tx_rw,
    open_tables,
    open_tables_mut,
    resize,
    current_map_size,
    disk_size_bytes,
}
criterion_main!(benches);

//---------------------------------------------------------------------------------------------------- Env benchmarks
/// [`Env::open`].
#[named]
fn open(c: &mut Criterion) {
    let tempdir = tempfile::tempdir().unwrap();
    let config = Config::low_power(Some(tempdir.path().into()));

    c.bench_function(function_name!(), |b| {
        b.iter_with_large_drop(|| {
            ConcreteEnv::open(config.clone()).unwrap();
        });
    });
}

/// [`Env::env_inner`].
#[named]
fn env_inner(c: &mut Criterion) {
    let (env, _tempdir) = tmp_concrete_env();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            black_box(env.env_inner());
        });
    });
}

/// Create and commit read-only transactions.
#[named]
fn tx_ro(c: &mut Criterion) {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            let tx_ro = black_box(env_inner.tx_ro()).unwrap();
            TxRo::commit(black_box(tx_ro)).unwrap();
        });
    });
}

/// Create and commit read/write transactions.
#[named]
fn tx_rw(c: &mut Criterion) {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            let tx_rw = black_box(env_inner.tx_rw()).unwrap();
            TxRw::commit(black_box(tx_rw)).unwrap();
        });
    });
}

/// Open all database tables in read-only mode.
#[named]
fn open_tables(c: &mut Criterion) {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            black_box(env_inner.open_db_ro::<Outputs>(&tx_ro)).unwrap();
            // env_inner.open_tables(&tx_ro).unwrap();
            // TODO: waiting on PR 102
        });
    });
}

/// Open all database tables in read/write mode.
#[named]
fn open_tables_mut(c: &mut Criterion) {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            black_box(env_inner.open_db_rw::<Outputs>(&tx_rw)).unwrap();
            // env_inner.open_tables_mut(&mut tx_rw).unwrap();
            // TODO: waiting on PR 102
        });
    });
}

/// `Env` memory map resizes.
#[named]
fn resize(c: &mut Criterion) {
    let (env, _tempdir) = tmp_concrete_env();

    // Resize by the OS page size.
    let page_size = page_size();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            // This test is only valid for `Env`'s that need to resize manually.
            if ConcreteEnv::MANUAL_RESIZE {
                env.resize_map(black_box(Some(ResizeAlgorithm::FixedBytes(page_size))));
            }
        });
    });
}

/// Access current memory map size of the database.
#[named]
fn current_map_size(c: &mut Criterion) {
    let (env, _tempdir) = tmp_concrete_env();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            // This test is only valid for `Env`'s that need to resize manually.
            if ConcreteEnv::MANUAL_RESIZE {
                black_box(env.current_map_size());
            }
        });
    });
}

/// Access on-disk size of the database.
#[named]
fn disk_size_bytes(c: &mut Criterion) {
    let (env, _tempdir) = tmp_concrete_env();

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            black_box(env.disk_size_bytes()).unwrap();
        });
    });
}
