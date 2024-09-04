# Hot-swappable backends
> See also: <https://github.com/Cuprate/cuprate/issues/209>.

Using a different backend is really as simple as re-building `cuprate_database` with a different feature flag:
```bash
# Use LMDB.
cargo build --package cuprate-database --features heed

# Use redb.
cargo build --package cuprate-database --features redb
```

This is "good enough" for now, however ideally, this hot-swapping of backends would be able to be done at _runtime_.

As it is now, `cuprate_database` cannot compile both backends and swap based on user input at runtime; it must be compiled with a certain backend, which will produce a binary with only that backend.

This also means things like [CI testing multiple backends is awkward](https://github.com/Cuprate/cuprate/blob/main/.github/workflows/ci.yml#L132-L136), as we must re-compile with different feature flags instead.