//! Benchmarks for [`block`] functions.

#![allow(unused_attributes, unused_crate_dependencies)]

use std::time::Instant;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use function_name::named;

use cuprate_blockchain::{
    cuprate_database::{Env, EnvInner},
    ops::block,
    tables::OpenTables,
};
use cuprate_test_utils::data::{BLOCK_V16_TX0, BLOCK_V1_TX2, BLOCK_V9_TX3};
use cuprate_types::VerifiedBlockInformation;

use cuprate_criterion_blockchain::generate_fake_blocks;

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets =
        add_block_v1_tx2,
        add_block_v9_tx3,
        add_block_v16_tx0,
}
criterion_main!(benches);

/// Inner function for benchmarking [`block::add_block`].
#[expect(clippy::significant_drop_tightening)]
fn add_block_inner(c: &mut Criterion, function_name: &str, block: &VerifiedBlockInformation) {
    let env = cuprate_criterion_blockchain::TmpEnv::new();

    c.bench_function(function_name, |b| {
        // We use `iter_custom` because we need to generate an
        // appropriate amount of blocks and only time the `add_block`.
        b.iter_custom(|count| {
            let blocks = black_box(generate_fake_blocks(block, count));

            let env_inner = env.env.env_inner();
            let tx_rw = env_inner.tx_rw().unwrap();
            let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();

            let start = Instant::now();
            for block in &blocks {
                black_box(block::add_block(block, &mut tables)).unwrap();
            }
            start.elapsed()
        });
    });
}

#[named]
fn add_block_v1_tx2(c: &mut Criterion) {
    add_block_inner(c, function_name!(), &BLOCK_V1_TX2);
}

#[named]
fn add_block_v9_tx3(c: &mut Criterion) {
    add_block_inner(c, function_name!(), &BLOCK_V9_TX3);
}

#[named]
fn add_block_v16_tx0(c: &mut Criterion) {
    add_block_inner(c, function_name!(), &BLOCK_V16_TX0);
}
