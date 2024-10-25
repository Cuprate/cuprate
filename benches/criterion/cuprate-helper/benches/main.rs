//! `cuprate_helper` benchmarks.
#![allow(unused_crate_dependencies)]

mod cast;
mod map;
mod num;
mod tx;

criterion::criterion_main! {
    cast::benches,
    map::benches,
    num::benches,
    tx::benches,
}
