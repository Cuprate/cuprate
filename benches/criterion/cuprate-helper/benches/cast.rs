//! Benchmarks for `cuprate_helper::cast`.
#![allow(unused_attributes, unused_crate_dependencies)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use function_name::named;

use cuprate_helper::cast::{
    i32_to_isize, i64_to_isize, isize_to_i64, u32_to_usize, u64_to_usize, usize_to_u64,
};

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = integer, unsigned,
}
criterion_main!(benches);

/// Benchmark integer casts.
#[named]
fn integer(c: &mut Criterion) {
    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            black_box(i32_to_isize(black_box(0)));
            black_box(i64_to_isize(black_box(0)));
            black_box(isize_to_i64(black_box(0)));
        });
    });
}

/// Benchmark unsigned integer casts.
#[named]
fn unsigned(c: &mut Criterion) {
    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            black_box(u32_to_usize(black_box(0)));
            black_box(u64_to_usize(black_box(0)));
            black_box(usize_to_u64(black_box(0)));
        });
    });
}
