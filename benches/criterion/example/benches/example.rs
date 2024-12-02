//! Benchmarks.
#![allow(unused_attributes, unused_crate_dependencies)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use function_name::named;

use cuprate_criterion_example::SomeHardToCreateObject;

// This is how you register criterion benchmarks.
criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = benchmark_1, benchmark_range,
}
criterion_main!(benches);

/// Benchmark a single input.
///
/// <https://bheisler.github.io/criterion.rs/book/user_guide/benchmarking_with_inputs.html#benchmarking-with-one-input>
#[named]
fn benchmark_1(c: &mut Criterion) {
    // It is recommended to use `function_name!()` as a benchmark
    // identifier instead of manually re-typing the function name.
    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            black_box(SomeHardToCreateObject::from(1));
        });
    });
}

/// Benchmark a range of inputs.
///
/// <https://bheisler.github.io/criterion.rs/book/user_guide/benchmarking_with_inputs.html#benchmarking-with-a-range-of-values>
#[named]
fn benchmark_range(c: &mut Criterion) {
    let mut group = c.benchmark_group(function_name!());

    for i in 0..4 {
        group.throughput(Throughput::Elements(i));
        group.bench_with_input(BenchmarkId::from_parameter(i), &i, |b, &i| {
            b.iter(|| {
                black_box(SomeHardToCreateObject::from(i));
            });
        });
    }

    group.finish();
}
