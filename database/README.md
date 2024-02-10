# Database
Cuprate's database implementation.

<!-- Did you know markdown automatically increments number lists, even if they are all 1...? -->
1. [Documentation](#documentation)
1. [File Structure](#file-structure)
    - [`src/`](#src)
    - [`src/service/`](#src-service)
    - [`src/backend/`](#src-backend)
1. [Backends](#backends)
    - [`heed`](#heed)
    - [`sanakirja`](#sanakirja)
1. [Layers](#layers)
    - [Database](#database)
    - [Trait](#trait)
    - [ConcreteDatabase](#concretedatabase)
    - [Thread-pool](#thread-pool)
    - [Service](#service)

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
| `TODO`      | This has to be implemented
| `SOMEDAY`   | This should be implemented... someday

# File Structure
A quick reference of the structure of the folders & files in `cuprate-database`.

Note that `lib.rs/mod.rs` files are purely for re-exporting/visibility/lints, and contain no code. Each sub-directory has a corresponding `mod.rs`.

## `src/`
The top-level `src/` files.

| File/Folder      | Purpose |
|------------------|---------|
| `constants.rs`   | General constants used throughout `cuprate-database`
| `database.rs`    | Abstracted database; `trait Database`
| `env.rs`         | Abstracted database environment; `trait Env`
| `error.rs`       | Database error types
| `free.rs`        | General free functions (related to the database)
| `macros.rs`      | General macros used throughout `cuprate-database`
| `pod.rs`         | Data (de)serialization; `trait Pod`
| `table.rs`       | Database table abstraction; `trait Table`
| `tables.rs`      | All the table definitions used by `cuprate-database`
| `transaction.rs` | Database transaction abstraction; `trait RoTx`, `trait RwTx`

## `src/service/`
This folder contains the `cupate_database::service` module.

| File/Folder    | Purpose |
|----------------|---------|
| `free.rs`      | General free functions used (related to `cuprate_database::service`)
| `read.rs`      | Read thread-pool definitions and logic
| `request.rs`   | Read/write `Request`s to the database
| `response.rs`  | Read/write `Response`'s from the database
| `write.rs`     | Write thread-pool definitions and logic

## `src/backend/`
This folder contains the actual database crates used as the backend for `cuprate-database`.

Each backend has its own folder.

| File/Folder  | Purpose |
|--------------|---------|
| `heed/`      | Backend using using forked [`heed`](https://github.com/Cuprate/heed)
| `sanakirja/` | Backend using [`sanakirja`](https://docs.rs/sanakirja)

### `src/backend/heed/`
| File/Folder      | Purpose |
|------------------|---------|
| `database.rs`    | Implementation of `trait Database`
| `env.rs`         | Implementation of `trait Env`
| `serde.rs`       | Data (de)serialization implementations
| `transaction.rs` | Implementation of `trait RoTx/RwTx`

### `src/backend/sanakirja/`
| File/Folder      | Purpose |
|------------------|---------|
| `database.rs`    | Implementation of `trait Database`
| `env.rs`         | Implementation of `trait Env`
| `transaction.rs` | Implementation of `trait RoTx/RwTx`

# Backends
`cuprate-database`'s `trait`s abstract over various actual databases.

Each database's implementation is located in its respective file in `src/backend/${DATABASE_NAME}.rs`.

## `heed`
The default database used is a modified fork of [`heed`](https://github.com/meilisearch/heed), located at [`Cuprate/heed`](https://github.com/Cuprate/heed).

To generate documentation of the fork for local use:
```bash
git clone --recursive https://github.com/Cuprate/heed
cargo doc
```
`LMDB` should not need to be installed as `heed` has a build script that pulls it in automatically.

## `sanakirja`
TODO

# Layers
TODO: update with accurate information when ready, update image.

## Database
## Trait
## ConcreteDatabase
## Thread
## Service
