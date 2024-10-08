# Benchmarking
Cuprate has 2 types of benchmarks:
- Criterion benchmarks
- `cuprate-benchmark` benchmarks

[Criterion](https://bheisler.github.io/criterion.rs/book/user_guide/advanced_configuration.html) is used for micro benchmarks; they time single functions, groups of functions, and generally are small in scope.

`cuprate-benchmark` and `cuprate-benchmark-lib` are custom in-house crates Cuprate uses for macro benchmarks; these test sub-systems, sections of a sub-system, or otherwise larger or more complicated code that isn't suited for micro benchmarks.

## File layout and purpose
All benchmarking related files are in the [`benches/`](https://github.com/Cuprate/cuprate/tree/main/benches) folder.

This directory is organized like such:

| Directory                     | Purpose |
|-------------------------------|---------|
| `benches/criterion/`          | Criterion (micro) benchmarks
| `benches/criterion/cuprate-*` | Criterion benchmarks for the crate with the same name
| `benches/benchmark/`          | Cuprate's custom benchmarking files
| `benches/benchmark/bin`       | The `cuprate-benchmark` crate; the actual binary run that links all benchmarks
| `benches/benchmark/lib`       | The `cuprate-benchmark-lib` crate; the benchmarking framework all benchmarks plug into
| `benches/benchmark/cuprate-*` | `cuprate-benchmark` benchmarks for the crate with the same name
