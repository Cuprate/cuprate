# Multimap
`cuprate_database` does not currently have an abstraction for [multimap tables](https://en.wikipedia.org/wiki/Multimap).

All tables are single maps of keys to values.

This matters as this means some of `cuprate_blockchain`'s tables differ from `monerod`'s tables - the primary key is stored _for all_ entries, compared to `monerod` only needing to store it once:

```rust
// `monerod` only stores `amount: 1` once,
// `cuprated` stores it each time it appears.
struct PreRctOutputId { amount: 1, amount_index: 0 }
struct PreRctOutputId { amount: 1, amount_index: 1 }
```

This means `cuprated`'s database will be slightly larger than `monerod`'s.

The current method `cuprate_blockchain` uses will be "good enough" as the multimap
keys needed for now are fixed, e.g. pre-RCT outputs are no longer being produced.

This may need to change in the future when multimap is all but required, e.g. for FCMP++.

Until then, multimap tables are not implemented as they are tricky to implement across all backends.