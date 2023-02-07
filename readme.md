# Cuprate, an upcoming experimental, modern & secure monero node. Written in Rust

(there is nothing working at the moment, but stay tuned)

### Introduction

Monero is actively used across the world and gains more and more users through the years. Unfortunately, it is clearly targeted by numerous adversaries with different set of ressources. As of now we are targeted by media disinformation, other cryptocurrency communities & even governements. The life of the project depends now on our efforts to make Monero usable by anyone while also remaining resilient against an attack.

The current state of Monero developpement is encouraging. Farcaster & COMIT have successfuly developped XMR<>BTC Atomic Swap, ETH<>XMR bridge is on the way, and other are draft. Not only it is a great addition to the UX but it also give monero resilience by developping way for people to access it in case of ban. Seraphis is on the way to make Monero even more private. As of consensus security, p2pool is now mature and actively used.

We can clearly applaud all the efforts that have been done. But there is still works to do. For example, we still don't have developped traffic obfuscation to bypass DPI. Without, it'll be easy for governements to dramatically reduce the access to the monero network, and by that reduce the number of people that could escape the financial system.

**Cuprate** is an ongoing effort to release an alternative implementation of the Monero Node with new features. It is developped in Rust and therefore enjoy from many advantages in term of security and stability. It will also help developping new features with high-level, safe and maintained librairies available in the rust ecosystem. 

Releasing an alternative node will reinforce the Monero Network if a security vulnerability is discovered in the current node maintained by the monero-core team. It will also encourage (i hope) more open-source developers to contribute to the project. 

### Status

The project is actualy handle by single guy that never really started a big project of this scale nor understand completely the monero codebase. That you like it or not, He have a lot of time ahead of him now and will continue to work on this.

I'm working on rewriting the blockchain_db part atm.

### Contributions

Any help on rewriting other parts of the node while also proposing improvements and respecting the goal of the project (which means try to be async and no weird api, or at least talk about it) is appreciated. 

I encourage anyone to review the work being done, discuss about it or propose agressive optimizations (at architectural level if needed, or even micro-optimizations in 'monolithic components').

### Code & Repo

For the moment i try to organize the repo like the official one.

### Improvements & Features

- Asynchronous Design. Networking will be more performant and versatile by using Tokio. It will permit the simultaneous connections of a hundred peers. Handling different protocols by peers will be possible in a modular manner.
- Traffic Obfuscation. Different protocol to bypass DPI will be available, such as [Levin protocol](https://github.com/vtnerd/monero/blob/docs_p2p_e2e/docs/LEVIN_PROTOCOL.md#encryption) (TLS based, see https://github.com/monero-project/monero/issues/7078) and QUIC [like Syncthing have done](https://github.com/syncthing/syncthing/pull/5737), but with offset and timing mitigations. Unless the monero-core team decide to implement these protocols, they'll only by available between cuprate peers.
- Blockchain Storage. LMDB is replaced by RocksDB, a high-performance database designed for SSD, already used by the Parity ethereum's rust client. HSE is also going to be implemented, as a more dsitributed and scalable alternative.
- Sandboxing & System. There will be maintained SELinux/Apparmor policy for this node. It will internally use seccomp/Landlock to limit syscalls being used in order to improve isolation of the node with rest of the OS & Wallet Software.
- API. A GraphQL API will be available to communicate with the node.
- Terminal Interface. More accessible interface based on the excellent [tui](https://lib.rs/crates/tui) library. There will be Geolocation of peers on map, VPN Detection, Ressource usages, statistics etc... 
- Tor connections. arti_client library will be embedded to make possible connections to tor peers without a system daemon or proxy (for the moment arti_client can't handle onion services, but it'll certainly in the near future).

### Regressions

- No integrated miner planned
- LMDB support removed (unless someone else want to work on it, or I've time). Which means that the blockchain synced by monerod is incompatible with cuprate
