# `cuprate-types`
Various data types shared by Cuprate.

<!-- Did you know markdown automatically increments number lists, even if they are all 1...? -->
1. [File Structure](#file-structure)
    - [`src/`](#src)

---

# File Structure
A quick reference of the structure of the folders & files in `cuprate-types`.

Note that `lib.rs/mod.rs` files are purely for re-exporting/visibility/lints, and contain no code. Each sub-directory has a corresponding `mod.rs`.

## `src/`
The top-level `src/` files.

| File                | Purpose |
|---------------------|---------|
| `service.rs`        | Types used in database requests; `enum {Request,Response}`
| `types.rs`          | Various general types used by Cuprate
