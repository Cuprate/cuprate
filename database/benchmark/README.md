# `cuprate-database-benchmark`
This is a standalone binary that allows testing/benchmarking `cuprate-database`.

<!-- Did you know markdown automatically increments number lists, even if they are all 1...? -->
1. [Documentation](#documentation)
1. [File Structure](#file-structure)
    - [`src/`](#src)
    - [`src/ops`](#src-ops)
    - [`src/service/`](#src-service)
    - [`src/backend/`](#src-backend)
1. [Benchmarking](#benchmarking)
1. [Testing](#testing)

# Documentation
In general, documentation for `database/` is split into 3:

| Documentation location    | Purpose |
|---------------------------|---------|
| `database/README.md`      | High level design of `cuprate-database`
| `cuprate-database`        | Practical usage documentation/warnings/notes/etc
| Source file `// comments` | Implementation-specific details (e.g, how many reader threads to spawn?)

This README serves as the overview/design document.

For actual practical usage, `cuprate-database`'s types and general usage are documented via standard Rust tooling.

Run:
```bash
cargo doc --package cuprate-database --open
```
at the root of the repo to open/read the documentation.

If this documentation is too abstract, refer to any of the source files, they are heavily commented. There are many `// Regular comments` that explain more implementation specific details that aren't present here or in the docs. Use the file reference below to find what you're looking for.

The code within `src/` is also littered with some `grep`-able comments containing some keywords:

| Word        | Meaning |
|-------------|---------|
| `INVARIANT` | This code makes an _assumption_ that must be upheld for correctness
| `SAFETY`    | This `unsafe` code is okay, for `x,y,z` reasons
| `FIXME`     | This code works but isn't ideal
| `HACK`      | This code is a brittle workaround
| `PERF`      | This code is weird for performance reasons
| `TODO`      | This must be implemented; There should be 0 of these in production code
| `SOMEDAY`   | This should be implemented... someday

# File Structure
A quick reference of the structure of the folders & files in `cuprate-database`.

Note that `lib.rs/mod.rs` files are purely for re-exporting/visibility/lints, and contain no code. Each sub-directory has a corresponding `mod.rs`.

## `src/`
The top-level `src/` files.

| File                | Purpose |
|---------------------|---------|
