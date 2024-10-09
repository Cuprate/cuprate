//! Benchmarks for `cuprate-json-rpc`.
//!
//! TODO: this crate is not finished.
#![allow(unused_crate_dependencies)]

mod response;

criterion::criterion_main! {
    response::serde,
}
