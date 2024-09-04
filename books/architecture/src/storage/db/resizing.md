# Resizing
`cuprate_database` itself does not handle memory map resizes automatically
(for database backends that need resizing, i.e. heed/LMDB).

When a user directly using `cuprate_database`, it is up to them on how to resize. The database will return [`RuntimeError::ResizeNeeded`](https://doc.cuprate.org/cuprate_database/enum.RuntimeError.html#variant.ResizeNeeded) when it needs resizing.

However, `cuprate_database` exposes some [resizing algorithms](https://doc.cuprate.org/cuprate_database/resize/index.html)
that define how the database's memory map grows.