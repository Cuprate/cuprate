# Benchmarking
Cuprate's benchmarks live in the [`Cuprate/benches`](https://github.com/Cuprate/benches) repository; there are 2 types of benchmarks:
- [Criterion](https://bheisler.github.io/criterion.rs/book/user_guide/advanced_configuration.html) benchmarks
- `cuprate-benchmark` benchmarks

Criterion is used for micro benchmarks; they time single functions, groups of functions, and generally are small in scope.

`cuprate-benchmark` and [`cuprate-benchmark-lib`](https://doc.cuprate.org/cuprate_benchmark_lib) are custom in-house crates Cuprate uses for macro benchmarks; these test sub-systems, sections of a sub-system, or otherwise larger or more complicated code that isn't well-suited for micro benchmarks.

## File layout and purpose
`Cuprate/benches` is organized like such:

| Directory                     | Purpose |
|-------------------------------|---------|
| [`criterion/`](https://github.com/Cuprate/benches/tree/main/criterion) | Criterion (micro) benchmarks
| `criterion/cuprate-*` | Criterion benchmarks for the crate with the same name
| [`benchmark/`](https://github.com/Cuprate/benches/tree/main/benchmark) | Cuprate's custom benchmarking files
| [`benchmark/bin`](https://github.com/Cuprate/benches/tree/main/benchmark/bin) | The `cuprate-benchmark` crate; the actual binary run that links all benchmarks
| [`benchmark/lib`](https://github.com/Cuprate/benches/tree/main/benchmark/lib) | The `cuprate-benchmark-lib` crate; the benchmarking framework all benchmarks plug into
| `benchmark/cuprate-*` | `cuprate-benchmark` benchmarks for the crate with the same name