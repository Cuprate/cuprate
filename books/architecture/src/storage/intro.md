# Storage
This section covers all things related to the on-disk storage of data within Cuprate.

## Overview
The quick overview is that Cuprate has a [database abstraction crate](./database-abstraction.md)
that handles "low-level" database details such as key and value (de)serialization, tables, transactions, etc.

This database abstraction crate is then used by all crates that need on-disk storage, i.e. the
- [Blockchain database](./blockchain/intro.md)
- [Transaction pool database](./txpool/intro.md)

## Service
The interface provided by all crates building on-top of the
database abstraction is a [`tower::Service`](https://docs.rs/tower), i.e.
database requests/responses are sent/received asynchronously.

As the interface details are similar across crates (threadpool, read operations, write operations),
the interface itself is abstracted in the [`cuprate_database_service`](./common/service/intro.md) crate,
which is then used by the crates.

## Diagram
This is roughly how database crates are set up.

```text
                                                           ┌─────────────────┐
┌──────────────────────────────────┐                       │                 │
│ Some crate that needs a database │  ┌────────────────┐   │                 │
│                                  │  │     Public     │   │                 │
│ ┌──────────────────────────────┐ │─►│ tower::Service │◄─►│ Rest of Cuprate │
│ │     Database abstraction     │ │  │      API       │   │                 │
│ └──────────────────────────────┘ │  └────────────────┘   │                 │
└──────────────────────────────────┘                       │                 │
                                                           └─────────────────┘
```
