# Creating
Creating a new Criterion-based benchmarking crate for one of Cuprate's crates is relatively simple,
although, it requires knowledge of how to use Criterion first:

1. Read the `Getting Started` section of <https://bheisler.github.io/criterion.rs/book>
2. Copy [`benches/criterion/example`](https://github.com/Cuprate/cuprate/tree/main/benches/criterion/example) as base
3. Get started

## Naming
New benchmark crates using Criterion should:
- Be in [`benches/criterion/`](https://github.com/Cuprate/cuprate/tree/main/benches/criterion/)
- Be in the `cuprate-criterion-$CRATE_NAME` format

For a real example, see:
[`cuprate-criterion-json-rpc`](https://github.com/Cuprate/cuprate/tree/main/benches/criterion/cuprate-json-rpc).

## Workspace
Finally, make sure to add the benchmark crate to the workspace
[`Cargo.toml`](https://github.com/Cuprate/cuprate/blob/main/Cargo.toml) file.

Your benchmark is now ready to be ran.