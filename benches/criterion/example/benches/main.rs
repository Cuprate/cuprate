//! Benchmarks examples.
#![allow(unused_crate_dependencies)]

// All modules within `benches/` are `mod`ed here.
mod example;

// And all the Criterion benchmarks are registered like so:
criterion::criterion_main! {
    example::benches,
}
