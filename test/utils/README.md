# Test Utils

This crate contains code that can be shared across multiple Cuprate crates tests, this crate should not be included in any
Cuprate crate, only in tests.

It currently contains:
- Code to spawn monerod instances and a testing network zone
- Real raw and typed Monero data, e.g. `Block, Transaction`
- An RPC client to generate types from `cuprate_types`
- Raw RPC request/response strings and binary data