#![doc = include_str!("../README.md")]

use std::hint::black_box;

use cuprate_benchmark_lib::Benchmark;

/// Marker struct that implements [`Benchmark`]
pub struct Example;

/// The input to our benchmark function.
pub type ExampleBenchmarkInput = u64;

/// The setup function that creates the input.
pub const fn example_benchmark_setup() -> ExampleBenchmarkInput {
    1
}

/// The main benchmarking function.
#[expect(clippy::unit_arg)]
pub fn example_benchmark_main(input: ExampleBenchmarkInput) {
    // In this case, we're simply benchmarking the
    // performance of simple arithmetic on the input data.

    fn math(input: ExampleBenchmarkInput, number: u64) {
        let x = input;
        let x = black_box(x * number);
        let x = black_box(x / number);
        let x = black_box(x + number);
        let _ = black_box(x - number);
    }

    for number in 1..100_000_000 {
        black_box(math(input, number));
    }
}

// This implementation will be run by `cuprate-benchmark`.
impl Benchmark for Example {
    type Input = ExampleBenchmarkInput;
    const SETUP: fn() -> Self::Input = example_benchmark_setup;
    const MAIN: fn(Self::Input) = example_benchmark_main;
}
