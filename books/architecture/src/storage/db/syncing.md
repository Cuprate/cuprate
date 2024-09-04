# Syncing
`cuprate_database`'s database has 5 disk syncing modes.

1. `FastThenSafe`
1. `Safe`
1. `Async`
1. `Threshold`
1. `Fast`

The default mode is `Safe`.

This means that upon each transaction commit, all the data that was written will be fully synced to disk.
This is the slowest, but safest mode of operation.

Note that upon any database `Drop`, the current implementation will sync to disk regardless of any configuration.

For more information on the other modes, read the documentation [here](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/config/sync_mode.rs#L63-L144).
