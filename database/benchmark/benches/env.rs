//! TODO

//---------------------------------------------------------------------------------------------------- Import
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use cuprate_database::{
    resize::{page_size, ResizeAlgorithm},
    tables::Outputs,
    ConcreteEnv, Env, EnvInner, TxRo, TxRw,
};

use cuprate_database_benchmark::tmp_concrete_env;

//---------------------------------------------------------------------------------------------------- Env benchmarks
/// [`Env::open`].
fn open(c: &mut Criterion) {
    c.bench_function("tmp_concrete_env", |b| {
        b.iter(|| {
            // We don't want `tempfile`'s file destruction code
            // to overtake the benchmark, so forget the object
            // so it doesn't get deconstructed.
            //
            // FIXME: this might hit file descriptor limits.
            std::mem::forget(tmp_concrete_env());
        });
    });
}

/// Create and commit read-only transactions.
fn tx_ro(c: &mut Criterion) {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();

    c.bench_function("tx_ro", |b| {
        b.iter(|| {
            let tx_ro = env_inner.tx_ro().unwrap();
            TxRo::commit(tx_ro).unwrap();
        });
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
        });
    });
}

/// Open all database tables in read-only mode.
fn open_tables(c: &mut Criterion) {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();

    c.bench_function("open_tables", |b| {
        b.iter(|| {
            env_inner.open_db_ro::<Outputs>(&tx_ro).unwrap();
            // env_inner.open_tables(&tx_ro).unwrap();
            // TODO: waiting on PR 102
        });
    });
}

/// Open all database tables in read/write mode.
fn open_tables_mut(c: &mut Criterion) {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw().unwrap();

    c.bench_function("open_tables_mut", |b| {
        b.iter(|| {
            env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();
            // env_inner.open_tables_mut(&mut tx_rw).unwrap();
            // TODO: waiting on PR 102
        });
    });
}

/// Test `Env` resizes.
fn resize(c: &mut Criterion) {
    let (env, _tempdir) = tmp_concrete_env();

    // Resize by the OS page size.
    let page_size = page_size();

    c.bench_function("resize", |b| {
        b.iter(|| {
            // This test is only valid for `Env`'s that need to resize manually.
            if ConcreteEnv::MANUAL_RESIZE {
                env.resize_map(Some(ResizeAlgorithm::FixedBytes(page_size)));
            }
        });
    });
}

criterion_group!(
    benches,
    open,
    tx_ro,
    tx_rw,
    open_tables,
    open_tables_mut,
    resize
);
criterion_main!(benches);
