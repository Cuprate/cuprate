//! TODO

//---------------------------------------------------------------------------------------------------- Use

//---------------------------------------------------------------------------------------------------- trait Benchmark
/// A benchmarking function and its inputs.
pub trait Benchmark {
    /// Input to the main benchmarking function.
    type Input;

    /// Setup function to generate the input.
    const SETUP: fn() -> Self::Input;

    /// The main function to benchmark.
    const MAIN: fn(Self::Input);
}
