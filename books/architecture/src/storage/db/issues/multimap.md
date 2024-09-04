# Multimap
Some of `cuprate_blockchain`'s tables differ from `monerod`'s tables, for example, the way multimap tables are done requires that the primary key is stored _for all_ entries, compared to `monerod` only needing to store it once.

For example:
```rust
// `monerod` only stores `amount: 1` once,
// `cuprated` stores it each time it appears.
struct PreRctOutputId { amount: 1, amount_index: 0 }
struct PreRctOutputId { amount: 1, amount_index: 1 }
```

This means `cuprated`'s database will be slightly larger than `monerod`'s.

The current method `cuprate_blockchain` uses will be "good enough" until usage shows that it must be optimized as multimap tables are tricky to implement across all backends.