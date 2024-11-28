//! [`Storable`] benchmarks.

#![allow(unused_crate_dependencies, unused_attributes)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use function_name::named;

use cuprate_blockchain::types::{Output, PreRctOutputId};
use cuprate_database::Storable;

use cuprate_criterion_database::{KEY, VALUE};

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets =
    pre_rct_output_id_as_bytes,
    pre_rct_output_id_from_bytes,
    output_as_bytes,
    output_from_bytes
}
criterion_main!(benches);

/// [`PreRctOutputId`] cast as bytes.
#[named]
fn pre_rct_output_id_as_bytes(c: &mut Criterion) {
    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            black_box(Storable::as_bytes(black_box(&KEY)));
        });
    });
}

/// [`PreRctOutputId`] cast from bytes.
#[named]
fn pre_rct_output_id_from_bytes(c: &mut Criterion) {
    let bytes = Storable::as_bytes(&KEY);

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            let _: PreRctOutputId = black_box(Storable::from_bytes(black_box(bytes)));
        });
    });
}

/// [`Output`] cast as bytes.
#[named]
fn output_as_bytes(c: &mut Criterion) {
    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            black_box(Storable::as_bytes(black_box(&VALUE)));
        });
    });
}

/// [`Output`] cast from bytes.
#[named]
fn output_from_bytes(c: &mut Criterion) {
    let bytes = Storable::as_bytes(&VALUE);

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            let _: Output = black_box(Storable::from_bytes(black_box(bytes)));
        });
    });
}
