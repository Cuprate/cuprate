//! Benchmarks for `cuprate-json-rpc`.
#![allow(unused_crate_dependencies)]

mod serde;
mod epee;

criterion::criterion_main! {
    epee::benches,
    serde::benches,
}
