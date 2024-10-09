#![expect(dead_code, reason = "code hidden behind feature flags")]

use cfg_if::cfg_if;

use crate::timings::Timings;

/// Print the final the final markdown table of benchmark timings.
pub(crate) fn print_timings(timings: &Timings) {
    println!("\nFinished all benchmarks, printing results:");

    cfg_if! {
        if #[cfg(feature = "json")] {
            print_timings_json(timings);
        } else {
            print_timings_markdown(timings);
        }
    }
}

/// Default timing formatting.
pub(crate) fn print_timings_markdown(timings: &Timings) {
    let mut s = String::new();
    s.push_str("| Benchmark                          | Time (seconds) |\n");
    s.push_str("|------------------------------------|----------------|");

    #[expect(clippy::iter_over_hash_type)]
    for (k, v) in timings {
        s += &format!("\n| {k:<34} | {v:<14} |");
    }

    println!("\n{s}");
}

/// Enabled via `json` feature.
pub(crate) fn print_timings_json(timings: &Timings) {
    let json = serde_json::to_string_pretty(timings).unwrap();
    println!("\n{json}");
}
