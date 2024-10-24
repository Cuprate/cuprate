# Benchmarking
Cuprate has 2 types of benchmarks:
- [Criterion](https://bheisler.github.io/criterion.rs/book/user_guide/advanced_configuration.html) benchmarks
- `cuprate-benchmark` benchmarks

Criterion is used for micro benchmarks; they time single functions, groups of functions, and generally are small in scope.

`cuprate-benchmark` and [`cuprate-benchmark-lib`](https://doc.cuprate.org/cuprate_benchmark_lib) are custom in-house crates Cuprate uses for macro benchmarks; these test sub-systems, sections of a sub-system, or otherwise larger or more complicated code that isn't well-suited for micro benchmarks.

## File layout and purpose
All benchmarking related files are in the [`benches/`](https://github.com/Cuprate/cuprate/tree/main/benches) folder.

This directory is organized like such:

| Directory                     | Purpose |
|-------------------------------|---------|
| [`benches/criterion/`](https://github.com/Cuprate/cuprate/tree/main/benches/criterion) | Criterion (micro) benchmarks
| `benches/criterion/cuprate-*` | Criterion benchmarks for the crate with the same name
| [`benches/benchmark/`](https://github.com/Cuprate/cuprate/tree/main/benches/benchmark) | Cuprate's custom benchmarking files
| [`benches/benchmark/bin`](https://github.com/Cuprate/cuprate/tree/main/benches/benchmark/bin) | The `cuprate-benchmark` crate; the actual binary run that links all benchmarks
| [`benches/benchmark/lib`](https://github.com/Cuprate/cuprate/tree/main/benches/benchmark/lib) | The `cuprate-benchmark-lib` crate; the benchmarking framework all benchmarks plug into
| `benches/benchmark/cuprate-*` | `cuprate-benchmark` benchmarks for the crate with the same name
