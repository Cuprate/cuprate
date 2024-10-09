## `cuprate-benchmark-lib`
This crate is the glue between
[`cuprate-benchmark`](https://github.com/Cuprate/cuprate/tree/benches/benches/benchmark/bin)
and all the benchmark crates.

It defines the [`cuprate_benchmark_lib::Benchmark`] trait,
which is the behavior of all benchmarks.

See the [`cuprate-benchmark-example`](https://github.com/Cuprate/cuprate/tree/benches/benches/benchmark/example)
crate to see an example implementation of this trait.

After implementing this trait, a few steps must
be done such that the `cuprate-benchmark` binary
can actually run your benchmark crate; see the
[`Benchmarking` section in the Architecture book](https://architecture.cuprate.org/benchmarking/intro.html)
to see how to do this.