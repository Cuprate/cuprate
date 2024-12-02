# cuprate-benchmark
Cuprate has 2 custom crates for general benchmarking:
- `cuprate-benchmark`; the actual binary crate ran
- `cuprate-benchmark-lib`; the library that other crates hook into

The abstract purpose of `cuprate-benchmark` is very simple:
1. Set-up the benchmark
1. Start timer
1. Run benchmark
1. Output data

`cuprate-benchmark` runs the benchmarks found in [`benches/benchmark/cuprate-*`](https://github.com/Cuprate/cuprate/tree/main/benches/benchmark).

`cuprate-benchmark-lib` defines the `Benchmark` trait that all
benchmark crates implement to "plug-in" to the benchmarking harness.

## Diagram
A diagram displaying the relation between `cuprate-benchmark` and related crates.

```
                    ┌─────────────────────┐
                    │ cuprate_benchmark   │
                    │ (actual binary ran) │
                    └──────────┬──────────┘
            ┌──────────────────┴───────────────────┐
            │ cuprate_benchmark_lib                │
            │ ┌───────────────────────────────────┐│
            │ │ trait Benchmark                   ││
            │ └───────────────────────────────────┘│
            └──────────────────┬───────────────────┘
┌───────────────────────────┐  │   ┌───────────────────────────┐
│ cuprate_benchmark_example ├──┼───┤ cuprate_benchmark_*       │
└───────────────────────────┘  │   └───────────────────────────┘
┌───────────────────────────┐  │   ┌───────────────────────────┐
│ cuprate_benchmark_*       ├──┴───┤ cuprate_benchmark_*       │
└───────────────────────────┘      └───────────────────────────┘
```