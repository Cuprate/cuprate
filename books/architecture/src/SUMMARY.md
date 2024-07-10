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
	- [⚪️ monero_serai](formats-protocols-types/monero-serai.md)
	- [⚪️ cuprate_types](formats-protocols-types/cuprate-types.md)
	- [⚪️ cuprate_helper](formats-protocols-types/cuprate-helper.md)
	- [⚪️ Epee](formats-protocols-types/epee.md)
	- [⚪️ Levin](formats-protocols-types/levin.md)

---

- [⚪️ Storage](storage/intro.md)
	- [⚪️ Database abstraction](storage/database-abstraction.md)
	- [⚪️ Blockchain](storage/blockchain.md)
	- [⚪️ Transaction pool](storage/transaction-pool.md)
	- [⚪️ Pruning](storage/pruning.md)

---

- [🔴 RPC](rpc/intro.md)
	- [⚪️ Types](rpc/types/intro.md)
		- [⚪️ JSON](rpc/types/json.md)
		- [⚪️ Binary](rpc/types/binary.md)
		- [⚪️ Other](rpc/types/other.md)
	- [⚪️ Interface](rpc/interface.md)
	- [⚪️ Router](rpc/router.md)
	- [⚪️ Handler](rpc/handler.md)
	- [⚪️ Methods](rpc/methods/intro.md)

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

- [⚪️ Resource model](resource-model/intro.md)
	- [⚪️ File system](resource-model/file-system.md)
	- [⚪️ Sockets](resource-model/sockets.md)
	- [⚪️ Memory](resource-model/memory.md)
	- [🟡 Concurrency and parallelism](resource-model/concurrency-and-parallelism/intro.md)
		- [⚪️ Map](resource-model/concurrency-and-parallelism/map.md)
		- [⚪️ The RPC server](resource-model/concurrency-and-parallelism/the-rpc-server.md)
		- [⚪️ The database](resource-model/concurrency-and-parallelism/the-database.md)
		- [⚪️ The block downloader](resource-model/concurrency-and-parallelism/the-block-downloader.md)
		- [⚪️ The verifier](resource-model/concurrency-and-parallelism/the-verifier.md)
		- [⚪️ Thread exit](resource-model/concurrency-and-parallelism/thread-exit.md)

---

- [⚪️ External Monero libraries](external-monero-libraries/intro.md)
	- [⚪️ Cryptonight](external-monero-libraries/cryptonight.md)
	- [🔴 RandomX](external-monero-libraries/randomx.md)
	- [🔴 monero_serai](external-monero-libraries/monero_serai.md)

---

- [⚪️ Benchmarking](benchmarking/intro.md)
	- [⚪️ Criterion](benchmarking/criterion.md)
	- [⚪️ Harness](benchmarking/harness.md)
- [⚪️ Testing](testing/intro.md)
	- [⚪️ Monero data](testing/monero-data.md)
	- [⚪️ RPC client](testing/rpc-client.md)
	- [⚪️ Spawning `monerod`](testing/spawning-monerod.md)
- [⚪️ Known issues and tradeoffs](known-issues-and-tradeoffs/intro.md)
	- [⚪️ Networking](known-issues-and-tradeoffs/networking.md)
	- [⚪️ RPC](known-issues-and-tradeoffs/rpc.md)
	- [⚪️ Storage](known-issues-and-tradeoffs/storage.md)

---

- [⚪️ Appendix](appendix/intro.md)
	- [🔴 Contributing](appendix/contributing.md)
	- [🔴 Crate documentation](appendix/crate-documentation.md)
	- [🔴 Build targets](appendix/build-targets.md)
	- [🔴 Protocol book](appendix/protocol-book.md)
	- [⚪️ User book](appendix/user-book.md)