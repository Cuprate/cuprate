# Creating
New benchmarks are plugged into `cuprate-benchmark` by:
1. Implementing `cuprate_benchmark_lib::Benchmark`
1. Registering the benchmark in the `cuprate_benchmark` binary

See [`benchmark/example`](https://github.com/Cuprate/benches/tree/main/benchmark/example)
for an example.

## Creating the benchmark crate
Before plugging into `cuprate-benchmark`, your actual benchmark crate must be created:

1. Create a new crate inside `benchmark` (consider copying `benchmark/example` as a base)
1. Pull in `cuprate_benchmark_lib` as a dependency
1. Create a benchmark
1. Implement `cuprate_benchmark_lib::Benchmark`

New benchmark crates using `cuprate-database` should:
- Be in [`benchmark/`](https://github.com/Cuprate/benches/tree/main/benchmark/)
- Be in the `cuprate-benchmark-$CRATE_NAME` format

For a real example, see:
[`cuprate-benchmark-database`](https://github.com/Cuprate/benches/tree/main/benchmark/cuprate-database).

## `cuprate_benchmark_lib::Benchmark`
This is the trait that standardizes all benchmarks ran under `cuprate-benchmark`.

It must be implemented by your benchmarking crate.

See `cuprate-benchmark-lib` crate documentation for a user-guide: <https://doc.cuprate.org/cuprate_benchmark_lib>.

## Adding a feature to `cuprate-benchmark`
After your benchmark's behavior is defined, it must be registered
in the binary that is actually ran: `cuprate-benchmark`.

If your benchmark is new, add a new crate feature to [`cuprate-benchmark`'s Cargo.toml file](https://github.com/Cuprate/benches/tree/main/benchmark/bin/Cargo.toml) with an optional dependency to your benchmarking crate.

Please remember to edit the feature table in the
[`README.md`](https://github.com/Cuprate/benches/tree/main/benchmark/bin/README.md) as well!

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

## Workspace
Finally, make sure to add the benchmark crate to the workspace
[`Cargo.toml`](https://github.com/Cuprate/benches/blob/main/Cargo.toml) file.

Your benchmark is now ready to be ran.