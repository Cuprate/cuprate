# Benches
This directory contains 3 sub-directories:

| Sub-directory | Purpose |
|---------------|---------|
| `micro/`      | Micro-benchmarks for crates (e.g. timings for a single function)
| `macro/`      | Macro-benchmarks for whole crates or sub-systems (using Cuprate's custom benchmarking harness)
| `harness/`    | Cuprate's custom benchmarking harness

## Harness
The harness is just another crate (that happens to be for benchmarking).

Conceptually, it's purpose is very simple:
1. Set-up the benchmark
1. Start timer
1. Run benchmark
1. Output data

This single harness runs the benchmarks found in `macro/`.

The way benchmarks "plug-in" to the harness is simply by implementing `trait Benchmark`.

See `cuprate-harness`' crate documentation for a user-guide:
```bash
cargo doc --open --package cuprate-harness
```

## Macro
Each sub-directory in here is a crate that plugs into the harness.

Benchmarks in `macro/` are for testing sub-systems and/or sections of a sub-system, e.g. the block downloader, the RPC server, the database, etc.

<!-- TODO -->
See `macro/cuprate-database` for an example.
<!-- TODO -->

## Micro
Each sub-directory in here is a crate that uses [Criterion](https://bheisler.github.io/criterion.rs/book) for timing single functions, groups of functions.

They are generally be small in scope.

<!-- TODO -->
See `macro/cuprate-json-rpc` for an example.
<!-- TODO -->