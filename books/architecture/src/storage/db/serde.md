# (De)serialization
All types stored inside the database are either bytes already or are perfectly bitcast-able.

As such, they do not incur heavy (de)serialization costs when storing/fetching them from the database. The main (de)serialization used is [`bytemuck`](https://docs.rs/bytemuck)'s traits and casting functions.

## Size and layout
The size & layout of types is stable across compiler versions, as they are set and determined with [`#[repr(C)]`](https://doc.rust-lang.org/nomicon/other-reprs.html#reprc) and `bytemuck`'s derive macros such as [`bytemuck::Pod`](https://docs.rs/bytemuck/latest/bytemuck/derive.Pod.html).

Note that the data stored in the tables are still type-safe; we still refer to the key and values within our tables by the type.

## How
The main deserialization `trait` for database storage is [`Storable`](https://doc.cuprate.org/cuprate_database/trait.Storable.html).

- Before storage, the type is [simply cast into bytes](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/storable.rs#L125)
- When fetching, the bytes are [simply cast into the type](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/storable.rs#L130)

When a type is casted into bytes, [the reference is casted](https://docs.rs/bytemuck/latest/bytemuck/fn.bytes_of.html), i.e. this is zero-cost serialization.

However, it is worth noting that when bytes are casted into the type, [it is copied](https://docs.rs/bytemuck/latest/bytemuck/fn.pod_read_unaligned.html). This is due to byte alignment guarantee issues with both backends, see:
- <https://github.com/AltSysrq/lmdb-zero/issues/8>
- <https://github.com/cberner/redb/issues/360>

Without this, `bytemuck` will panic with [`TargetAlignmentGreaterAndInputNotAligned`](https://docs.rs/bytemuck/latest/bytemuck/enum.PodCastError.html#variant.TargetAlignmentGreaterAndInputNotAligned) when casting.

Copying the bytes fixes this problem, although it is more costly than necessary. However, in the main use-case for `cuprate_database` (`tower::Service` API) the bytes would need to be owned regardless as the `Request/Response` API uses owned data types (`T`, `Vec<T>`, `HashMap<K, V>`, etc).

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

Again, it's unfortunate that these must be owned, although in the `tower::Service` use-case, they would have to be owned anyway.