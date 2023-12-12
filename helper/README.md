## Helper
`helper/` is the kitchen-sink crate for very generic, not necessarily Cuprate specific functions, types, etc.

This allows all workspace crates to share, and aids compile times.

If a 3rd party's crate/functions/types are small enough, it could be moved here to trim dependencies and allow easy modifications.
