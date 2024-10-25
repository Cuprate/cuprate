use tracing::{info, instrument, trace};

use cuprate_benchmark_lib::Benchmark;

use crate::timings::Timings;

/// Run a [`Benchmark`] and record its timing.
#[instrument(skip_all)]
pub(crate) fn run_benchmark<B: Benchmark>(timings: &mut Timings) {
    // Get the benchmark name.
    let name = B::name();
    trace!("Running benchmark: {name}");

    // Setup the benchmark input.
    let input = B::SETUP();

    // Sleep before running the benchmark.
    trace!("Pre-benchmark, sleeping for: {:?}", B::POST_SLEEP_DURATION);
    std::thread::sleep(B::PRE_SLEEP_DURATION);

    // Run/time the benchmark.
    let now = std::time::Instant::now();
    B::MAIN(input);
    let time = now.elapsed().as_secs_f32();

    // Print the benchmark timings.
    info!("{name:>34} ... {time}");
    assert!(
        timings.insert(name, time).is_none(),
        "There were 2 benchmarks with the same name - this collides the final output: {name}",
    );

    // Sleep for a cooldown period after the benchmark run.
    trace!("Post-benchmark, sleeping for: {:?}", B::POST_SLEEP_DURATION);
    std::thread::sleep(B::POST_SLEEP_DURATION);
}
