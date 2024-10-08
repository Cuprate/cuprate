# Creating
New benchmarks are plugged into `cuprate-benchmark` by:
1. Implementing `cuprate_benchmark_lib::Benchmark`
1. Registering the benchmark in the `cuprate_benchmark` binary

See [`benches/benchmark/example`](https://github.com/Cuprate/cuprate/tree/main/benches/benchmark/example)
for an example. For a real example, see:
[`cuprate-benchmark-database`](https://github.com/Cuprate/cuprate/tree/main/benches/benchmark/cuprate-database).

## Creating the benchmark crate
Before plugging into `cuprate-benchmark`, your actual benchmark crate must be created:

1. Create a new crate inside `benches/benchmark` (consider copying `benches/benchmark/example` as a base)
1. Pull in `cuprate_benchmark_lib` as a dependency
1. Create a benchmark
1. Implement `cuprate_benchmark_lib::Benchmark`

## `cuprate_benchmark_lib::Benchmark`
This is the trait that standardizes all benchmarks ran under `cuprate-benchmark`.

It must be implemented by your benchmarking crate.

See `cuprate-benchmark-lib` crate documentation for a user-guide: <https://doc.cuprate.org/cuprate_benchmark_lib>.

## Adding a feature to `cuprate-benchmark`
After your benchmark's behavior is defined, it must be registered
in the binary that is actually ran: `cuprate-benchmark`.

If your benchmark is new, add a new crate feature to [`cuprate-benchmark`'s Cargo.toml file](https://github.com/Cuprate/cuprate/tree/main/benches/benchmark/bin/Cargo.toml) with an optional dependency to your benchmarking crate.

## Adding to `cuprate-benchmark`'s `main()`
After adding your crate's feature, add a conditional line that run the benchmark
if the feature is enabled to the `main()` function:

For example, if your crate's name is `egg`:
```rust
cfg_if! {
	if #[cfg(feature = "egg")] {
		run::run_benchmark::<cuprate_benchmark_egg::Benchmark>(&mut timings);
	}
}
```