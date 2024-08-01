# Cuprate's `tower::Service` database abstraction.

This crate contains the building blocks for creating a `tower::Service` interface to `cuprate_blockchain`.

It is split into 2 `tower::Service`s a read service which is backed by a `rayon::ThreadPool` and a write service which
spawns a single thread to handle write requests.
