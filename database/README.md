# Database
Cuprate's database implementation.

- [1. Documentation](#1-documentation)
- [2. File structure](#2-file-structure)
    - [2.1 `src/`](#21-src)
    - [2.2 `src/backend/`](#22-srcbackend)
    - [2.3 `src/config/`](#23-srcconfig)
    - [2.4 `src/ops/`](#24-srcops)
    - [2.5 `src/service/`](#25-srcservice)
- [3. Backends](#3-backends)
    - [3.1 heed](#31-heed)
    - [3.2 redb](#32-redb)
    - [3.3 redb-memory](#33-redb-memory)
    - [3.4 sanakirja](#34-sanakirja)
    - [3.5 MDBX](#35-mdbx)
- [4. Layers](#4-layers)
    - [4.1 Backend](#41-backend)
    - [4.2 Trait](#42-trait)
    - [4.3 ConcreteEnv](#43-concreteenv)
    - [4.4 `ops`](#44-ops)
    - [4.5 `service`](#45-service)
- [5. The service](#5-the-service)
    - [5.1 Initialization](#51-initialization)
    - [5.2 Requests](#53-requests)
    - [5.3 Responses](#54-responses)
    - [5.4 Thread model](#52-thread-model)
    - [5.5 Shutdown](#55-shutdown)
- [6. Syncing](#6-Syncing)
- [7. Resizing](#7-resizing)
- [8. (De)serialization](#8-deserialization)
- [9. Schema](#9-schema)
    - [9.1 Tables](#91-tables)
    - [9.2 Multimap tables](#92-multimap-tables)
- [10. Known issues and tradeoffs](#10-known-issues-and-tradeoffs)
    - [10.1 Traits abstracting backends](#101-traits-abstracting-backends)
    - [10.2 Hot-swappable backends](#102-hot-swappable-backends)
    - [10.3 Copying unaligned bytes](#103-copying-unaligned-bytes)
    - [10.4 Non-fixed sized data](#104-non-fixed-sized-data)
    - [10.5 Endianness](#105-endianness)
    - [10.6 Extra tables](#106-extra-tables)

---

## 1. Documentation
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

## 2. File structure
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
| `unsafe_unsendable.rs` | Marker type to impl `Send` for objects not `Send`

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
This folder contains the `cupate_database::config` module; configuration options for the database.

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

### 3.1 heed
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

### 3.2 redb
The 2nd database backend is the 100% Rust [`redb`](https://github.com/cberner/redb).

The upstream versions from [`crates.io`](https://crates.io/crates/redb) are used.

`redb`'s filenames inside Cuprate's database folder (`~/.local/share/cuprate/database/`) are:

| Filename    | Purpose |
|-------------|---------|
| `data.redb` | Main data file

<!-- TODO: document DB on remote filesystem (does redb allow this?) -->

### 3.3 redb-memory
This backend is 100% the same as `redb`, although, it uses `redb::backend::InMemoryBackend` which is a key-value store that completely resides in memory instead of a file.

All other details about this should be the same as the normal `redb` backend.

### 3.4 sanakirja
[`sanakirja`](https://docs.rs/sanakirja) was a candidate as a backend, however there were problems with maximum value sizes.

The default maximum value size is [1012 bytes](https://docs.rs/sanakirja/1.4.1/sanakirja/trait.Storable.html) which was too small for our requirements. Using [`sanakirja::Slice`](https://docs.rs/sanakirja/1.4.1/sanakirja/union.Slice.html) and [sanakirja::UnsizedStorage](https://docs.rs/sanakirja/1.4.1/sanakirja/trait.UnsizedStorable.html) was attempted, but there were bugs found when inserting a value in-between `512..=4096` bytes.

As such, it is not implemented.

### 3.5 MDBX
[`MDBX`](https://erthink.github.io/libmdbx) was a candidate as a backend, however MDBX deprecated the custom key/value comparison functions, this makes it a bit trickier to implement duplicate tables. It is also quite similar to the main backend LMDB (of which it was originally a fork of).

As such, it is not implemented (yet).

## 4. Layers
`cuprate_database` is logically abstracted into 5 layers, starting from the lowest:
1. Backend
2. Trait
3. ConcreteEnv
4. `ops`
5. `service`

Each layer is built upon the last.

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

Instead of dealing with the database directly (`get()`, `delete()`), the `ops` layer provides more abstract functions that deal with commonly used Monero operations (`add_block()`, `pop_block()`).

### 4.5 `service`
The final layer abstracts the database completely into a [Monero-specific `async` request/response API](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/types/src/service.rs#L18-L78), using [`tower::Service`](https://docs.rs/tower/latest/tower/trait.Service.html).

For more information on this layer, see the next section: [`The service`](#5-the-service).

## 5. The service
The main API `cuprate_database` exposes for other crates to use is the `cuprate_database::service` module.

This module exposes an `async` request/response API with `tower::Service`, backed by a threadpool, that allows reading/writing Monero-related data to the database.

`cuprate_database::service` itself manages the database using a separate writer thread & reader thread-pool, and uses the previously mentioned [`ops`](#44-ops) functions when responding to requests.

### 5.1 Initialization
The service is started simply by calling: [`cuprate_database::service::init()`](https://github.com/Cuprate/cuprate/blob/d0ac94a813e4cd8e0ed8da5e85a53b1d1ace2463/database/src/service/free.rs#L23). From a normal user's perspective, this is the last `cuprate_database` function that will be called.

This function initializes the database, spawns threads, and returns a:
- Read handle to the database (cloneable)
- Write handle to the database (not cloneable)

These "handles" implement the `tower::Service` trait, which allows sending requests and receiving responses `async`hronously.

### 5.2 Requests
Along with the 2 handles, there are 2 types of requests:
- [`ReadRequest`](https://github.com/Cuprate/cuprate/blob/d0ac94a813e4cd8e0ed8da5e85a53b1d1ace2463/types/src/service.rs#L23-L90)
- [`WriteRequest`](https://github.com/Cuprate/cuprate/blob/d0ac94a813e4cd8e0ed8da5e85a53b1d1ace2463/types/src/service.rs#L93-L105)

`ReadRequest` is for retrieving various types of information from the database.

`WriteRequest` currently only has 1 variant: to write a block to the database.

### 5.3 Responses
After sending one of the above requests using the read/write handle, the value returned is _not_ the response, yet an `async`hronous  channel that will eventually return the response:
```rust,ignore
let response_channel: Channel = read_handle.call(ReadResponse::ChainHeight)?;
let response: ReadResponse = response_channel.await?;

assert_eq!(matches!(response), Response::ChainHeight(_));
```

After `await`'ing upon the channel, a `Response` will be returned when the `service` threadpool has fetched the value from the database and sent it off. Note that this channel is a oneshot channel, it can be dropped after retrieving the response as it will never receive a message again.

Both read/write requests variants match in name with `Response` types, i.e.
- `ReadRequest::ChainHeight` leads to `Response::ChainHeight`
- `WriteRequest::WriteBlock` leads to `Response::WriteBlockOk`

### 5.4 Thread model
As noted in the [`Layers`](#layers) section, the base database abstractions themselves are not concerned with parallelism, they are mostly functions to be called from a single-thread.

However, the `cuprate_database::service` API, _does_ have a thread model backing it.

When [`cuprate_database::service`'s initialization function](https://github.com/Cuprate/cuprate/blob/9c27ba5791377d639cb5d30d0f692c228568c122/database/src/service/free.rs#L33-L44) is called, threads will be spawned and maintained until the user drops (disconnects) the returned handles.

The current behavior is:
- [1 writer thread](https://github.com/Cuprate/cuprate/blob/9c27ba5791377d639cb5d30d0f692c228568c122/database/src/service/write.rs#L52-L66)
- [As many reader threads as there are system threads](https://github.com/Cuprate/cuprate/blob/9c27ba5791377d639cb5d30d0f692c228568c122/database/src/service/read.rs#L104-L126)

For example, on a system with 32-threads, `cuprate_database` will spawn:
- 1 writer thread
- 32 reader threads

whose sole responsibility is to listen for database requests, access the database (potentially in parallel), and return a response.

Note that the `1 system thread = 1 reader thread` model is only the default setting, the reader thread count can be configured by the user to be any number between `1 .. amount_of_system_threads`.

The reader threads are managed by [`rayon`](https://docs.rs/rayon).

For an example of where multiple reader threads are used: given a request that asks if any key-image within a set already exists, `cuprate_database` will [split that work between the threads with `rayon`](https://github.com/Cuprate/cuprate/blob/9c27ba5791377d639cb5d30d0f692c228568c122/database/src/service/read.rs#L490-L503).

### 5.5 Shutdown
Once the read/write handles are `Drop`ed, the backing thread(pool) will gracefully exit, automatically.

Note the writer thread and reader threadpool aren't connected whatsoever; dropping the write handle will make the writer thread exit, however, the reader handle is free to be held onto and can be continued to be read from - and vice-versa for the write handle.

## 6. Syncing
`cuprate_database`'s database has 5 disk syncing modes.

1. FastThenSafe
1. Safe
1. Async
1. Threshold
1. Fast

The default mode is `Safe`.

This means that upon each transaction commit, all the data that was written will be fully synced to disk. This is the slowest, but safest mode of operation.

Note that upon any database `Drop`, whether via `service` or dropping the database directly, the current implementation will sync to disk regardless of any configuration.

For more information on the other modes, read the documentation [here](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/config/sync_mode.rs#L63-L144).

## 7. Resizing
Database backends that require manually resizing will, by default, use a similar algorithm as `monerod`'s.

Note that this only relates to the `service` module, where the database is handled by `cuprate_database` itself, not the user. In the case of a user directly using `cuprate_database`, it is up to them on how to resize.

Within `service`, the resizing logic defined [here](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/service/write.rs#L139-L201) does the following:

- If there's not enough space to fit a write request's data, start a resize
- Each resize adds around [`1_073_745_920`](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/resize.rs#L104-L160) bytes to the current map size
- A resize will be attempted `3` times before failing

There are other [resizing algorithms](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/resize.rs#L38-L47) that define how the database's memory map grows, although currently the behavior of [`monerod`](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/resize.rs#L104-L160) is closely followed.

## 8. (De)serialization
All types stored inside the database are either bytes already, or are perfectly bitcast-able.

As such, they do not incur heavy (de)serialization costs when storing/fetching them from the database. The main (de)serialization used is [`bytemuck`](https://docs.rs/bytemuck)'s traits and casting functions.

The size & layout of types is stable across compiler versions, as they are set and determined with [`#[repr(C)]`](https://doc.rust-lang.org/nomicon/other-reprs.html#reprc) and `bytemuck`'s derive macros such as [`#[derive(bytemuck::Pod)]`](https://docs.rs/bytemuck/latest/bytemuck/derive.Pod.html).

Note that the data stored in the tables are still type-safe; we still refer to the key and values within our tables by the type.

The main deserialization `trait` for database storage is: [`cuprate_database::Storable`](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/storable.rs#L16-L115).

- Before storage, the type is [simply cast into bytes](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/storable.rs#L125)
- When fetching, the bytes are [simply cast into the type](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/storable.rs#L130)

When a type is casted into bytes, [the reference is casted](https://docs.rs/bytemuck/latest/bytemuck/fn.bytes_of.html), i.e. this is zero-cost serialization.

However, it is worth noting that when bytes are casted into the type, [it is copied](https://docs.rs/bytemuck/latest/bytemuck/fn.pod_read_unaligned.html). This is due to byte alignment guarantee issues with both backends, see:
- https://github.com/AltSysrq/lmdb-zero/issues/8
- https://github.com/cberner/redb/issues/360

Without this, `bytemuck` will panic with [`TargetAlignmentGreaterAndInputNotAligned`](https://docs.rs/bytemuck/latest/bytemuck/enum.PodCastError.html#variant.TargetAlignmentGreaterAndInputNotAligned) when casting.

Copying the bytes fixes this problem, although it is more costly than necessary. However, in the main use-case for `cuprate_database` (the `service` module) the bytes would need to be owned regardless as the `Request/Response` API uses owned data types (`T`, `Vec<T>`, `HashMap<K, V>`, etc).

Practically speaking, this means lower-level database functions that normally look like such:
```rust
fn get(key: &Key) -> &Value;
```
end up looking like this in `cuprate_database`:
```rust
fn get(key: &Key) -> Value;
```

Since each backend has its own (de)serialization methods, our types are wrapped in compatibility types that map our `Storable` functions into whatever is required for the backend, e.g:
- [`StorableHeed<T>`](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/backend/heed/storable.rs#L11-L45)
- [`StorableRedb<T>`](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/backend/redb/storable.rs#L11-L30)

Compatibility structs also exist for any `Storable` containers:
- [`StorableVec<T>`](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/storable.rs#L135-L191)
- [`StorableBytes`](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/storable.rs#L208-L241)

Again, it's unfortunate that these must be owned, although in `service`'s use-case, they would have to be owned anyway.

## 9. Schema
This following section contains Cuprate's database schema, it may change throughout the development of Cuprate, as such, nothing here is final.

### 9.1 Tables
The `CamelCase` names of the table headers documented here (e.g. `TxIds`) are the actual type name of the table within `cuprate_database`.

Note that words written within `code blocks` mean that it is a real type defined and usable within `cuprate_database`. Other standard types like u64 and type aliases (TxId) are written normally.

Within `cuprate_database::tables`, the below table is essentially defined as-is with [a macro](https://github.com/Cuprate/cuprate/blob/31ce89412aa174fc33754f22c9a6d9ef5ddeda28/database/src/tables.rs#L369-L470).

Many of the data types stored are the same data types, although are different semantically, as such, a map of aliases used and their real data types is also provided below.

| Alias                                              | Real Type |
|----------------------------------------------------|-----------|
| BlockHeight, Amount, AmountIndex, TxId, UnlockTime | u64
| BlockHash, KeyImage, TxHash, PrunableHash          | [u8; 32]

| Table             | Key                  | Value              | Description |
|-------------------|----------------------|--------------------|-------------|
| `BlockBlobs`      | BlockHeight          | `StorableVec<u8>`  | Maps a block's height to a serialized form block blobs (bytes)
| `BlockHeights`    | BlockHash            | BlockHeight        | Maps a block's hash to its height
| `BlockInfos`      | BlockHeight          | `BlockInfo`        | Contains metadata of all blocks
| `KeyImages`       | KeyImage             | ()                 | This table is a set with no value, it stores transaction key images
| `NumOutputs`      | Amount               | u64                | Maps an output's amount to the number of outputs with that amount
| `Outputs`         | `PreRctOutputId`     | `Output`           | This table contains legacy CryptoNote outputs which have clear amounts. This table will not contain an output with 0 amount.
| `PrunedTxBlobs`   | TxId                 | `StorableVec<u8>`  | Contains pruned transaction blobs (even if the database is not pruned)
| `PrunableTxBlobs` | TxId                 | `StorableVec<u8>`  | Contains the prunable part of a transaction
| `PrunableHashes`  | TxId                 | PrunableHash       | Contains the hash of the prunable part of a transaction
| `RctOutputs`      | AmountIndex          | `RctOutput`        | Contains RingCT outputs mapped from their global RCT index
| `TxBlobs`         | TxId                 | `StorableVec<u8>`  | Serialized transaction blobs (bytes)
| `TxIds`           | TxHash               | TxId               | Maps a transaction's hash to its index/ID
| `TxHeights`       | TxId                 | BlockHeight        | Maps a transaction's ID to the height of the block it comes from
| `TxOutputs`       | TxId                 | `StorableVec<u64>` | Gives the amount indices of a transaction's outputs
| `TxUnlockTime`    | TxId                 | UnlockTime         | Stores the unlock time of a transaction (only if it has a non-zero lock time)

The definitions for aliases and types (e.g. `RctOutput`) are within the [`cuprate_database::types`](https://github.com/Cuprate/cuprate/blob/31ce89412aa174fc33754f22c9a6d9ef5ddeda28/database/src/types.rs#L51) module.

<!-- TODO(Boog900): We could split this table again into `RingCT (non-miner) Outputs` and `RingCT (miner) Outputs` as for miner outputs we can store the amount instead of commitment saving 24 bytes per miner output. -->

### 9.2 Multimap tables
When referencing outputs, Monero will [use the amount and the amount index](https://github.com/monero-project/monero/blob/c8214782fb2a769c57382a999eaf099691c836e7/src/blockchain_db/lmdb/db_lmdb.cpp#L3447-L3449). This means 2 keys are needed to reach an output.

With LMDB you can set the `DUP_SORT` flag on a table and then set the key/value to:
```rust
Key = KEY_PART_1
```
```rust
Value = {
    KEY_PART_2,
    VALUE // The actual value we are storing.
}
```

Then you can set a custom value sorting function that only takes `KEY_PART_2` into account; this is how `monerod` does it.

This requires that the underlying database supports:
- multimap tables
- custom sort functions on values
- setting a cursor on a specific key/value

---

Another way to implement this is as follows:
```rust
Key = { KEY_PART_1, KEY_PART_2 }
```
```rust
Value = VALUE
```

Then the key type is simply used to look up the value; this is how `cuprate_database` does it.

For example, the key/value pair for outputs is:
```rust
PreRctOutputId => Output
```
where `PreRctOutputId` looks like this:
```rust
struct PreRctOutputId {
    amount: u64,
    amount_index: u64,
}
```

<!-- TODO(Boog900):  We also need to get the amount of outputs with a certain amount so the database will need to allow getting keys less than a key i.e. to get the number of outputs with amount `10` we would get the first key below `(10 | MAX)` and add one to `KEY_PART_2`. We also have to make sure the DB is storing these values in the correct order for this to work. -->

## 10. Known issues and tradeoffs
### 10.1 Traits abstracting backends
### 10.2 Hot-swappable backends
### 10.3 Copying unaligned bytes
### 10.4 Non-fixed sized data
### 10.5 Endianness
### 10.6 Extra tables