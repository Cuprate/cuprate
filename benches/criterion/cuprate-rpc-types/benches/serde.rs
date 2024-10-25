//! This module contains benchmarks for any
//! non-trivial/manual `serde` implementation.
//!
//! Types with the standard `serde` derive implementation are not included.

#![allow(unused_attributes, unused_crate_dependencies)]

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use function_name::named;
use serde_json::{from_str, to_string};

use cuprate_rpc_types::misc::TxEntry;

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets =
    serde_from_str_tx_entry,
    serde_to_string_tx_entry,
}
criterion_main!(benches);

/// TODO
macro_rules! impl_serde_benchmark {
    (
        $(
            // The type to test =>
            // A `const: &str` from `cuprate_test_utils` for that type or just an inline expression
            $t:ty => $t_example:expr
        ),* $(,)?
    ) => { paste::paste! {
        $(
            #[named]
            fn [<serde_from_str_ $t:snake>](c: &mut Criterion) {
                c.bench_function(function_name!(), |b| {
                    b.iter(|| {
                        drop(from_str::<$t>(
                            black_box($t_example)
                        ).unwrap());
                    });
                });
            }

            #[named]
            fn [<serde_to_string_ $t:snake>](c: &mut Criterion) {
                let t = $t::default();

                c.bench_function(function_name!(), |b| {
                    b.iter_batched(
                        || t.clone(),
                        |t| drop(to_string(black_box(&t)).unwrap()),
                        BatchSize::SmallInput,
                    );
                });
            }
        )*
    }};
}

impl_serde_benchmark! {
    TxEntry => r#"{"as_hex":"","as_json":"","double_spend_seen":false,"prunable_as_hex":"","prunable_hash":"","pruned_as_hex":"","received_timestamp":0,"relayed":false,"tx_hash":"","in_pool":false}"#,
    // Distribution => "TODO: enable after type is finalized"
}
