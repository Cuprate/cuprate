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
1. [Schema](#database-schema)

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
| `key.rs`         | Abstracted database keys; `trait Key`
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

# Database Schema

This document contains Cuprate's database schema, it may change a lot during this stage of development, nothing here is final.

## Transactions

### Tx IDs

This table will map a tx hash to tx id (u64).

| Key      | Value |
| -------- | ----- |
| [u8; 32] | u64   |

`Constant Size = true`

### Tx Heights

This table will map a tx to the block height it came from.

| Key        | Value        |
| ---------- | ------------ |
| TxID (u64) | Height (u64) |

`Constant Size = true`

### Tx Unlock Time

This table will store the unlock time of a tx (only if the tx has a non-zero lock-time).

| Key        | Value             |
| ---------- | ----------------- |
| TxID (u64) | unlock time (u64) |

`Constant Size = true`

### Pruned Tx Blobs

This table will contain pruned tx blobs (even if the DB is not pruned).

| Key        | Value                 |
| ---------- | --------------------- |
| TxID (u64) | Pruned Blob (Vec<u8>) |

`Constant Size = false`

### Prunable Tx Blobs

This table will contain the prunable part of a tx.

| Key        | Value                   |
| ---------- | ----------------------- |
| TxID (u64) | Prunable Blob (Vec<u8>) |

`Constant Size = false`

### Prunable Hash

This table will contain the hash of the prunable part of a tx.

| Key        | Value                    |
| ---------- | ------------------------ |
| TxID (u64) | Prunable Hash ([u8; 32]) |

`Constant Size = true`

## Tx Outputs

This table gives the amount index's of the transaction outputs.

| Key        | Value                  |
| ---------- | ---------------------- |
| TxID (u64) | Amount Idxs (Vec<u64>) |

`Constant Size = false`

## Outputs

### CryptoNote Outputs

This table will contain legacy CryptoNote outputs which have clear amounts.
This table will not contain an output with 0 amount.

| Primary Key  | Secondary Key      | Value              |
| ------------ | ------------------ | ------------------ |
| Amount (u64) | Amount Index (u32) | Output (See below) |

> This table stores the amount idex as a u32 to save space as the creation of v1 txs is limited, u32::MAX should never be hit.

```rust
struct Output{
    key: [u8; 32],
    // We could get this from the tx_idx with the Tx Heights table but that would require another look up per out.
    height: u64, 
    tx_idx: u64,
    // For if the tx that created this out has a time-lock - this means we only need to look on Tx Unlock Time if this is true.
    locked: bool 
}
// TODO: local_index?

```

`Constant Size = true`

### RingCT Outputs

| Key                | Value                 |
| ------------------ | --------------------- |
| Amount Index (u64) | RctOutput (see below) |

```rust
struct RctOutput{
    key: [u8; 32],
    // We could get this from the tx_idx with the Tx Heights table but that would require another look up per out.
    height: u64, 
    tx_idx: u64,
    // For if the tx that created this out has a time-lock - this means we only need to look on Tx Unlock Time if this is true.
    locked: bool,
    // The amount commitment of this output.
    commitment: [u8; 32]
}
// TODO: local_index?

```

`Constant Size = true`

> TODO: We could split this table again into `RingCT (non-miner) Outputs` and `RingCT (miner) Outputs` as for miner outputs we can
> store the amount instead of commitment saving 24 bytes per miner output.

## Key Images

### Key Images

This table stores tx key images

| Key                  | Value |
| -------------------- | ----- |
| Key Image ([u8; 32]) | ()    |

`Constant Size = true`

## Blocks

### Block Heights

Maps a block hash to a height.

| Key                   | Value              |
| --------------------- | ------------------ |
| Block Hash ([u8; 32]) | Block Height (u64) |

`Constant Size = true`

### Block Blob

Stores the blocks blob.

| Key                | Value                |
| ------------------ | -------------------- |
| Block Height (u64) | Block Blob (Vec<u8>) |

`Constant Size = false`

### Block Info V1

Stores info about blocks up to HF 4

| Key                | Value                   |
| ------------------ | ----------------------- |
| Block Height (u64) | BlockInfoV1 (see below) |

```rust
struct BlockInfoV1 {
    timestamp: u64,
    total_generated_coins: u64,
    weight: u64,
    cumulative_difficulty: u64,
    block_hash: [u8; 32],
}
```

`Constant Size = true`

### Block Info V2

Stores info about blocks between HF 4 and 10

| Key                | Value                   |
| ------------------ | ----------------------- |
| Block Height (u64) | BlockInfoV2 (see below) |

```rust
struct BlockInfoV2 {
    timestamp: u64,
    total_generated_coins: u64,
    weight: u64,
    cumulative_difficulty: u64,
    block_hash: [u8; 32],
    cumulative_rct_outs: u32
}
```

`Constant Size = true`

### Block Info V3

Stores info about blocks from HF 10 onwards

| Key                | Value                   |
| ------------------ | ----------------------- |
| Block Height (u64) | BlockInfoV3 (see below) |

```rust
struct BlockInfoV3 {
    timestamp: u64,
    total_generated_coins: u64,
    weight: u64,
    cumulative_difficulty: u128,
    block_hash: [u8; 32],
    cumulative_rct_outs: u64,
    long_term_weight: u64
}
```

`Constant Size = true`

> When getting a blocks info start on table V1 and if a block isn't there move to the next or keep a cache of numb blocks in each table
> so we know what table to look in by the blocks height.

# MultiMap Tables

When referencing outputs Monero will use the amount and the amount index. This means 2 keys are needed to reach an output.

With LMDB you can set `DUP_SORT` on a table and then set the key/ value to:

```
KEY: KEY_PART_1
```

```
VALUE: {
  KEY_PART_2,
  VALUE // The acctual value we are storing.
}
```

Then you can set a custom value sort that only take into account `KEY_PART_2`. This is how monerod does it. 

This requires that the underlying DB supports multimap tables, custom sort functions on values and setting a cursor on a specific
key/value.

Another way to implement this is as follows:

```
KEY: KEY_PART_1 | KEY_PART_2
```

```
VALUE: VALUE
```

We also need to get the amount of outputs with a certain amount so the database will need to allow getting keys less than a key
i.e. to get the number of outputs with amount `10` we would get the first key below `(10 | MAX)` and add one to `KEY_PART_2`.

We also have to make sure the DB is storing these values in the correct order for this to work.
