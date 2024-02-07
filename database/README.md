# Database
This is the main design document and implementation of the database used by Cuprate.

The code within `/database/src` is also littered with comments. Some `grep`-able keywords:

| Word        | Meaning |
|-------------|---------|
| `INVARIANT` | This code makes an _assumption_ that must be upheld for correctness
| `SAFETY`    | This `unsafe` code is okay, for `x,y,z` reasons
| `FIXME`     | This code works but isn't ideal
| `HACK`      | This code is a brittle workaround
| `PERF`      | This code is weird for performance reasons
| `TODO`      | This has to be implemented
| `SOMEDAY`   | This should be implemented... someday

---

1. [Documentation](#documentation)
2. [File Structure](#file-structure)
    - [`src/`](#src)
    - [`src/service/`](#src-service)
    - [`src/backend/`](#src-backend)
3. [Layers](#layers)
4. [Backends](#backends)
    - [`heed`](#heed)
    - [`sanakirja`](#sanakirja)

---

# Documentation
This README serves as the overview/design document.

For actual usage, `cuprate-database`'s types and general usage are documented via standard Rust tooling.

Run:
```bash
cargo doc --package cuprate-database --open
```
at the root of the repo to open/read the documentation.

# File Structure
A quick reference of the structure of the folders & files in `cuprate-database`.

## `src/`
The top-level `src/` files.

| File/Folder      | Purpose |
|------------------|---------|
| `constants.rs`   | TODO
| `database.rs`    | TODO
| `error.rs`       | TODO
| `free.rs`        | TODO
| `lib.rs`         | TODO
| `macros.rs`      | TODO
| `pod.rs`         | TODO
| `table.rs`       | TODO
| `transaction.rs` | TODO

## `src/service/`
This folder contains the `cupate_database::service` module.

This module provides the:
- public `tower::Service` abstractions for the database
- thread-pool system for database readers/writers

| File/Folder    | Purpose |
|----------------|---------|
| `init.rs`      | TODO
| `read.rs`      | TODO
| `request.rs`   | TODO
| `response.rs`  | TODO
| `write.rs`     | TODO

## `src/backend/`
This folder contains the actual database crates used as the backend for the `trait Database` that `cuprate-database` exposes.

Each backend has its respective folder.

| File/Folder  | Purpose |
|--------------|---------|
| `heed/`      | Backend `trait Database` using using forked [`heed`](https://github.com/Cuprate/heed)
| `sanakirja/` | Backend `trait Database` impl using `sanakirja`

### `src/backend/heed/`
| File/Folder  | Purpose |
|--------------|---------|
| TODO         | TODO

### `src/backend/sanakirja/`
| File/Folder  | Purpose |
|--------------|---------|
| TODO         | TODO

# Layers
TODO: update to more accurate information, update image.

The database is abstracted into 5 layers internally.

Starting from the lowest layer:
1. **The database** - this is the actual database, or a thin Rust shim on-top of the database that calls database operations directly, e.g `get()`, `commit()`, `delete()`, etc
2. **The trait** - this is the `trait` that abstracts over _all_ databases and allows keeping the function signatures and behavior the same but allows for swapping out databases; each database will have this implemented located in `src/backend/`, with each database (`LMDB`, `MDBX`, `sled`, etc) having its own file defining the mappings. This `trait` is meant to cover all features across databases, and will have provided methods that may not necessarily be the most efficient - if a database can implement a method in a better way, it is re-implemented and will shadow the provided version
3. **The abstract database** - this is a concrete object and handle to _some_ database that implements the generic `trait`
4. **The thread** - this is the dedicated thread that is the logical _owner_ of the abstract database. It acts as a kernel between the async public interface and the internal database calls. This thread is responsible for converting the high-level "requests" from other Cuprate crates (`add_block()`, `get_block()`, etc) via channel messages and is responsible for doing the underlying work with the database to eventually return a "response" to the calling code, again, via a channel
5. **The `tower::Service`** - this is the public API that other Cuprate crates will interface with; the abstract database will have `tower::Service<R>` implemented for each `R`, where `R` is a specific high-level request other Cuprate crates need, e.g. `add_block()`,  `get_block()`, etc - this request is executed by "the thread" which eventually returns the result of the function

<div align="center">
    <img src="https://github.com/hinto-janai/cuprate/assets/101352116/b7d7cbe3-ce55-44ea-92cc-ecde10cf519a" width="50%"/>
</div>

# Backends
`cuprate-database`'s `trait Database` abstracts over various actual databases.

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