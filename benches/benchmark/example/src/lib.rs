#![doc = include_str!("../README.md")]

/// TODO
pub struct Example;

impl cuprate_benchmark_lib::Benchmark for Example {
    type Input = ();
    const SETUP: fn() -> Self::Input = || {};
    const MAIN: fn(Self::Input) = |()| {};
}
