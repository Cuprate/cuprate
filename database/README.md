# Database
Cuprate's database implementation.

- [1. Documentation](#1-documentation)
- [2. File Structure](#2-file-structure)
    - [2.1 `src/`](#21-src)
    - [2.2 `src/backend/`](#22-srcbackend)
    - [2.3 `src/config`](#23-srcconfig)
    - [2.4 `src/ops`](#24-srcops)
    - [2.5 `src/service/`](#25-srcservice)
- [3. Backends](#3-backends)
    - [3.1 `heed`](#31-heed)
    - [3.2 `redb`](#32-redb)
    - [3.3 `redb-memory`](#33-redb-memory)
    - [3.4 `sanakirja`](#34-sanakirja)
    - [3.5 `MDBX`](#35-mdbx)
- [4. Layers](#4-layers)
    - [4.1 Backend](#41-backend)
    - [4.2 Trait](#42-trait)
    - [4.3 ConcreteEnv](#43-concreteenv)
    - [4.4 `ops`](#44-ops)
    - [4.5 `service`](#45-service)
- [5. Syncing](#5-Syncing)
- [6. Thread model](#6-thread-model)
- [7. Resizing](#7-resizing)
- [8. (De)serialization](#8-deserialization)

---

### 1. Documentation
Documentation for `database/` is split into 3 locations:

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

## 2. File Structure
A quick reference of the structure of the folders & files in `cuprate-database`.

Note that `lib.rs/mod.rs` files are purely for re-exporting/visibility/lints, and contain no code. Each sub-directory has a corresponding `mod.rs`.

### 2.1 `src/`
The top-level `src/` files.

| File                   | Purpose |
|------------------------|---------|
| `constants.rs`         | General constants used throughout `cuprate-database`
| `database.rs`          | Abstracted database; `trait DatabaseR{o,w}`
| `env.rs`               | Abstracted database environment; `trait Env`
| `error.rs`             | Database error types
| `free.rs`              | General free functions (related to the database)
| `key.rs`               | Abstracted database keys; `trait Key`
| `resize.rs`            | Database resizing algorithms
| `storable.rs`          | Data (de)serialization; `trait Storable`
| `table.rs`             | Database table abstraction; `trait Table`
| `tables.rs`            | All the table definitions used by `cuprate-database`
| `tests.rs`             | Utilities for `cuprate_database` testing
| `transaction.rs`       | Database transaction abstraction; `trait TxR{o,w}`
| `types.rs`             | Database-specific types
| `unsafe_unsendable.rs` | Marker type for `Send` objects not proveable to the compiler

### 2.2 `src/backend/`
This folder contains the implementation for actual databases used as the backend for `cuprate-database`.

Each backend has its own folder.

| Folder/File | Purpose |
|-------------|---------|
| `heed/`     | Backend using using [`heed`](https://github.com/meilisearch/heed) (LMDB)
| `redb/`     | Backend using [`redb`](https://github.com/cberner/redb)
| `tests.rs`  | Backend-agnostic tests

All backends follow the same file structure:

| File             | Purpose |
|------------------|---------|
| `database.rs`    | Implementation of `trait DatabaseR{o,w}`
| `env.rs`         | Implementation of `trait Env`
| `error.rs`       | Implementation of backend's errors to `cuprate_database`'s error types
| `storable.rs`    | Compatibility layer between `cuprate_database::Storable` and backend-specific (de)serialization
| `transaction.rs` | Implementation of `trait TxR{o,w}`
| `types.rs`       | Type aliases for long backend-specific types

### 2.3 `src/config/`
| File                | Purpose |
|---------------------|---------|
| `config.rs`         | Main database `Config` struct
| `reader_threads.rs` | Reader thread configuration for `service` thread-pool
| `sync_mode.rs`      | Disk sync configuration for backends

### 2.4 `src/ops/`
This folder contains the `cupate_database::ops` module.

These are higher-level functions abstracted over the database, that are Monero-related.

| File            | Purpose |
|-----------------|---------|
| `block.rs`      | Block related (main functions)
| `blockchain.rs` | Blockchain related (height, cumulative values, etc)
| `key_image.rs`  | Key image related
| `macros.rs`     | Macros specific to `ops/`
| `output.rs`     | Output related
| `property.rs`   | Database properties (pruned, version, etc)
| `tx.rs`         | Transaction related

### 2.5 `src/service/`
This folder contains the `cupate_database::service` module.

The `async`hronous request/response API other Cuprate crates use instead of managing the database directly themselves.

| File           | Purpose |
|----------------|---------|
| `free.rs`      | General free functions used (related to `cuprate_database::service`)
| `read.rs`      | Read thread-pool definitions and logic
| `tests.rs`     | Thread-pool tests and test helper functions
| `types.rs`     | `cuprate_database::service`-related type aliases
| `write.rs`     | Writer thread definitions and logic

## 3. Backends
`cuprate-database`'s `trait`s allow abstracting over the actual database, such that any backend in particular could be used.

Each database's implementation for those `trait`'s are located in its respective folder in `src/backend/${DATABASE_NAME}/`.

### 3.1 `heed`
The default database used is [`heed`](https://github.com/meilisearch/heed) (LMDB).

The upstream versions from [`crates.io`](https://crates.io/crates/heed) are used.

`LMDB` should not need to be installed as `heed` has a build script that pulls it in automatically.

`heed`'s filenames inside Cuprate's database folder (`~/.local/share/cuprate/database/`) are:

| Filename   | Purpose |
|------------|---------|
| `data.mdb` | Main data file
| `lock.mdb` | Database lock file

`heed`-specific notes:
- [There is a maximum reader limit](https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L1372). Other potential processes (e.g. `xmrblocks`) that are also reading the `data.mdb` file need to be accounted for.
- [LMDB does not work on remote filesystem](https://github.com/LMDB/lmdb/blob/b8e54b4c31378932b69f1298972de54a565185b1/libraries/liblmdb/lmdb.h#L129).

### 3.2 `redb`
The 2nd database backend is the 100% Rust [`redb`](https://github.com/cberner/redb).

The upstream versions from [`crates.io`](https://crates.io/crates/redb) are used.

`redb`'s filenames inside Cuprate's database folder (`~/.local/share/cuprate/database/`) are:

| Filename    | Purpose |
|-------------|---------|
| `data.redb` | Main data file

TODO: document DB on remote filesystem (does redb allow this?)

### 3.3 `redb-memory`
This backend is 100% the same as `redb`, although, it uses `redb::backend::InMemoryBackend` which is a key-value store that completely resides in memory instead of a file.

All other details about this should be the same as the normal `redb` backend.

### 3.4 `sanakirja`
[`sanakirja`](https://docs.rs/sanakirja) was a candidate as a backend, however there were problems with maximum value sizes.

The default maximum value size is [1012 bytes](https://docs.rs/sanakirja/1.4.1/sanakirja/trait.Storable.html) which was too small for our requirements. Using [`sanakirja::Slice`](https://docs.rs/sanakirja/1.4.1/sanakirja/union.Slice.html) and [sanakirja::UnsizedStorage](https://docs.rs/sanakirja/1.4.1/sanakirja/trait.UnsizedStorable.html) was attempted, but there were bugs found when inserting a value in-between `512..=4096` bytes.

As such, it is not implemented.

### 3.5 `MDBX`
[`MDBX`](https://erthink.github.io/libmdbx) was a candidate as a backend, however MDBX deprecated the custom key/value comparison functions, this makes it a bit trickier to implement duplicate tables. It is also quite similar to the main backend LMDB (of which it was originally a fork of).

As such, it is not implemented (yet).

## 4. Layers
`cuprate_database` is logically abstracted into 4 layers, starting from the lowest:
1. Backend
2. Trait
3. ConcreteEnv
4. Thread-pool
5. Service

where each layer is built upon the last.

<!-- TODO: insert image here after database/ split -->

### 4.1 Backend
This is the actual database backend implementation (or a Rust shim over one).

Examples:
- `heed` (LMDB)
- `redb`

`cuprate_database` itself just uses a backend, it does not implement one.

All backends have the following attributes:
- [Embedded](https://en.wikipedia.org/wiki/Embedded_database)
- [Multiversion concurrency control](https://en.wikipedia.org/wiki/Multiversion_concurrency_control)
- [ACID](https://en.wikipedia.org/wiki/ACID)
- Are `(key, value)` oriented and have the expected API (`get()`, `insert()`, `delete()`)
- Are table oriented (`"table_name" -> (key, value)`)
- Allows concurrent readers

### 4.2 Trait
`cuprate_database` provides a set of `trait`s that abstract over the various database backends.

This allows the function signatures and behavior to stay the same but allows for swapping out databases in an easier fashion.

All common behavior of the backend's are encapsulated here and used instead of using the backend directly.

Examples:
- [`trait Env`](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/env.rs)
- [`trait {TxRo, TxRw}`](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/transaction.rs)
- [`trait {DatabaseRo, DatabaseRw}`](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/database.rs)

For example, instead of calling `LMDB` or `redb`'s `get()` function directly, `DatabaseRo::get()` is called.

### 4.3 ConcreteEnv
This is the non-generic, concrete `struct` provided by `cuprate_database` that contains all the data necessary to operate the database. The actual database backend `ConcreteEnv` will use internally depends on which backend feature is used.

`ConcreteEnv` implements `trait Env`, which opens the door to all the other traits.

The equivalent objects in the backends themselves are:
- [`heed::Env`](https://docs.rs/heed/0.20.0/heed/struct.Env.html)
- [`redb::Database`](https://docs.rs/redb/2.1.0/redb/struct.Database.html)

This is the main object used when handling the database directly, although that is not strictly necessary as a user if the `service` layer is used.

### 4.4 `ops`
These are Monero-specific functions that use the abstracted `trait` forms of the database.

Instead of dealing with the database directly (`get()`, `delete()`), the `ops` layer provides more abstract functions that deal with commonly used Monero operations (`get_block()`, `add_block()`, `pop_block()`).

### 4.5 `service`
The final layer abstracts the database completely into a Monero-specific `async` request/response API, using `tower::Service`.

It handles the database using a separate writer thread & reader thread-pool, and uses the previously mentioned `ops` functions when responding to requests.

Instead of handling the database directly, requests for data (e.g. Outputs) can be sent here and receive responses using handles that connect to this layer.

For more information on the backing thread-pool, see [`Thread model`](#thread-model).

## 5. Syncing
`cuprate_database`'s database has 5 disk syncing modes.

- FastThenSafe
- Safe
- Async
- Threshold
- Fast

The default mode is `Safe`.

This means that upon each transaction commit, all the data that was written will be fully synced to disk. This is the slowest, but safest mode of operation.

Note that upon any database `Drop`, whether via `service` or dropping the database directly, the current implementation will sync to disk regardless of any configuration.

For more information on the other modes, read the documentation [here](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/config/sync_mode.rs#L63-L144).

## 6. Thread model
As noted in the [`Layers`](#layers) section, the base database abstractions themselves are not concerned with threads, they are mostly functions to be called from a single-thread.

However, the actual API `cuprate_database` exposes for practical usage by the main `cuprated` binary (and other `async` use-cases) is the asynchronous `service` API, which _does_ have a thread model backing it.

As such, when `cuprate_database::service`'s initialization function is called, threads will be spawned.

The current system is:
- [1 writer thread](https://github.com/Cuprate/cuprate/blob/9c27ba5791377d639cb5d30d0f692c228568c122/database/src/service/write.rs#L52-L66)
- [As many reader threads as there are system threads](https://github.com/Cuprate/cuprate/blob/9c27ba5791377d639cb5d30d0f692c228568c122/database/src/service/read.rs#L104-L126)

For example, on a system with 32-threads, `cuprate_database` will spawn:
- 1 writer thread
- 32 reader threads

whose sole responsibility is to listen for database requests, access the database (potentially in parallel), and return a response.

Note that the `1 system thread = 1 reader thread` model is only the default setting, the reader thread count can be configured by the user to be any number between `1 .. amount_of_system_threads`.

The reader threads are managed by [`rayon`](https://docs.rs/rayon).

For an example of where multiple reader threads are used: given a request that asks if any key-image within a set already exists, `cuprate_database` will [split that work between the threads with `rayon`](https://github.com/Cuprate/cuprate/blob/9c27ba5791377d639cb5d30d0f692c228568c122/database/src/service/read.rs#L490-L503).

Once the [handles](https://github.com/Cuprate/cuprate/blob/9c27ba5791377d639cb5d30d0f692c228568c122/database/src/service/free.rs#L33) to these threads are `Drop`ed, the backing thread(pool) will gracefully exit, automatically.

## 7. Resizing
Database backends that require manually resizing will, by default, use a similar algorithm as `monerod`'s.

Note that this relates to the `service` module, where the database is handled by `cuprate_database` itself, not the user. In the case of a user directly using `cuprate_database`, it is up to them on how to resize.

- Each resize statically adds around [`1_073_745_920`](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/resize.rs#L104-L160) bytes to the current map size
- Resizes occur simply when the current memory map size cannot contain new data that has come in
- A resize will be attempted `3` times before failing

https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/service/write.rs#L139-L201.


## 8. (De)serialization
All the types stored inside the database are either bytes already, or are perfectly bitcast-able.

As such, they do not incur heavy (de)serialization costs when storing/fetching them from the database. The main (de)serialization used is [`bytemuck`](https://docs.rs/bytemuck)'s traits and casting functions.

The main deserialization `trait` for database storage is: [`cuprate_database::Storable`](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/storable.rs#L16-L115).

- Before storage, the type is [simply cast into bytes](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/storable.rs#L125)
- When fetching, the bytes are [simply cast into the type](hhttps://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/storable.rs#L130)

It is worth noting that when bytes are casted into the type, it is copied, the reference is not casted. This is due to byte alignment issues with both backends. This is more costly than necessary although in the main use-case for `cuprate_database`, the `service` module, the bytes would need to be owned regardless.

Practically speaking, this mean functions that normally look like such:
```rust
fn get(key: &Key) -> &Value;
```
end up looking like this in `cuprate_database`:
```rust
fn get(key: &Key) -> Value;
```

The data stored in the tables are still type-safe, but require a compatibility type to wrap them, e.g:
- [`StorableHeed<T>`](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/backend/heed/storable.rs#L11-L45)
- [`StorableRedb<T>`](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/backend/redb/storable.rs#L11-L30)