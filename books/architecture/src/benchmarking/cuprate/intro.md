# cuprate-benchmark
Cuprate has 2 custom crates for macro benchmarking:
- `cuprate-benchmark`; the actual binary crate ran
- `cuprate-benchmark-lib`; the library that other crates hook into

The purpose of `cuprate-benchmark` is very simple:
1. Set-up the benchmark
1. Start timer
1. Run benchmark
1. Output data

`cuprate-benchmark` runs the benchmarks found in [`benches/benchmark/cuprate-*`](https://github.com/Cuprate/cuprate/tree/main/benches/benchmark).
