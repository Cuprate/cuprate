# Copying unaligned bytes
As mentioned in [`(De)serialization`](../serde.md), bytes are _copied_ when they are turned into a type `T` due to unaligned bytes being returned from database backends.

Using a regular reference cast results in an improperly aligned type `T`; [such a type even existing causes undefined behavior](https://doc.rust-lang.org/reference/behavior-considered-undefined.html). In our case, `bytemuck` saves us by panicking before this occurs.

Thus, when using `cuprate_database`'s database traits, an _owned_ `T` is returned.

This is doubly unfortunately for `&[u8]` as this does not even need deserialization.

For example, `StorableVec` could have been this:
```rust
enum StorableBytes<'a, T: Storable> {
    Owned(T),
    Ref(&'a T),
}
```
but this would require supporting types that must be copied regardless with the occasional `&[u8]` that can be returned without casting. This was hard to do so in a generic way, thus all `[u8]`'s are copied and returned as owned `StorableVec`s.

This is a _tradeoff_ `cuprate_database` takes as:
- `bytemuck::pod_read_unaligned` is cheap enough
- The main API, `service`, needs to return owned value anyway
- Having no references removes a lot of lifetime complexity

The alternative is either:
- Using proper (de)serialization instead of casting (which comes with its own costs)
- Somehow fixing the alignment issues in the backends mentioned previously