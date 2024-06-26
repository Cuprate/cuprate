# Benches
This directory contains Cuprate's benchmarks and benchmarking utilities.

- [1. File layout and purpose](#1-file-layout-and-purpose)
- [2. Harness](#2-harness)
	- [2.1 Creating a harness benchmark](#21-creating-a-harness-benchmark)
	- [2.2 Running a harness benchmark](#22-running-a-harness-benchmark)
- [3. Criterion](#3-criterion)
	- [2.1 Creating a Criterion benchmark](#21-creating-a-criterion-benchmark)
	- [2.2 Running a Criterion benchmark](#22-running-a-criterion-benchmark)

## 1. File layout and purpose
This directory is sorted into 4 important categories:

| Sub-directory | Purpose |
|---------------|---------|
| `harness/src` | Cuprate's custom benchmarking harness **binary**
| `harness/lib` | Cuprate's custom benchmarking harness **library**
| `harness/*`   | Macro-benchmarks for whole crates or sub-systems (using Cuprate's custom benchmarking harness)
| `criterion/*` | Micro-benchmarks for crates (e.g. timings for a single function)

## 2. Harness
The harness is:
- `cuprate-harness`; the actual binary crate ran
- `cuprate-harness-lib`; the library that other crates hook into

The purpose of the harness is very simple:
1. Set-up the benchmark
1. Start timer
1. Run benchmark
1. Output data

The harness runs the benchmarks found in `harness/`.

The way benchmarks "plug-in" to the harness is simply by implementing `cuprate_harness_lib::Benchmark`.

See `cuprate-harness-lib` crate documentation for a user-guide:
```bash
cargo doc --open --package cuprate-harness-lib
```

### 2.1 Creating a harness benchmark
1. Create a new crate inside `benches/harness` (consider copying `benches/harness/test` as a base)
2. Pull in `cuprate_harness_lib` as a dependency
3. Implement `cuprate_harness_lib::Benchmark`
4. Add a feature inside `cuprate_harness` for your benchmark

### 2.2 Running a harness benchmark
After your benchmark is implemented, run this command:
```bash
cargo run --release --package cuprate-harness --features $YOUR_BENCHMARK_CRATE_FEATURE
```
For example, to run the test benchmark:
```bash
cargo run --release --package cuprate-harness --features test
```

## 3. Criterion
Each sub-directory in here is a crate that uses [Criterion](https://bheisler.github.io/criterion.rs/book) for timing single functions and/or groups of functions.

They are generally be small in scope.

See [`criterion/cuprate-json-rpc`](https://github.com/Cuprate/cuprate/tree/main/benches/criterion/cuprate-json-rpc) for an example.

### 3.1 Creating a Criterion benchmark
1. Copy [`criterion/test`](https://github.com/Cuprate/cuprate/tree/main/benches/criterion) as base
2. Read the `Getting Started` section of <https://bheisler.github.io/criterion.rs/book>
3. Get started

### 3.1 Running a Criterion benchmark
To run all Criterion benchmarks, run this from the repository root:
```bash
cargo bench
```

To run specific package(s), use:
```bash
cargo bench --package $CRITERION_BENCHMARK_CRATE_NAME
```
For example:
```bash
cargo bench --package cuprate-criterion-json-rpc
```