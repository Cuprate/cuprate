# Crates
This is an index of all of Cuprate's in-house crates it uses and maintains.

They are categorized into groups.

Crate documentation for each crate can be found by clicking the crate name or by visiting <https://doc.cuprate.org>. Documentation can also be built manually by running this at the root of the `cuprate` repository:
```bash
cargo doc --package $CRATE
```
For example, this will generate and open `cuprate-blockchain` documentation:
```bash
cargo doc --open --package cuprate-blockchain
```

## Consensus
| Crate | In-tree path | Purpose |
|-------|--------------|---------|
| [`cuprate-consensus`](https://doc.cuprate.org/cuprate_consensus) | [`consensus/`](https://github.com/Cuprate/cuprate/tree/main/consensus) | TODO
| [`cuprate-consensus-context`](https://doc.cuprate.org/cuprate_consensus_context) | [`consensus/context/`](https://github.com/Cuprate/cuprate/tree/main/consensus/context) | TODO
| [`cuprate-consensus-rules`](https://doc.cuprate.org/cuprate_consensus_rules) | [`consensus/rules/`](https://github.com/Cuprate/cuprate/tree/main/consensus/rules) | TODO
| [`cuprate-fast-sync`](https://doc.cuprate.org/cuprate_fast_sync) | [`consensus/fast-sync/`](https://github.com/Cuprate/cuprate/tree/main/consensus/fast-sync) | Fast block synchronization

## Networking
| Crate | In-tree path | Purpose |
|-------|--------------|---------|
| [`cuprate-epee-encoding`](https://doc.cuprate.org/cuprate_epee_encoding) | [`net/epee-encoding/`](https://github.com/Cuprate/cuprate/tree/main/net/epee-encoding) | Epee (de)serialization
| [`cuprate-fixed-bytes`](https://doc.cuprate.org/cuprate_fixed_bytes) | [`net/fixed-bytes/`](https://github.com/Cuprate/cuprate/tree/main/net/fixed-bytes) | Fixed byte containers backed by `byte::Byte`
| [`cuprate-levin`](https://doc.cuprate.org/cuprate_levin) | [`net/levin/`](https://github.com/Cuprate/cuprate/tree/main/net/levin) | Levin bucket protocol implementation
| [`cuprate-wire`](https://doc.cuprate.org/cuprate_wire) | [`net/wire/`](https://github.com/Cuprate/cuprate/tree/main/net/wire) | TODO

## P2P
| Crate | In-tree path | Purpose |
|-------|--------------|---------|
| [`cuprate-address-book`](https://doc.cuprate.org/cuprate_address_book) | [`p2p/address-book/`](https://github.com/Cuprate/cuprate/tree/main/p2p/address-book) | TODO
| [`cuprate-async-buffer`](https://doc.cuprate.org/cuprate_async_buffer) | [`p2p/async-buffer/`](https://github.com/Cuprate/cuprate/tree/main/p2p/async-buffer) | A bounded SPSC, FIFO, asynchronous buffer that supports arbitrary weights for values
| [`cuprate-dandelion-tower`](https://doc.cuprate.org/cuprate_dandelion_tower) | [`p2p/dandelion-tower/`](https://github.com/Cuprate/cuprate/tree/main/p2p/dandelion-tower) | TODO
| [`cuprate-p2p`](https://doc.cuprate.org/cuprate_p2p) | [`p2p/p2p/`](https://github.com/Cuprate/cuprate/tree/main/p2p/p2p) | TODO
| [`cuprate-p2p-bucket`](https://doc.cuprate.org/cuprate_p2p_bucket) | [`p2p/bucket/`](https://github.com/Cuprate/cuprate/tree/main/p2p/bucket) | A collection data structure discriminating its items into "buckets" of limited size.
| [`cuprate-p2p-core`](https://doc.cuprate.org/cuprate_p2p_core) | [`p2p/p2p-core/`](https://github.com/Cuprate/cuprate/tree/main/p2p/p2p-core) | TODO

## Storage
| Crate | In-tree path | Purpose |
|-------|--------------|---------|
| [`cuprate-blockchain`](https://doc.cuprate.org/cuprate_blockchain) | [`storage/blockchain/`](https://github.com/Cuprate/cuprate/tree/main/storage/blockchain) | Blockchain database built on-top of `cuprate-database` & `cuprate-database-service`
| [`cuprate-database`](https://doc.cuprate.org/cuprate_database) | [`storage/database/`](https://github.com/Cuprate/cuprate/tree/main/storage/database) | Pure database abstraction
| [`cuprate-database-service`](https://doc.cuprate.org/cuprate_database_service) | [`storage/database-service/`](https://github.com/Cuprate/cuprate/tree/main/storage/database-service) | `tower::Service` + thread-pool abstraction built on-top of `cuprate-database`
| [`cuprate-txpool`](https://doc.cuprate.org/cuprate_txpool) | [`storage/txpool/`](https://github.com/Cuprate/cuprate/tree/main/storage/txpool) | Transaction pool database built on-top of `cuprate-database` & `cuprate-database-service`

## RPC
| Crate | In-tree path | Purpose |
|-------|--------------|---------|
| [`cuprate-json-rpc`](https://doc.cuprate.org/cuprate_json_rpc) | [`rpc/json-rpc/`](https://github.com/Cuprate/cuprate/tree/main/rpc/json-rpc) | JSON-RPC 2.0 implementation
| [`cuprate-rpc-types`](https://doc.cuprate.org/cuprate_rpc_types) | [`rpc/types/`](https://github.com/Cuprate/cuprate/tree/main/rpc/types) | Monero RPC types and traits
| [`cuprate-rpc-interface`](https://doc.cuprate.org/cuprate_rpc_interface) | [`rpc/interface/`](https://github.com/Cuprate/cuprate/tree/main/rpc/interface) | RPC interface & routing

## ZMQ
| Crate | In-tree path | Purpose |
|-------|--------------|---------|
| [`cuprate-zmq-types`](https://doc.cuprate.org/cuprate_zmq_types) | [`zmq/types/`](https://github.com/Cuprate/cuprate/tree/main/zmq/types) | Message types for ZMQ Pub/Sub interface

## 1-off crates
| Crate | In-tree path | Purpose |
|-------|--------------|---------|
| [`cuprate-constants`](https://doc.cuprate.org/cuprate_constants) | [`constants/`](https://github.com/Cuprate/cuprate/tree/main/constants) | Shared `const/static` data across Cuprate
| [`cuprate-cryptonight`](https://doc.cuprate.org/cuprate_cryptonight) | [`cryptonight/`](https://github.com/Cuprate/cuprate/tree/main/cryptonight) | CryptoNight hash functions
| [`cuprate-pruning`](https://doc.cuprate.org/cuprate_pruning) | [`pruning/`](https://github.com/Cuprate/cuprate/tree/main/pruning) | Monero pruning logic/types
| [`cuprate-helper`](https://doc.cuprate.org/cuprate_helper) | [`helper/`](https://github.com/Cuprate/cuprate/tree/main/helper) | Kitchen-sink helper crate for Cuprate
| [`cuprate-test-utils`](https://doc.cuprate.org/cuprate_test_utils) | [`test-utils/`](https://github.com/Cuprate/cuprate/tree/main/test-utils) | Testing utilities for Cuprate
| [`cuprate-types`](https://doc.cuprate.org/cuprate_types) | [`types/`](https://github.com/Cuprate/cuprate/tree/main/types) | Shared types across Cuprate

## Benchmarks
| Crate | In-tree path | Purpose |
|-------|--------------|---------|
| [`cuprate-benchmark`](https://doc.cuprate.org/cuprate_benchmark) | [`benches/benchmark/bin/`](https://github.com/Cuprate/cuprate/tree/main/benches/benchmark/bin) | Cuprate benchmarking binary
| [`cuprate-benchmark-lib`](https://doc.cuprate.org/cuprate_benchmark_lib) | [`benches/benchmark/lib/`](https://github.com/Cuprate/cuprate/tree/main/benches/benchmark/lib) | Cuprate benchmarking library
| `cuprate-benchmark-*` | [`benches/benchmark/cuprate-*`](https://github.com/Cuprate/cuprate/tree/main/benches/benchmark/) | Benchmark for a Cuprate crate that uses `cuprate-benchmark`
| `cuprate-criterion-*` | [`benches/criterion/cuprate-*`](https://github.com/Cuprate/cuprate/tree/main/benches/criterion) | Benchmark for a Cuprate crate that uses [Criterion](https://bheisler.github.io/criterion.rs/book)