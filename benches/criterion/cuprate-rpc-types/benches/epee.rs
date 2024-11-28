//! This module contains benchmarks for any
//!
//! - non-trivial
//! - manual
//! - common
//!
//! type with a `epee` implementation.
//!
//! Types with the standard `epee` derive implementation are not included.

#![allow(unused_attributes, unused_crate_dependencies)]

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use function_name::named;

use cuprate_epee_encoding::{from_bytes, to_bytes};
use cuprate_rpc_types::bin::GetBlocksRequest;

/// Create [`to_bytes`] and [`from_bytes`] benchmarks for `epee` types.
macro_rules! generate_epee_benchmarks {
    (
        $(
            $t:ty
        ),* $(,)?
    ) => { paste::paste! {
        // Generate the benchmarking functions.
        $(
            #[named]
            fn [<epee_from_bytes_ $t:snake>](c: &mut Criterion) {
                let bytes = to_bytes($t::default()).unwrap();

                // `iter_batched()` is used so the `Default::default()`
                // is not part of the timings.
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

        // Enable all the benchmark functions created in this macro.
        criterion_group! {
            name = benches;
            config = Criterion::default();
            targets =
            $(
                [<epee_from_bytes_ $t:snake>],
                [<epee_to_bytes_ $t:snake>],
            )*
        }
        criterion_main!(benches);
    }};
}

generate_epee_benchmarks! {
    GetBlocksRequest,
    // GetBlocksResponse // TODO: fix epee impl
}
