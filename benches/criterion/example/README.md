## `cuprate-criterion-example`
An example of using Criterion for benchmarking Cuprate crates.

Consider copy+pasting this crate to use as a base when creating new Criterion benchmark crates.

## `src/`
Benchmark crates have a `benches/` ran by `cargo bench`, but they are also crates themselves,
as in, they have a `src` folder that `benches/` can pull code from.

The `src` directories in these benchmarking crates are usually filled with
helper functions, types, etc, that are used repeatedly in the benchmarks.

## `benches/`
These are the actual benchmarks ran by `cargo bench`.
