# Traits abstracting backends
Although all database backends used are very similar, they have some crucial differences in small implementation details that must be worked around when conforming them to `cuprate_database`'s traits.

Put simply: using `cuprate_database`'s traits is less efficient and more awkward than using the backend directly.

For example:
- [Data types must be wrapped in compatibility layers when they otherwise wouldn't be](https://github.com/Cuprate/cuprate/blob/d0ac94a813e4cd8e0ed8da5e85a53b1d1ace2463/database/src/backend/heed/env.rs#L101-L116)
- [There are types that only apply to a specific backend, but are visible to all](https://github.com/Cuprate/cuprate/blob/d0ac94a813e4cd8e0ed8da5e85a53b1d1ace2463/database/src/error.rs#L86-L89)
- [There are extra layers of abstraction to smoothen the differences between all backends](https://github.com/Cuprate/cuprate/blob/d0ac94a813e4cd8e0ed8da5e85a53b1d1ace2463/database/src/env.rs#L62-L68)
- [Existing functionality of backends must be taken away, as it isn't supported in the others](https://github.com/Cuprate/cuprate/blob/d0ac94a813e4cd8e0ed8da5e85a53b1d1ace2463/database/src/database.rs#L27-L34)

This is a _tradeoff_ that `cuprate_database` takes, as:
- The backend itself is usually not the source of bottlenecks in the greater system, as such, small inefficiencies are OK
- None of the lost functionality is crucial for operation
- The ability to use, test, and swap between multiple database backends is [worth it](https://github.com/Cuprate/cuprate/pull/35#issuecomment-1952804393)
