# Database abstraction
[`cuprate_database`](https://doc.cuprate.org/cuprate_database) is Cuprate’s database abstraction.

This crate abstracts various database backends with `trait`s.

All backends have the following attributes:

- [Embedded](https://en.wikipedia.org/wiki/Embedded_database)
- [Multiversion concurrency control](https://en.wikipedia.org/wiki/Multiversion_concurrency_control)
- [ACID](https://en.wikipedia.org/wiki/ACID)
- Are `(key, value)` oriented and have the expected API (`get()`, `insert()`, `delete()`)
- Are table oriented (`"table_name" -> (key, value)`)
- Allows concurrent readers

The currently implemented backends are:
- [`heed`](https://github.com/meilisearch/heed) (LMDB)
- [`redb`](https://github.com/cberner/redb)

Said precicely, `cuprate_database` is the embedded database other Cuprate
crates interact with instead of using any particular backend implementation.
This allows the backend to be swapped and/or future backends to be implemented.