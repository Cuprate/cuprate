#![doc = include_str!("../README.md")]
#![allow(
    unused_crate_dependencies,
    reason = "this crate imports many potentially unused dependencies"
)]

mod print;
mod run;
mod timings;

use cfg_if::cfg_if;

/// What `main()` does:
/// 1. Run all enabled benchmarks
/// 2. Record benchmark timings
/// 3. Print timing data
///
/// To add a new benchmark to be ran here:
/// 1. Copy + paste a `cfg_if` block
/// 2. Change it to your benchmark's feature flag
/// 3. Change it to your benchmark's type
fn main() {
    let mut timings = timings::Timings::new();

    cfg_if! {
        if #[cfg(not(any(feature = "example")))] {
            compile_error!("[cuprate_benchmark]: no feature specified. Use `--features $BENCHMARK_FEATURE` when building.");
        }
    }

    cfg_if! {
        if #[cfg(feature = "example")] {
            run::run_benchmark::<cuprate_benchmark_example::Example>(&mut timings);
        }
    }

    print::print_timings(&timings);
}
