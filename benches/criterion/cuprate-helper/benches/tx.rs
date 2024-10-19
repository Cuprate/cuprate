//! Benchmarks for `cuprate_helper::cast`.
#![allow(unused_attributes, unused_crate_dependencies)]

use criterion::black_box as b;
use criterion::{criterion_group, criterion_main, Criterion};
use function_name::named;

use cuprate_helper::tx;
use cuprate_test_utils::data::{TX_V1_SIG0, TX_V1_SIG2, TX_V2_RCT3};

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = tx_fee,
}
criterion_main!(benches);

/// Benchmark [`curpate_helper::tx::tx_fee`].
#[named]
fn tx_fee(c: &mut Criterion) {
    c.bench_function(function_name!(), |bench| {
        bench.iter(|| {
            b(tx::tx_fee(b(&TX_V1_SIG0.tx)));
            b(tx::tx_fee(b(&TX_V1_SIG2.tx)));
            b(tx::tx_fee(b(&TX_V2_RCT3.tx)));
        });
    });
}
