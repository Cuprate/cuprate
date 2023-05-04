<div align=center>
  <img src="misc/logo/CuprateLogo.svg" alt="Cuprate" height="280"/>
</div>
<h1 align=center> Cuprate </h1>

<h4 align=center> an upcoming experimental, modern & secure monero node. Written in Rust </h4>

&nbsp;
<p align="center">
  <a href="#introduction">Introduction</a> |
  <a href="#status">Status</a> |
  <a href="#improvements--features">Features</a> |
  <a href="#contributions">Contributions</a> |
  <a href="#contact">Contact</a> |
  <a href="#donations">Donations</a>
</p>
  
> **Warning** nothing is working at the moment. But stay tuned for adventures

<h3>Introduction</h3>
<details>
  
  <summary>Why?</summary>
  
Monero is actively used across the world and gains more and more users through the years. Unfortunately, it is clearly targeted by numerous adversaries with different set of resources. As of now we are targeted by media disinformation, other cryptocurrency communities & even governements. The life of the project depends now on our efforts to make Monero usable by anyone while also remaining resilient against an attack.

The current state of Monero development is encouraging. Farcaster & COMIT have successfully developed XMR<>BTC Atomic Swap, ETH<>XMR bridge is on the way, and other are draft. Not only is it a great addition to the UX but it also give Monero resilience by developing ways for people to access it if it were to be banned. Seraphis is on the way to make Monero even more private and p2pool is now mature and actively used.

We can clearly applaud all the efforts that have been done. But there is still works to do. For example, we still don't have a way to use traffic obfuscation to bypass DPI. Without, it'll be easy for governements to dramatically reduce access to the Monero network, and by that reduce the number of people that can escape financial surveillance.
</details>
  
**Cuprate** is an ongoing effort to release an alternative implementation of monerod (the only Monero node) with new features. It is developed in Rust and therefore enjoys many advantages in terms of security and stability. It will also help developing new features with high-level, safe and maintained librairies available in the rust ecosystem. 

Releasing an alternative node will reinforce the Monero Network if a security vulnerability is discovered in the current node maintained by the Monero-core team. It will also encourage (I hope) more open-source developers to contribute to improving Monero. 

  
### Status

Status of current parts being work on can be found in the [pull request section](https://github.com/SyntheticBird45/cuprate/pulls).

@boog900 has delivered the net code and is working on ringCT & P2P.

@SyntheticBird45 is working on the database.

 ## Improvements & Features
  
  <details> <summary>Traffic Obfuscation</summary> </br> Different protocol to bypass DPI will be available, such as with a proposal for <a href="https://github.com/vtnerd/monero/blob/docs_p2p_e2e/docs/LEVIN_PROTOCOL.md#encryption">Levin protocol</a> (TLS based, see https://github.com/monero-project/monero/issues/7078) and QUIC <a href="https://github.com/syncthing/syncthing/pull/5737">like Syncthing have done</a>, but with offset and timing mitigations. Unless the monero-core team decide to implement these protocols, they'll only by available between cuprate peers.</details>
  
  <details> <summary>Blockchain Storage</summary> </br>LMDB is replaced by MDBX, a spiritual successor of LMDB with insane performance, already used by the reth Ethereum's rust client. HSE (Heterogeneous Storage Engine for Micron, optimized for SSD & random writes & reads) is also going to be implemented, as a more dsitributed and scalable alternative. </details>
  
<details> <summary>Sandboxing & System</summary> </br> 
- For Linux : There will be maintained SELinux/Apparmor policy for this node for major linux distributions. It will internally use seccomp to limit syscalls being used. Landlock is also going to be setup in order to improve isolation of the node with rest of the OS.
</br>- For Windows : It still need some research but we could use capability primitives & WinAPI to limit access to certain system functions.
</br>- For macOS : There is unfortunately no library to setup some isolation, as Apple seems to have deprecated Seatbelt.
</details>
  
<details> <summary>RPC</summary> </br> ZeroMQ as well as gRPC will be available to communicate with the node. Powered by tonic library from Tokio</details>
  
<details> <summary>Terminal Interface</summary> </br> More accessible interface based on the excellent <a href="https://lib.rs/crates/tui">tui</a> library. There will be Geolocation of peers on map, VPN Detection, Ressource usages, statistics etc... </details>
  
<details> <summary>Tor connections</summary> </br> arti_client library will be embedded to make possible connections to tor peers without a system daemon or proxy (for the moment arti_client can't handle onion services, but it'll certainly in the near future). i2p support is not planned at the moment</details>

### Regressions

- No integrated miner planned
- LMDB support removed. Which means that the blockchain synced by monerod is incompatible with cuprate.
- [Some](https://github.com/monero-project/monero/blob/c5d10a4ac43941fe7f234d487f6dd54996a9aa33/src/wallet/wallet2.cpp#L3930) [funny](https://github.com/monero-project/monero/blob/c5d10a4ac43941fe7f234d487f6dd54996a9aa33/src/common/dns_utils.cpp#L134) [messages](https://github.com/monero-project/monero/blob/c5d10a4ac43941fe7f234d487f6dd54996a9aa33/src/common/util.cpp#L602) in the original codebase will be lost.

## Contributions

Any help is appreciated. If you want to help but don't know where to start, you can take a look at the [issues section](https://github.com/SyntheticBird45/cuprate/issues) 

We encourage anyone to review the work being done, discuss about it or propose agressive optimizations (at architectural level if needed, or even micro-optimizations in 'monolithic components').

For non-developers people, you can also propose ideas in the [discussion section](https://github.com/SyntheticBird45/cuprate/discussions). The sooner we hear about your ideas, the better the chance are we implement them into Cuprate.

## Code & Repository

No unsafe code is permitted in the project, and the codebase will rarely contain `.expect()` or `.unwrap()`, we discourage the use
of these, as it implies that all patterns are correctly handled. This way the node should never suddenly crash.

The repository is a cargo workspace. You will find every corresponding codebase in their crates folders. These crates are librairies and the main crates used to compile the node can be found in src/

### Security measures
<details><summary>Exploit Mitigations</summary></br>
As specified in the cargo.toml, cuprate releases are compiled with several rustflags & cargoflags to improve binary security:

</br><details><summary>Debug informations are cleared & symbols are stripped.</summary></br>
Even if the source code is available, sometimes you can find bugs in a program by looking at the metadata left by the compiler at assembly level. Stipping these metadata help mitigating some vulnerability analysis. Of course someone could recompile it without these flags. The same way some people could tunes some compilation flags if they decide to compile it by themselves. But it is likely to change call hierarchy and other data that could ruin a potential vulnerability. </details> 
<details><summary>In case of panic, the node immediately abort.</summary></br>
This isn't to be annoying. This is security measure. Most of the times, exploits are designed to use vulnerabilities that don't crash the targeted process but is definitely modifying it's behavior. In such case, where a function doesn't end properly, the sanest way to deal with it, is to stop all the threads immediately. If you don't, you risk to trigger a vulnerability or execute potential malware code.</details>
<details><summary>Forward-Edge <a href="https://en.wikipedia.org/wiki/Control-flow_integrity">Control-Flow Integrity</a></summary></br>
This is an exploit mitigation that can be enable in GCC & LLVM to fight against <a href="https://en.wikipedia.org/wiki/Return-oriented_programming">Return-oriented programming</a>. This isn't enabled by default in Rust, because to make a rop chain you need first to corrupt a pointer (which is *normally* impossible), but since we focus on security it's worth enabling it. CFI is basically a combination of added code to verify if the program is respecting it's functions call hierarchy or if its calling part of the binary it shouldn't do.</details>
<details><summary>Compiling as a <a href="https://en.wikipedia.org/wiki/Position-independent_code">Position Indepent Executable</a></summary></br>
This is a type of executable that permit its machine code to be executed regardless of it's address layout by dynamically playing with its global offset table. This way, functions called each others based on offset instead of absolute address. It permit better security because at each execution the address being used in the execution stack change. This is great to make a potential exploit unreliable on targeted machines.</details>
<details><summary>Using stack-protector=all</summary></br> Stack protector are a set of strategy used by LLVM & GCC to detect buffer overflow & general memory corruption. By setting it as all, we tell LLVM to enable this strategy to all functions. Making it as difficult as possible to corrupt memory without being detected (=abort).
</details>
</details>

### Dependencies

<details>
<summary>Dependencies</summary>

| Dependencies |   Reason    |
|----------------|-----------|
| monero-rs        | Used to define monero's type and serialize/deserialize data. 
| serde                  | serialize/deserialize support. 
| thiserror            | used to Derive(Error) in the codebase.
| libmdbx        | safe wrapper for mdbx implementation.

</details>

### License

Cuprate is licensed under AGPL but some of the crates that make up Cuprate are licensed under MIT. Each crate will have it's license in its `Cargo.toml` with a corresponding `LICENSE` file.


## Contact

If you wish to contact contributors privately, you can import our pgp keys from the misc/gpg_keys folder. You can also contact us directly on Matrix (see contributors list in `Cargo.toml`). If you wish to follow the development closely or just talk to us more casually, you can join our [Revolt server](https://rvlt.gg/DZtCpfW1).</br>

## Donations

We're working on Cuprate in our free time, it take times & effort to make progress. We greatly appreciate your support, it really means a lot and encourage us to continue. If you wanna buy us a coffee (or tea for some of us) you can send your kindness at this address : </br><p align=center><strong>82rrTEtqbEa7GJkk7WeRXn67wC3acqG5mc7k6ce1b37jTdv5uM15gJa3vw7s4fDuA31BEufjBj2DzZUb42UqBaP23APEujL</strong></p>

<div align=center><img src="https://raw.githubusercontent.com/Cuprate/cuprate/main/qr-code.png"></img></div>
