# `cuprate-database-benchmark`
This is a benchmarking suite that allows testing/benchmarking `cuprate-database` with [`criterion`](https://bheisler.github.io/criterion.rs/book/criterion_rs.html).

For more information on `cargo bench` and `criterion`:
- https://doc.rust-lang.org/cargo/commands/cargo-bench.html
- https://bheisler.github.io/criterion.rs/book/criterion_rs.html

<!-- Did you know markdown automatically increments number lists, even if they are all 1...? -->
1. [Usage](#Usage)
1. [File Structure](#file-structure)
    - [`src/`](#src)
    - [`benches/`](#benches)

# Usage
Ensure the system is as quiet as possible (no background tasks) before starting and during the benchmarks.

To start all benchmarks, run:
```bash
cargo bench --package cuprate-database-benchmarks
```

# File Structure
A quick reference of the structure of the folders & files in `cuprate-database`.

Note that `lib.rs/mod.rs` files are purely for re-exporting/visibility/lints, and contain no code. Each sub-directory has a corresponding `mod.rs`.

## `src/`
The top-level `src/` files.

The actual `cuprate-database-benchmark` library crate is just used as a helper for the benchmarks within `benches/`.

| File                | Purpose |
|---------------------|---------|
| `helper.rs`         | Helper functions

## `benches/`
The actual benchmarks.

Each file represents some logical benchmark grouping.

| File                  | Purpose |
|-----------------------|---------|
| `db.rs`               | `trait Database{Ro,Rw,Iter}` benchmarks
| `db_multi_thread.rs`  | Same as `db.rs` but multi-threaded
| `env.rs`              | `trait {Env, EnvInner, TxR{o,w}, Tables[Mut]}` benchmarks
| `env_multi_thread.rs` | Same as `env.rs` but multi-threaded
| `storable.rs`         | `trait Storable` benchmarks