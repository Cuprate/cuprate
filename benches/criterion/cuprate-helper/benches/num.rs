//! Benchmarks for `cuprate_helper::cast`.
#![allow(unused_attributes, unused_crate_dependencies)]

use criterion::black_box as b;
use criterion::{criterion_group, criterion_main, Criterion};
use function_name::named;

use cuprate_helper::num;

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets =
        cmp_float,
        cmp_float_nan,
        get_mid,
        median,
}
criterion_main!(benches);

/// Benchmark [`curpate_helper::num::cmp_float`].
#[named]
fn cmp_float(c: &mut Criterion) {
    c.bench_function(function_name!(), |bench| {
        bench.iter(|| {
            b(num::cmp_float(b(0.0), b(0.0)));
        });
    });
}

/// Benchmark [`curpate_helper::num::cmp_float_nan`].
#[named]
fn cmp_float_nan(c: &mut Criterion) {
    c.bench_function(function_name!(), |bench| {
        bench.iter(|| {
            b(num::cmp_float_nan(b(0.0), b(0.0)));
        });
    });
}

/// Benchmark [`curpate_helper::num::get_mid`].
#[named]
fn get_mid(c: &mut Criterion) {
    c.bench_function(function_name!(), |bench| {
        bench.iter(|| {
            b(num::get_mid(b(0_u8), b(0_u8)));
            b(num::get_mid(b(1_i64), b(10_i64)));
            b(num::get_mid(b(0.0_f32), b(0.0_f32)));
            b(num::get_mid(b(0.0_f64), b(10.0_f64)));
        });
    });
}

/// Benchmark [`curpate_helper::num::median`].
#[named]
fn median(c: &mut Criterion) {
    c.bench_function(function_name!(), |bench| {
        bench.iter(|| {
            b(num::median(b(vec![0_u8, 1, 2, 3, 4, 5])));
            b(num::median(b(vec![0.0_f32, 1.0, 2.0, 3.0, 4.0, 5.0])));
            b(num::median(b(vec![0_u64; 100])));
        });
    });
}
