//! Benchmarks for `cuprate-json-rpc`.
#![allow(unused_crate_dependencies)]

mod epee;
mod serde;

criterion::criterion_main! {
    epee::benches,
    serde::benches,
}
