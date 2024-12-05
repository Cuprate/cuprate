# Cuprate's `tower::Service` database abstraction.

This crate contains the building blocks for creating a [`tower::Service`] interface to [`cuprate_blockchain`](https://doc.cuprate.org/cuprate_blockchain).

It is split into 2 `tower::Service`s:
1. A [read service](crate::DatabaseReadService) which is backed by a [`rayon::ThreadPool`]
1. A [write service](crate::DatabaseWriteHandle) which spawns a single thread to handle write requests
