#![doc = include_str!("../README.md")]
#![allow(
    unused_crate_dependencies,
    reason = "this crate imports many potentially unused dependencies"
)]

use std::{collections::HashMap, io::Write};

use cfg_if::cfg_if;

use cuprate_benchmark_lib::Benchmark;

fn main() {
    let mut timings = HashMap::new();

    cfg_if! {
        if #[cfg(not(any(feature = "database", feature = "example")))] {
            compile_error!("[cuprate_benchmark]: no feature specified. Use `--features $BENCHMARK_FEATURE` when building.");
        }
    }

    cfg_if! {
        if #[cfg(feature = "database")] {
            run_benchmark::<cuprate_benchmark_database::Benchmark>(&mut timings);
        }
    }

    cfg_if! {
        if #[cfg(feature = "example")] {
            run_benchmark::<cuprate_benchmark_example::Example>(&mut timings);
        }
    }

    print_timings(&timings);
}

fn run_benchmark<B: Benchmark>(timings: &mut HashMap<&'static str, f32>) {
    let name = std::any::type_name::<B>();

    print!("{name:>34} ... ");
    std::io::stdout().flush().unwrap();

    let input = B::SETUP();

    let now = std::time::Instant::now();
    B::MAIN(input);
    let time = now.elapsed().as_secs_f32();

    println!("{time}");
    assert!(
        timings.insert(name, time).is_none(),
        "[cuprate_benchmark]: there were 2 benchmarks with the same name - this collides the final output: {name}",
    );
}

fn print_timings(timings: &HashMap<&'static str, f32>) {
    let mut s = String::new();
    s.push_str("| Benchmark                          | Time (seconds) |\n");
    s.push_str("|------------------------------------|----------------|");
    #[expect(clippy::iter_over_hash_type)]
    for (k, v) in timings {
        s += &format!("\n| {k:<34} | {v:<14} |");
    }

    println!("\n{s}");
}
