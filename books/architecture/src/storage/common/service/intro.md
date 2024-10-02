# tower::Service
Both [`cuprate_blockchain`](https://doc.cuprate.org/cuprate_blockchain)
and [`cuprate_txpool`](https://doc.cuprate.org/cuprate_txpool) provide
`async` [`tower::Service`](https://docs.rs/tower)s that define database requests/responses.

The main API that other Cuprate crates use.

There are 2 `tower::Service`s:
1. A read service which is backed by a [`rayon::ThreadPool`](https://docs.rs/rayon)
1. A write service which spawns a single thread to handle write requests

As this behavior is the same across all users of [`cuprate_database`](https://doc.cuprate.org/cuprate_database),
it is extracted into its own crate: [`cuprate_database_service`](https://doc.cuprate.org/cuprate_database_service).

## Diagram
As a recap, here is how this looks to a user of a higher-level database crate,
`cuprate_blockchain` in this example. Starting from the lowest layer:

1. `cuprate_database` is used to abstract the database
1. `cuprate_blockchain` builds on-top of that with tables, types, operations
1. `cuprate_blockchain` exposes a `tower::Service` using `cuprate_database_service`
1. The user now interfaces with `cuprate_blockchain` with that `tower::Service` in a request/response fashion

```
                         ┌──────────────────┐
                         │ cuprate_database │
                         └────────┬─────────┘
┌─────────────────────────────────┴─────────────────────────────────┐
│ cuprate_blockchain                                                │
│                                                                   │
│ ┌──────────────────────┐  ┌─────────────────────────────────────┐ │
│ │ Tables, types        │  │ ops                                 │ │
│ │ ┌───────────┐┌─────┐ │  │ ┌─────────────┐ ┌──────────┐┌─────┐ │ │
│ │ │ BlockInfo ││ ... │ ├──┤ │ add_block() │ │ add_tx() ││ ... │ │ │
│ │ └───────────┘└─────┘ │  │ └─────────────┘ └──────────┘└─────┘ │ │
│ └──────────────────────┘  └─────┬───────────────────────────────┘ │
│                                 │                                 │
│                       ┌─────────┴───────────────────────────────┐ │
│                       │ tower::Service                          │ │
│                       │ ┌──────────────────────────────┐┌─────┐ │ │
│                       │ │ Blockchain{Read,Write}Handle ││ ... │ │ │
│                       │ └──────────────────────────────┘└─────┘ │ │
│                       └─────────┬───────────────────────────────┘ │
│                                 │                                 │
└─────────────────────────────────┼─────────────────────────────────┘
                                  │
		                    ┌─────┴─────┐
       ┌────────────────────┴────┐ ┌────┴──────────────────────────────────┐
       │ Database requests       │ │ Database responses                    │
       │ ┌─────────────────────┐ │ │ ┌───────────────────────────────────┐ │
       │ │ FindBlock([u8; 32]) │ │ │ │ FindBlock(Option<(Chain, usize)>) │ │
       │ └─────────────────────┘ │ │ └───────────────────────────────────┘ │
       │ ┌─────────────────────┐ │ │ ┌───────────────────────────────────┐ │
       │ │ ChainHeight         │ │ │ │ ChainHeight(usize, [u8; 32])      │ │
       │ └─────────────────────┘ │ │ └───────────────────────────────────┘ │
       │ ┌─────────────────────┐ │ │ ┌───────────────────────────────────┐ │
       │ │ ...                 │ │ │ │ ...                               │ │
       │ └─────────────────────┘ │ │ └───────────────────────────────────┘ │
       └─────────────────────────┘ └───────────────────────────────────────┘
                            ▲          │
                            │          ▼
                     ┌─────────────────────────┐
                     │ cuprate_blockchain user │
                     └─────────────────────────┘
```