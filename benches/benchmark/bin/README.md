## `cuprate-benchmark`
This crate links all benchmarks together into a single binary that can be run as: `cuprate-benchmark`.

`cuprate-benchmark` will run all enabled benchmarks sequentially and print data at the end.

## Benchmarks
Benchmarks are opt-in and enabled via features.

| Feature  | Enables which benchmark crate? |
|----------|--------------------------------|
| example  | cuprate-benchmark-example      |
| database | cuprate-benchmark-database     |

## Features
These are features that aren't for enabling benchmarks, but rather for other things.

| Features | Does what |
|----------|-----------|
| json     | Prints JSON timings instead of a markdown table