//! Benchmarking trait.

use std::time::Duration;

/// A benchmarking function and its inputs.
pub trait Benchmark {
    /// The benchmark's name.
    ///
    /// This is automatically implemented
    /// as the name of the [`Self`] type.
    fn name() -> &'static str {
        std::any::type_name::<Self>()
    }

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

    /// `cuprate-benchmark` will sleep for this [`Duration`] after
    /// creating the [`Self::Input`], but before starting [`Self::MAIN`].
    ///
    /// 1 second by default.
    const PRE_SLEEP_DURATION: Duration = Duration::from_secs(1);

    /// `cuprate-benchmark` will sleep for this [`Duration`] after [`Self::MAIN`].
    ///
    /// 1 second by default.
    const POST_SLEEP_DURATION: Duration = Duration::from_secs(1);
}
