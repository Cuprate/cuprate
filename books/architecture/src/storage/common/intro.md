# Common behavior
The crates that build on-top of the database abstraction ([`cuprate_database`](https://doc.cuprate.org/cuprate_database))
share some common behavior including but not limited to:

- Defining their specific database tables and types
- Having an `ops` module
- Exposing a `tower::Service` API (backed by a threadpool) for public usage

This section provides more details on these behaviors.