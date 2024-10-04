//! TODO

/// A benchmarking function and its inputs.
pub trait Benchmark {
    /// Input to the main benchmarking function.
    ///
    /// This is passed to [`Self::MAIN`].
    type Input;

    /// Setup function to generate the input.
    ///
    /// This function is not timed.
    const SETUP: fn() -> Self::Input;

    /// The main function to benchmark.
    ///
    /// The start of the timer begins right before
    /// this function is called and ends after the
    /// function returns.
    const MAIN: fn(Self::Input);
}
