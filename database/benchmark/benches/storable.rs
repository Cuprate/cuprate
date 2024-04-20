//! TODO

//---------------------------------------------------------------------------------------------------- Import
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use function_name::named;

use cuprate_database::{
    types::{Output, PreRctOutputId},
    Storable,
};

//---------------------------------------------------------------------------------------------------- Criterion
criterion_group! {
    benches,
    pre_rct_output_id_as_bytes,
    pre_rct_output_id_from_bytes,
    output_as_bytes,
    output_from_bytes
}
criterion_main!(benches);

//---------------------------------------------------------------------------------------------------- Constants
/// 16 bytes.
const PRE_RCT_OUTPUT_ID: PreRctOutputId = PreRctOutputId {
    amount: 1,
    amount_index: 123,
};

/// 48 bytes.
const OUTPUT: Output = Output {
    key: [35; 32],
    height: 45_761_798,
    output_flags: 0,
    tx_idx: 2_353_487,
};

//---------------------------------------------------------------------------------------------------- Storable benchmarks
/// [`PreRctOutputId`] cast as bytes.
#[named]
fn pre_rct_output_id_as_bytes(c: &mut Criterion) {
    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            black_box(Storable::as_bytes(black_box(&PRE_RCT_OUTPUT_ID)));
        });
    });
}

/// [`PreRctOutputId`] cast from bytes.
#[named]
fn pre_rct_output_id_from_bytes(c: &mut Criterion) {
    let bytes = Storable::as_bytes(&PRE_RCT_OUTPUT_ID);

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
            black_box(Storable::as_bytes(black_box(&OUTPUT)));
        });
    });
}

/// [`Output`] cast from bytes.
#[named]
fn output_from_bytes(c: &mut Criterion) {
    let bytes = Storable::as_bytes(&OUTPUT);

    c.bench_function(function_name!(), |b| {
        b.iter(|| {
            let _: Output = black_box(Storable::from_bytes(black_box(bytes)));
        });
    });
}
