# Multimap tables
## Outputs
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

## How `cuprate_blockchain` does it
Another way to implement this is as follows:
```rust
Key = { KEY_PART_1, KEY_PART_2 }
```
```rust
Value = VALUE
```

Then the key type is simply used to look up the value; this is how `cuprate_blockchain` does it
as [`cuprate_database` does not have a multimap abstraction (yet)](../../db/issues/multimap.md).

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