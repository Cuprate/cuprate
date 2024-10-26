//! This module contains benchmarks for any
//!
//! - non-trivial
//! - manual
//! - common
//!
//! type with a `serde` implementation.
//!
//! Types with the standard `serde` derive implementation are not included.

#![allow(unused_attributes, unused_crate_dependencies)]

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use function_name::named;
use serde_json::{from_str, to_string};

use cuprate_rpc_types::{
    json::{
        CalcPowRequest, GetBlockHeadersRangeResponse, GetBlockResponse, GetBlockTemplateResponse,
        GetConnectionsResponse, GetInfoResponse, GetLastBlockHeaderResponse, SyncInfoResponse,
    },
    misc::TxEntry,
};
use cuprate_test_utils::rpc::data::json::{
    CALC_POW_REQUEST, GET_BLOCK_HEADERS_RANGE_RESPONSE, GET_BLOCK_RESPONSE,
    GET_BLOCK_TEMPLATE_RESPONSE, GET_CONNECTIONS_RESPONSE, GET_INFO_RESPONSE,
    GET_LAST_BLOCK_HEADER_RESPONSE, SYNC_INFO_RESPONSE,
};

/// Generate [`from_str`] and [`to_string`] benchmarks for `serde` types.
macro_rules! generate_serde_benchmarks {
    (
        $(
            // The type to test =>
            // A `const: &str` from `cuprate_test_utils` for that type or just an inline expression
            $t:ty => $t_example:expr
        ),* $(,)?
    ) => { paste::paste! {
        // Generate the benchmarking functions.
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

        // Enable all the benchmark functions created in this macro.
        criterion_group! {
            name = benches;
            config = Criterion::default();
            targets =
            $(
                [<serde_from_str_ $t:snake>],
                [<serde_to_string_ $t:snake>],
            )*
        }
        criterion_main!(benches);
    }};
}

generate_serde_benchmarks! {
    // Custom serde types.
    TxEntry => r#"{"as_hex":"","as_json":"","double_spend_seen":false,"prunable_as_hex":"","prunable_hash":"","pruned_as_hex":"","received_timestamp":0,"relayed":false,"tx_hash":"","in_pool":false}"#,
    // Distribution => "TODO: enable after type is finalized"

    // Common types or heavy types (heap types, many fields, etc).
    GetLastBlockHeaderResponse => GET_LAST_BLOCK_HEADER_RESPONSE,
    CalcPowRequest => CALC_POW_REQUEST,
    SyncInfoResponse => SYNC_INFO_RESPONSE,
    GetInfoResponse => GET_INFO_RESPONSE,
    GetBlockResponse => GET_BLOCK_RESPONSE,
    GetConnectionsResponse => GET_CONNECTIONS_RESPONSE,
    GetBlockTemplateResponse => GET_BLOCK_TEMPLATE_RESPONSE,
    GetBlockHeadersRangeResponse => GET_BLOCK_HEADERS_RANGE_RESPONSE
}
