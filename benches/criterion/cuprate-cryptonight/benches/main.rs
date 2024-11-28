//! Benchmarks for `cuprate-cryptonight`.
#![allow(unused_crate_dependencies)]

mod hash;

criterion::criterion_main! {
    hash::benches
}
