# Database
This is the main design document and implementation of the database used by Cuprate.

The code within `database/src` is also littered with comments. Some `grep`-able keywords:

| Word        | Meaning |
|-------------|---------|
| `INVARIANT` | This code makes an _assumption_ that must be upheld for correctness
| `SAFETY`    | This `unsafe` code is okay, for `x,y,z` reasons
| `FIXME`     | This code works but isn't ideal
| `HACK`      | This code is a brittle workaround
| `PERF`      | This code is weird for performance reasons
| `TODO`      | This has to be implemented
| `SOMEDAY`   | This should be implemented... someday

---

1. [File Structure](#file-structure)
2. [Layers](#Layers)

---

## File Structure
A quick reference of the structure of the folders & files located in `database/src/`

| File/Folder    | Purpose |
|----------------|---------|

## Layers
The database is abstracted into 5 layers internally.

Starting from the lowest layer:
1. **The database** - this is the actual database, or a thin Rust shim on-top of the database that calls database operations directly, e.g `get()`, `commit()`, `delete()`, etc
2. **The trait** - this is the `trait` that abstracts over _all_ databases and allows keeping the function signatures and behavior the same but allows for swapping out databases; each database will have this implementated located in `src/backend/`, with each database (`LMDB`, `MDBX`, `sled`, etc) having its own file defining the mappings. This `trait` is meant to cover all features across databases, and will have provided methods that may not necessarily be the most efficient - if a database can implement a method in a better way, it is re-implemented and will shadow the provided version
3. **The abstract database** - this is a concrete object and handle to _some_ database that implements the generic `trait`
4. **The thread** - this is the dedicated thread that is the logical _owner_ of the abstract database. It acts as a kernel between the async public interface and the internal database calls. This thread is responsible for converting the high-level "requests" from other Cuprate crates (`add_block()`, `get_block()`, etc) via channel messages and is responsible for doing the underlying work with the database to eventually return a "response" to the calling code, again, via a channel
5. **The `tower::Service`** - this is the public API that other Cuprate crates will interface with; the abstract database will have `tower::Service<R>` implemented for each `R`, where `R` is a specific high-level request other Cuprate crates need, e.g. `add_block()`,  `get_block()`, etc - this request is executed by "the thread" which eventually returns the result of the function

<div align="center">
    <img src="https://github.com/hinto-janai/cuprate/assets/101352116/b7d7cbe3-ce55-44ea-92cc-ecde10cf519a" width="50%"/>
</div>
