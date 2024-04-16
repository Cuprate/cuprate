# Database
Cuprate's database implementation.

<!-- Did you know markdown automatically increments number lists, even if they are all 1...? -->
1. [Documentation](#documentation)
1. [File Structure](#file-structure)
    - [`src/`](#src)
    - [`src/ops`](#src-ops)
    - [`src/service/`](#src-service)
    - [`src/backend/`](#src-backend)
1. [Benchmarking](#benchmarking)
1. [Backends](#backends)
    - [`heed`](#heed)
    - [`redb`](#redb)
    - [`redb-memory`](#redb-memory)
    - [`sanakirja`](#sanakirja)
    - [`MDBX`](#mdbx)
1. [Layers](#layers)
    - [Database](#database)
    - [Trait](#trait)
    - [ConcreteEnv](#concreteenv)
    - [Thread-pool](#thread-pool)
    - [Service](#service)
1. [Resizing](#resizing)
1. [Flushing](#flushing)
1. [(De)serialization](#deserialization)

---

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
| `config.rs`         | Database `Env` configuration
| `constants.rs`      | General constants used throughout `cuprate-database`
| `database.rs`       | Abstracted database; `trait DatabaseR{o,w}`
| `env.rs`            | Abstracted database environment; `trait Env`
| `error.rs`          | Database error types
| `free.rs`           | General free functions (related to the database)
| `key.rs`            | Abstracted database keys; `trait Key`
| `resize.rs`         | Database resizing algorithms
| `storable.rs`       | Data (de)serialization; `trait Storable`
| `table.rs`          | Database table abstraction; `trait Table`
| `tables.rs`         | All the table definitions used by `cuprate-database`
| `transaction.rs`    | Database transaction abstraction; `trait TxR{o,w}`
| `types.rs`          | Database table schema types

## `src/ops/`
This folder contains the `cupate_database::ops` module.

TODO: more detailed descriptions.

| File            | Purpose |
|-----------------|---------|
| `alt_block.rs`  | Alternative blocks
| `block.rs`      | Blocks
| `blockchain.rs` | Blockchain-related
| `output.rs`     | Outputs
| `property.rs`   | Properties
| `spent_key.rs`  | Spent keys
| `tx.rs`         | Transactions

## `src/service/`
This folder contains the `cupate_database::service` module.

| File           | Purpose |
|----------------|---------|
| `free.rs`      | General free functions used (related to `cuprate_database::service`)
| `read.rs`      | Read thread-pool definitions and logic
| `request.rs`   | Read/write `Request`s to the database
| `response.rs`  | Read/write `Response`'s from the database
| `tests.rs`     | Thread-pool tests and test helper functions
| `write.rs`     | Write thread-pool definitions and logic

## `src/backend/`
This folder contains the actual database crates used as the backend for `cuprate-database`.

Each backend has its own folder.

| Folder       | Purpose |
|--------------|---------|
| `heed/`      | Backend using using forked [`heed`](https://github.com/Cuprate/heed)
| `sanakirja/` | Backend using [`sanakirja`](https://docs.rs/sanakirja)

All backends follow the same file structure:

| File             | Purpose |
|------------------|---------|
| `database.rs`    | Implementation of `trait DatabaseR{o,w}`
| `env.rs`         | Implementation of `trait Env`
| `error.rs`       | Implementation of backend's errors to `cuprate_database`'s error types
| `storable.rs`    | Compatibility layer between `cuprate_database::Storable` and backend-specific (de)serialization
| `tests.rs`       | Tests for the specific backend
| `transaction.rs` | Implementation of `trait TxR{o,w}`
| `types.rs`       | Type aliases for long backend-specific types

# Benchmarking
There is a standalone binary within `benchmark` that allows various testing and benchmarking on all the features of `cuprate-database`.

See [`benchmark/README.md`](benchmark/) for more info.

# Backends
`cuprate-database`'s `trait`s abstract over various actual databases.

Each database's implementation is located in its respective file in `src/backend/${DATABASE_NAME}.rs`.

## `heed`
The default database used is [`heed`](https://github.com/meilisearch/heed) (LMDB).

`LMDB` should not need to be installed as `heed` has a build script that pulls it in automatically.

`heed`'s filenames inside Cuprate's database folder (`~/.local/share/cuprate/database/`) are:

| Filename   | Purpose |
|------------|---------|
| `data.mdb` | Main data file
| `lock.mdb` | Database lock file

TODO: document max readers limit: https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L1372. Other potential processes (e.g. `xmrblocks`) that are also reading the `data.mdb` file need to be accounted for.

TODO: document DB on remote filesystem: https://github.com/LMDB/lmdb/blob/b8e54b4c31378932b69f1298972de54a565185b1/libraries/liblmdb/lmdb.h#L129.

## `redb`
The 2nd database backend is the 100% Rust [`redb`](https://github.com/cberner/redb).

The upstream versions from [`crates.io`](https://crates.io/crates/redb) are used.

`redb`'s filenames inside Cuprate's database folder (`~/.local/share/cuprate/database/`) are:

| Filename    | Purpose |
|-------------|---------|
| `data.redb` | Main data file

TODO: document DB on remote filesystem (does redb allow this?)

## `redb-memory`
This backend is 100% the same as `redb`, although, it uses `redb::backend::InMemoryBackend` which is a key-value store that completely resides in memory instead of a file.

All other details about this should be the same as the normal `redb` backend.

## `sanakirja`
[`sanakirja`](https://docs.rs/sanakirja) was a candidate as a backend, however there were problems with maximum value sizes.

The default maximum value size is [1012 bytes](https://docs.rs/sanakirja/1.4.1/sanakirja/trait.Storable.html) which was too small for our requirements. Using [`sanakirja::Slice`](https://docs.rs/sanakirja/1.4.1/sanakirja/union.Slice.html) and [sanakirja::UnsizedStorage](https://docs.rs/sanakirja/1.4.1/sanakirja/trait.UnsizedStorable.html) was attempted, but there were bugs found when inserting a value in-between `512..=4096` bytes.

As such, it is not implemented.

## `MDBX`
[`MDBX`](https://erthink.github.io/libmdbx) was a candidate as a backend, however MDBX deprecated the custom key/value comparison functions, this makes it a bit trickier to implement duplicate tables. It is also quite similar to the main backend LMDB (of which it was originally a fork of).

As such, it is not implemented (yet).

# Layers
TODO: update with accurate information when ready, update image.

## Database
## Trait
## ConcreteEnv
## Thread
## Service

# Resizing
TODO: document resize algorithm:
- Exactly when it occurs
- How much bytes are added

All backends follow the same algorithm.

# Flushing
TODO: document disk flushing behavior.
- Config options
- Backend-specific behavior

# (De)serialization
TODO: document `Storable` and how databases (de)serialize types when storing/fetching.
