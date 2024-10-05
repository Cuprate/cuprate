# Thread model
The base database abstractions themselves are not concerned with parallelism, they are mostly functions to be called from a single-thread.

However, the `cuprate_database_service` API, _does_ have a thread model backing it.

When a `Service`'s init() function is called, threads will be spawned and
maintained until the user drops (disconnects) the returned handles.

The current behavior for thread count is:
- [1 writer thread](https://github.com/Cuprate/cuprate/blob/0941f68efcd7dfe66124ad0c1934277f47da9090/storage/service/src/service/write.rs#L48-L52)
- [As many reader threads as there are system threads](https://github.com/Cuprate/cuprate/blob/0941f68efcd7dfe66124ad0c1934277f47da9090/storage/service/src/reader_threads.rs#L44-L49)

For example, on a system with 32-threads, `cuprate_database_service` will spawn:
- 1 writer thread
- 32 reader threads

whose sole responsibility is to listen for database requests, access the database (potentially in parallel), and return a response.

Note that the `1 system thread = 1 reader thread` model is only the default setting, the reader thread count can be configured by the user to be any number between `1 .. amount_of_system_threads`.

The reader threads are managed by [`rayon`](https://docs.rs/rayon).

For an example of where multiple reader threads are used: given a request that asks if any key-image within a set already exists, `cuprate_blockchain` will [split that work between the threads with `rayon`](https://github.com/Cuprate/cuprate/blob/0941f68efcd7dfe66124ad0c1934277f47da9090/storage/blockchain/src/service/read.rs#L400).