# Summary

[Cuprate Architecture](cuprate-architecture.md)
[🟡 Foreword](foreword.md)

---

- [🟠 Intro](intro/intro.md)
    - [🟡 Who this book is for](intro/who-this-book-is-for.md)
    - [🔴 Required knowledge](intro/required-knowledge.md)
    - [🔴 How to use this book](intro/how-to-use-this-book.md)

---

- [⚪️ Bird's eye view](birds-eye-view/intro.md)
    - [⚪️ Map](birds-eye-view/map.md)
    - [⚪️ Components](birds-eye-view/components.md)

---

- [⚪️ Formats, protocols, types](formats-protocols-types/intro.md)
    - [⚪️ monero_oxide](formats-protocols-types/monero-oxide.md)
    - [⚪️ cuprate_types](formats-protocols-types/cuprate-types.md)
    - [⚪️ cuprate_helper](formats-protocols-types/cuprate-helper.md)
    - [⚪️ Epee](formats-protocols-types/epee.md)
    - [⚪️ Levin](formats-protocols-types/levin.md)

---

- [🟢 Storage](storage/intro.md)
    - [🟢 Database abstraction](storage/db/intro.md)
        - [🟢 Abstraction](storage/db/abstraction/intro.md)
            - [🟢 Backend](storage/db/abstraction/backend.md)
            - [🟢 ConcreteEnv](storage/db/abstraction/concrete_env.md)
            - [🟢 Trait](storage/db/abstraction/trait.md)
        - [🟢 Syncing](storage/db/syncing.md)
        - [🟢 (De)serialization](storage/db/serde.md)
        - [🟢 Known issues and tradeoffs](storage/db/issues/intro.md)
            - [🟢 Abstracting backends](storage/db/issues/traits.md)
            - [🟢 Hot-swap](storage/db/issues/hot-swap.md)
            - [🟢 Unaligned bytes](storage/db/issues/unaligned.md)
            - [🟢 Endianness](storage/db/issues/endian.md)
            - [🟢 Multimap](storage/db/issues/multimap.md)
    - [🟢 Common behavior](storage/common/intro.md)
        - [🟢 Types](storage/common/types.md)
        - [🟢 `ops`](storage/common/ops.md)
        - [🟢 `tower::Service`](storage/common/service/intro.md)
            - [🟢 Initialization](storage/common/service/initialization.md)
            - [🟢 Requests](storage/common/service/requests.md)
            - [🟢 Responses](storage/common/service/responses.md)
            - [🟢 Thread model](storage/common/service/thread-model.md)
            - [🟢 Shutdown](storage/common/service/shutdown.md)
    - [🟢 Blockchain](storage/blockchain/intro.md)
        - [🟢 Schema](storage/blockchain/schema/intro.md)
            - [🟢 Tables](storage/blockchain/schema/tables.md)
            - [🟢 Multimap tables](storage/blockchain/schema/multimap.md)
    - [⚪️ Transaction pool](storage/txpool/intro.md)
    - [⚪️ Pruning](storage/pruning/intro.md)

---

- [🟢 RPC](rpc/intro.md)
    - [🟡 JSON-RPC 2.0](rpc/json-rpc.md)
    - [🟢 The types](rpc/types/intro.md)
        - [🟢 Misc types](rpc/types/misc-types.md)
        - [🟢 Base RPC types](rpc/types/base-types.md)
        - [🟢 The type generator macro](rpc/types/macro.md)
        - [🟢 Metadata](rpc/types/metadata.md)
        - [🟡 (De)serialization](rpc/types/deserialization.md)
    - [🟢 The interface](rpc/interface.md)
    - [🔴 The handler](rpc/handler/intro.md)
    - [🔴 The server](rpc/server/intro.md)
    - [🟢 Differences with `monerod`](rpc/differences/intro.md)
        - [🟢 JSON field ordering](rpc/differences/json-field-ordering.md)
        - [🟢 JSON formatting](rpc/differences/json-formatting.md)
        - [🟢 JSON strictness](rpc/differences/json-strictness.md)
        - [🟡 JSON-RPC strictness](rpc/differences/json-rpc-strictness.md)
        - [🟡 HTTP methods](rpc/differences/http-methods.md)
        - [🟡 RPC payment](rpc/differences/rpc-payment.md)
        - [🟢 Custom strings](rpc/differences/custom-strings.md)
        - [🔴 Unsupported RPC calls](rpc/differences/unsupported-rpc-calls.md)
        - [🔴 RPC calls with different behavior](rpc/differences/rpc-calls-with-different-behavior.md)

---

- [⚪️ ZMQ](zmq/intro.md)
    - [⚪️ TODO](zmq/todo.md)

---

- [⚪️ Consensus](consensus/intro.md)
    - [⚪️ Verifier](consensus/verifier.md)
    - [⚪️ TODO](consensus/todo.md)

---

- [⚪️ Networking](networking/intro.md)
    - [⚪️ P2P](networking/p2p.md)
    - [⚪️ Dandelion++](networking/dandelion.md)
    - [⚪️ Proxy](networking/proxy.md)
    - [⚪️ Tor](networking/tor.md)
    - [⚪️ i2p](networking/i2p.md)
    - [⚪️ IPv4/IPv6](networking/ipv4-ipv6.md)

---

- [🔴 Instrumentation](instrumentation/intro.md)
    - [⚪️ Logging](instrumentation/logging.md)
    - [⚪️ Data collection](instrumentation/data-collection.md)

---

- [⚪️ Binary](binary/intro.md)
    - [⚪️ CLI](binary/cli.md)
    - [⚪️ Config](binary/config.md)
    - [⚪️ Logging](binary/logging.md)

---

- [⚪️ Resources](resources/intro.md)
    - [⚪️ File system](resources/fs/intro.md)
        - [🟡 Index of PATHs](resources/fs/paths.md)
    - [⚪️ Sockets](resources/sockets/index.md)
        - [🔴 Index of ports](resources/sockets/ports.md)
    - [⚪️ Memory](resources/memory.md)
    - [🟡 Concurrency and parallelism](resources/cap/intro.md)
        - [⚪️ Map](resources/cap/map.md)
        - [⚪️ The RPC server](resources/cap/the-rpc-server.md)
        - [⚪️ The database](resources/cap/the-database.md)
        - [⚪️ The block downloader](resources/cap/the-block-downloader.md)
        - [⚪️ The verifier](resources/cap/the-verifier.md)
        - [⚪️ Thread exit](resources/cap/thread-exit.md)
        - [🔴 Index of threads](resources/cap/threads.md)

---

- [⚪️ External Monero libraries](external-monero-libraries/intro.md)
    - [⚪️ Cryptonight](external-monero-libraries/cryptonight.md)
    - [🔴 RandomX](external-monero-libraries/randomx.md)
    - [🔴 monero_oxide](external-monero-libraries/monero-oxide.md)

---

- [🟢 Benchmarking](benchmarking/intro.md)
    - [🟢 Criterion](benchmarking/criterion/intro.md)
        - [🟢 Creating](benchmarking/criterion/creating.md)
        - [🟢 Running](benchmarking/criterion/running.md)
    - [🟢 `cuprate-benchmark`](benchmarking/cuprate/intro.md)
        - [🟢 Creating](benchmarking/cuprate/creating.md)
        - [🟢 Running](benchmarking/cuprate/running.md)

---

- [⚪️ Testing](testing/intro.md)
    - [⚪️ Monero data](testing/monero-data.md)
    - [⚪️ RPC client](testing/rpc-client.md)
    - [⚪️ Spawning `monerod`](testing/spawning-monerod.md)
- [⚪️ Known issues and tradeoffs](known-issues-and-tradeoffs/intro.md)
    - [⚪️ Networking](known-issues-and-tradeoffs/networking.md)
    - [⚪️ RPC](known-issues-and-tradeoffs/rpc.md)
    - [⚪️ Storage](known-issues-and-tradeoffs/storage.md)

---

- [🟢 Monero oddities](oddities/intro.md)
    - [🟡 Little-endian IPv4 addresses](oddities/le-ipv4.md)

---

- [⚪️ Appendix](appendix/intro.md)
    - [🟢 Crates](appendix/crates.md)
    - [🔴 Contributing](appendix/contributing.md)
    - [🔴 Build targets](appendix/build-targets.md)
    - [🔴 Protocol book](appendix/protocol-book.md)
    - [⚪️ User book](appendix/user-book.md)