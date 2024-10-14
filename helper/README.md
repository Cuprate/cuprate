## Helper
`helper/` is the kitchen-sink crate for very generic, not necessarily Cuprate specific functions, types, etc.

This allows all workspace crates to share, and aids compile times.

If a 3rd party's crate/functions/types are small enough, it could be moved here to trim dependencies and allow easy modifications.

## Features
Modules can be selectively used/compiled with cargo's `--feature` or `features = ["..."]`.

All features are off by default.

See [`Cargo.toml`](Cargo.toml)'s `[features]` table to see what features there are and what they enable.

Special non-module related features:
- `serde`: Enables serde implementations on applicable types
- `std`: Enables usage of `std`

## `#[no_std]`
Each modules documents whether it requires `std` or not.

If a module that requires `std` is enabled, `helper` will automatically use `std`.