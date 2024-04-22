//! `cuprate_database::service` benchmarks.

//---------------------------------------------------------------------------------------------------- Import
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use function_name::named;

use cuprate_database::{
    config::Config,
    resize::{page_size, ResizeAlgorithm},
    tables::Outputs,
    ConcreteEnv, Env, EnvInner, TxRo, TxRw,
};

use cuprate_database_benchmark::tmp_env_all_threads;

//---------------------------------------------------------------------------------------------------- Criterion
criterion_group! {
    benches,
}
criterion_main!(benches);

//---------------------------------------------------------------------------------------------------- Benchmarks

// TODO
