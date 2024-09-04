# tower::Service
Both [`cuprate_blockchain`](https://doc.cuprate.org/cuprate_blockchain)
and [`cuprate_txpool`](https://doc.cuprate.org/cuprate_txpool) provide
[`tower::Service`](https://docs.rs/tower)s that define how other outside
Cuprate crates access the database.

There are 2 `tower::Service`s:
1. A read service which is backed by a [`rayon::ThreadPool`](https://docs.rs/rayon)
1. A write service which spawns a single thread to handle write requests

As this behavior is the same across all users of [`cuprate_database`](https://doc.cuprate.org/cuprate_database),
it is extracted into its own crate: [`cuprate_database_service`](https://doc.cuprate.org/cuprate_database_service).