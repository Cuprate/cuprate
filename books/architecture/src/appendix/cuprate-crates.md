# Cuprate crates
This is an index of all of Cuprate's in-house crates it uses and maintains.

They are categorized into groups.

## Consensus
| Crate | In-tree path | Purpose |
|-------|--------------|---------|
| [`cuprate-consensus`](https://cuprate.org/cuprate-consensus-rules) | [`consensus/`](https://github.com/Cuprate/cuprate/tree/main/consensus) | TODO
| [`cuprate-consensus-rules`](https://cuprate.org/cuprate-consensus-rules) | [`consensus/rules/`](https://github.com/Cuprate/cuprate/tree/main/consensus-rules) | TODO
| [`cuprate-fast-sync`](https://cuprate.org/cuprate-fast-sync) | [`consensus/fast-sync/`](https://github.com/Cuprate/cuprate/tree/main/consensus/fast-sync) | Fast block synchronization

## Networking
| Crate | In-tree path | Purpose |
|-------|--------------|---------|
| [`cuprate-epee-encoding`](https://cuprate.org/cuprate-epee-encoding) | [`net/epee-encoding/`](https://github.com/Cuprate/cuprate/tree/main/net/epee-encoding) | Epee (de)serialization
| [`cuprate-fixed-bytes`](https://cuprate.org/cuprate-fixed-bytes) | [`net/fixed-bytes/`](https://github.com/Cuprate/cuprate/tree/main/net/fixed-bytes) | Fixed byte containers backed by `byte::Byte`
| [`cuprate-levin`](https://cuprate.org/cuprate-levin) | [`net/levin/`](https://github.com/Cuprate/cuprate/tree/main/net/levin) | Levin protocol implementation
| [`cuprate-wire`](https://cuprate.org/cuprate-wire) | [`net/wire/`](https://github.com/Cuprate/cuprate/tree/main/net/wire) | TODO

## P2P
| Crate | In-tree path | Purpose |
|-------|--------------|---------|
| [`cuprate-address-book`](https://cuprate.org/cuprate-address-book) | [`p2p/address-book/`](https://github.com/Cuprate/cuprate/tree/main/p2p/address-book) | TODO
| [`cuprate-async-buffer`](https://cuprate.org/cuprate-async-buffer) | [`p2p/async-buffer/`](https://github.com/Cuprate/cuprate/tree/main/p2p/async-buffer) | A bounded SPSC, FIFO, asynchronous buffer that supports arbitrary weights for values
| [`cuprate-dandelion-tower`](https://cuprate.org/cuprate-dandelion-tower) | [`p2p/dandelion-tower/`](https://github.com/Cuprate/cuprate/tree/main/p2p/dandelion-tower) | TODO
| [`cuprate-p2p`](https://cuprate.org/cuprate-p2p) | [`p2p/p2p/`](https://github.com/Cuprate/cuprate/tree/main/p2p/p2p) | TODO
| [`cuprate-p2p-core`](https://cuprate.org/cuprate-p2p-core) | [`p2p/p2p-core/`](https://github.com/Cuprate/cuprate/tree/main/p2p/p2p-core) | TODO

## Storage
| Crate | In-tree path | Purpose |
|-------|--------------|---------|
| [`cuprate-blockchain`](https://cuprate.org/cuprate-blockchain) | [`storage/blockchain/`](https://github.com/Cuprate/cuprate/tree/main/storage/blockchain) | Blockchain database built on-top of `cuprate-database` & `cuprate-database-service`
| [`cuprate-database`](https://cuprate.org/cuprate-database) | [`storage/database/`](https://github.com/Cuprate/cuprate/tree/main/storage/database) | Pure database abstraction
| [`cuprate-database-service`](https://cuprate.org/cuprate-database-service) | [`storage/database-service/`](https://github.com/Cuprate/cuprate/tree/main/storage/database-service) | `tower::Service` + thread-pool abstraction built on-top of `cuprate-database`
| [`cuprate-txpool`](https://cuprate.org/cuprate-txpool) | [`storage/txpool/`](https://github.com/Cuprate/cuprate/tree/main/storage/txpool) | Transaction pool database built on-top of `cuprate-database` & `cuprate-database-service`

## RPC
| Crate | In-tree path | Purpose |
|-------|--------------|---------|
| [`cuprate-json-rpc`](https://cuprate.org/cuprate-json-rpc) | [`rpc/json-rpc/`](https://github.com/Cuprate/cuprate/tree/main/rpc/json-rpc) | JSON-RPC 2.0 implementation
| [`cuprate-rpc-types`](https://cuprate.org/cuprate-rpc-types) | [`rpc/types/`](https://github.com/Cuprate/cuprate/tree/main/rpc/types) | Monero RPC types and traits
| [`cuprate-rpc-interface`](https://cuprate.org/cuprate-rpc-interface) | [`rpc/interface/`](https://github.com/Cuprate/cuprate/tree/main/rpc/interface) | RPC interface & routing

## 1-off crates
| Crate | In-tree path | Purpose |
|-------|--------------|---------|
| [`cuprate-cryptonight`](https://cuprate.org/cuprate-cryptonight) | [`cryptonight/`](https://github.com/Cuprate/cuprate/tree/main/cryptonight) | Cryptonight hash functions
| [`cuprate-pruning`](https://cuprate.org/cuprate-pruning) | [`pruning/`](https://github.com/Cuprate/cuprate/tree/main/pruning) | Monero pruning logic/types
| [`cuprate-helper`](https://cuprate.org/cuprate-helper) | [`helper/`](https://github.com/Cuprate/cuprate/tree/main/helper) | Kitchen-sink helper crate for Cuprate
| [`cuprate-test-utils`](https://cuprate.org/cuprate-test-utils) | [`test-utils/`](https://github.com/Cuprate/cuprate/tree/main/test-utils) | Testing utilities for Cuprate
| [`cuprate-types`](https://cuprate.org/cuprate-types) | [`types/`](https://github.com/Cuprate/cuprate/tree/main/types) | Shared types across Cuprate
