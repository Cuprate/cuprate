# Database
This is the main design document and implementation of the database used by Cuprate.

The source is located at the repository root in `database/`.

The code within `database/` is also littered with comments. Some `grep`-able keywords:

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
2. [Overview](#Overview)

---

### File Structure
A quick reference of the structure of the folders & files located in `database/src/`

| File/Folder    | Purpose |
|----------------|---------|

### Overview

