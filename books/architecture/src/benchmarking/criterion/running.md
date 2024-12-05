# Running
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