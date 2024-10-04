use std::io::Write;

use cuprate_benchmark_lib::Benchmark;

use crate::timings::Timings;

/// Run a [`Benchmark`] and record its timing.
pub(crate) fn run_benchmark<B: Benchmark>(timings: &mut Timings) {
    // Print the benchmark name.
    let name = std::any::type_name::<B>();
    print!("{name:>34} ... ");
    std::io::stdout().flush().unwrap();

    // Setup the benchmark input.
    let input = B::SETUP();

    // Sleep before running the benchmark.
    std::thread::sleep(B::PRE_SLEEP_DURATION);

    // Run/time the benchmark.
    let now = std::time::Instant::now();
    B::MAIN(input);
    let time = now.elapsed().as_secs_f32();

    // Print the benchmark timings.
    println!("{time}");
    assert!(
        timings.insert(name, time).is_none(),
        "[cuprate_benchmark]: there were 2 benchmarks with the same name - this collides the final output: {name}",
    );

    // Sleep for a cooldown period after the benchmark run.
    std::thread::sleep(B::POST_SLEEP_DURATION);
}
