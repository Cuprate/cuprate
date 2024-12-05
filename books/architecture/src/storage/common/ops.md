# `ops`
Both [`cuprate_blockchain`](https://doc.cuprate.org/cuprate_blockchain)
and [`cuprate_txpool`](https://doc.cuprate.org/cuprate_txpool) expose an
`ops` module containing abstracted abstracted Monero-related database operations.

For example, [`cuprate_blockchain::ops::block::add_block`](https://doc.cuprate.org/cuprate_blockchain/ops/block/fn.add_block.html).

These functions build on-top of the database traits and allow for more abstracted database operations.

For example, instead of these signatures:
```rust
fn get(_: &Key) -> Value;
fn put(_: &Key, &Value);
```
the `ops` module provides much higher-level signatures like such:
```rust
fn add_block(block: &Block) -> Result<_, _>;
```

Although these functions are exposed, they are not the main API, that would be next section:
the [`tower::Service`](./service/intro.md) (which uses these functions).