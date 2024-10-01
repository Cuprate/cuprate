# Backend
First, we need an actual database implementation.

`cuprate-database`'s `trait`s allow abstracting over the actual database, such that any backend in particular could be used.

This page is an enumeration of all the backends Cuprate has, has tried, and may try in the future.

## `heed`
The default database used is [`heed`](https://github.com/meilisearch/heed) (LMDB). The upstream versions from [`crates.io`](https://crates.io/crates/heed) are used. `LMDB` should not need to be installed as `heed` has a build script that pulls it in automatically.

`heed`'s filenames inside Cuprate's data folder are:

| Filename   | Purpose |
|------------|---------|
| `data.mdb` | Main data file
| `lock.mdb` | Database lock file

`heed`-specific notes:
- [There is a maximum reader limit](https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L1372). Other potential processes (e.g. `xmrblocks`) that are also reading the `data.mdb` file need to be accounted for
- [LMDB does not work on remote filesystem](https://github.com/LMDB/lmdb/blob/b8e54b4c31378932b69f1298972de54a565185b1/libraries/liblmdb/lmdb.h#L129)

## `redb`
The 2nd database backend is the 100% Rust [`redb`](https://github.com/cberner/redb).

The upstream versions from [`crates.io`](https://crates.io/crates/redb) are used.

`redb`'s filenames inside Cuprate's data folder are:

| Filename    | Purpose |
|-------------|---------|
| `data.redb` | Main data file

<!-- TODO: document DB on remote filesystem (does redb allow this?) -->

## `redb-memory`
This backend is 100% the same as `redb`, although, it uses [`redb::backend::InMemoryBackend`](https://docs.rs/redb/2.1.2/redb/backends/struct.InMemoryBackend.html) which is a database that completely resides in memory instead of a file.

All other details about this should be the same as the normal `redb` backend.

## `sanakirja`
[`sanakirja`](https://docs.rs/sanakirja) was a candidate as a backend, however there were problems with maximum value sizes.

The default maximum value size is [1012 bytes](https://docs.rs/sanakirja/1.4.1/sanakirja/trait.Storable.html) which was too small for our requirements. Using [`sanakirja::Slice`](https://docs.rs/sanakirja/1.4.1/sanakirja/union.Slice.html) and [sanakirja::UnsizedStorage](https://docs.rs/sanakirja/1.4.1/sanakirja/trait.UnsizedStorable.html) was attempted, but there were bugs found when inserting a value in-between `512..=4096` bytes.

As such, it is not implemented.

## `MDBX`
[`MDBX`](https://erthink.github.io/libmdbx) was a candidate as a backend, however MDBX deprecated the custom key/value comparison functions, this makes it a bit trickier to implement multimap tables. It is also quite similar to the main backend LMDB (of which it was originally a fork of).

As such, it is not implemented (yet).
