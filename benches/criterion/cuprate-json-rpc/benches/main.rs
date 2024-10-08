//! Benchmarks for `cuprate-json-rpc`.
#![allow(unused_crate_dependencies)]

mod response;

criterion::criterion_main! {
    response::benches,
}
