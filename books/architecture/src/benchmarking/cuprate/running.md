# Running
`cuprate-benchmark` benchmarks are ran with this command:
```bash
cargo run --release --package cuprate-benchmark --features $BENCHMARK_CRATE_FEATURE
```

For example, to run the example benchmark:
```bash
cargo run --release --package cuprate-benchmark --features example
```

Use the `all` feature to run all benchmarks:
```bash
# Run all benchmarks
cargo run --release --package cuprate-benchmark --features all
```
