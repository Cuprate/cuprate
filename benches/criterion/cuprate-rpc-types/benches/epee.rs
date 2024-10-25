//! This module contains benchmarks for any
//! non-trivial/manual `epee` implementation.
//!
//! Types with the standard `epee` derive implementation are not included.

#![allow(unused_attributes, unused_crate_dependencies)]

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use function_name::named;

use cuprate_epee_encoding::{from_bytes, to_bytes};
use cuprate_rpc_types::bin::{GetBlocksRequest, GetBlocksResponse};

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets =
    epee_to_bytes_get_blocks_request,
    epee_from_bytes_get_blocks_request,
    epee_to_bytes_get_blocks_response,
    epee_from_bytes_get_blocks_response,
}
criterion_main!(benches);

/// TODO
macro_rules! impl_epee_benchmark {
    (
        $(
            $t:ty
        ),* $(,)?
    ) => { paste::paste! {
        $(
            #[named]
            fn [<epee_from_bytes_ $t:snake>](c: &mut Criterion) {
                let bytes = to_bytes($t::default()).unwrap();

                c.bench_function(function_name!(), |b| {
                    b.iter_batched(
                        || bytes.clone(),
                        |mut bytes| drop(from_bytes::<$t, _>(black_box(&mut bytes)).unwrap()),
                        BatchSize::SmallInput,
                    );
                });
            }

            #[named]
            fn [<epee_to_bytes_ $t:snake>](c: &mut Criterion) {
                let t = $t::default();

                c.bench_function(function_name!(), |b| {
                    b.iter_batched(
                        || t.clone(),
                        |t| drop(to_bytes(black_box(t)).unwrap()),
                        BatchSize::SmallInput,
                    );
                });
            }
        )*
    }};
}

impl_epee_benchmark! {
    GetBlocksRequest,
    GetBlocksResponse
}
