<div align="center">
<img src="images/CuprateLogo.svg" width="50%"/>

[Cuprate](https://github.com/Cuprate/cuprate) is an alternative and independent [Monero](https://getmonero.org) node implementation that is focused on being fast, user-friendly, and backwards compatible with [`monerod`](https://github.com/monero-project/monero).

This project is currently a work-in-progress; the `cuprated` node can be ran by users although it is not yet ready for production. This book contains brief sections documenting `cuprated` usage, however, be aware that it is **incomplete** and missing sections.

To get started, see: [`Getting started`](./getting-started/intro.md).

</div>

---

# FAQ
Frequently asked questions about Cuprate.

## Who?
Cuprate was started by [SyntheticBird45](https://github.com/SyntheticBird45) in [early 2023](https://github.com/Cuprate/cuprate/commit/2c7cb27548c727550ce4684cb31d0eafcf852c8e) and was later joined by [boog900](https://github.com/boog900), [hinto-janai](https://github.com/hinto-janai), and [other contributors](https://github.com/Cuprate/cuprate/graphs/contributors).

A few Cuprate contributors are funded by Monero's [Community Crowdfunding System](https://ccs.getmonero.org) to work on Cuprate and occasionally `monerod`.

## What is `cuprated`?
`monerod` is the [daemon](https://en.wikipedia.org/wiki/Daemon_(computing)) of the Monero project, the Monero node.

`cuprated` is the daemon of the Cuprate project, the Cuprate node.

Both operate on the same network, the Monero network, and are responsible for roughly the same tasks.

For more information on the role of alternative node implementations, see:
- <https://clientdiversity.org>
- <https://bchfaq.com/knowledge-base/what-are-the-full-node-implementations-for-bitcoin-cash>
- <https://zfnd.org/zebra-stable-release>

## Does `cuprated` replace `monerod`?
No.

`cuprated` cannot currently replace `monerod` in production environments. With that said, there will be practical performance benefits for users to use `cuprated` eventually.

## Is it safe to run `cuprated`?
**⚠️ This project is still in development; do NOT use `cuprated` for any serious purposes ⚠️**

`cuprated` is fine to run for non-serious purposes and has a similar attack surface to other network connected services.

See [`Resources`](./resources/intro.md) for information on what system resources `cuprated` will use.

## What files does `cuprated` create?
See [`Resources/Disk`](./resources/disk.md).

## What can `cuprated` currently do?
Cuprate's node (`cuprated`) can currently:

- Sync the blockchain and transaction pool
- Broadcast and relay blocks and transactions
- Help other peers sync their blockchain

## How fast does `cuprated` sync?
The current full verification sync timings are around ~7.5x faster than `monerod`.

In real terms, 16 hour full verification syncs and 4 hour fast-sync syncs have been reported on consumer grade hardware. On faster hardware (14 threads, 10Gbps networking), sub 2 hour fast-syncs have been reported.

Various testing results can be found [here](https://github.com/Cuprate/cuprate/issues/195).

## How to see status of `cuprated`?
In the terminal running `cuprated`, type `status`.

Use the `help` command to see the full list of commands.

## How to tell `cuprated` is fully synced?
`cuprated` emits a message when it is fully synced: `Synchronised with the network`.

It also logs its block height status when syncing, for example:

```text
2025-05-01T22:17:10.270002Z  INFO incoming_block{height=3402413 txs=66}: Successfully added block hash="e93464a7feea9b472dd734e61574e295f4b8f809c48ff78ef76d12111992ada7"
```

## How big is the database?
As of May 1st 2025, `cuprated`'s database is ~270GB in size.

`monerod`'s database is ~225GB in size.

This is 1.2x larger.

This is planned to be improved in the future.

## Is the database compatible with `monerod`?
No.

The database `cuprated` generates and uses cannot directly be used by `monerod` and vice-versa. Supporting this is possible but there are no current plans to do so.

## Can I connect a wallet to `cuprated`?
Not yet.

Wallets require the [daemon RPC API](https://docs.getmonero.org/rpc-library/monerod-rpc). This is actively being worked on to be backwards compatible with `monerod`, although it is not yet available.

See the [RPC section](rpc.md) for more information.

## Can `cuprated` be used with an anonymity network like Tor?
Not yet (directly).

Tor is planned to be integrated into `cuprated` via [`arti`](https://arti.torproject.org), although this is not yet available.

## `cuprated` won't start because of a "killswitch", why?
The current alpha builds of `cuprated` contain killswitches that activate 1 week after the _next_ release is out. If the killswitch activates, you must upgrade to the [latest release](https://github.com/Cuprate/cuprate/releases/latest).

The reasoning for why this exists can be found here: <https://github.com/Cuprate/cuprate/pull/365>.

## What is the release schedule?
New versions of `cuprated` are planned to release every 4 weeks.

See [this GitHub issue](https://github.com/Cuprate/cuprate/issues/374) for more details.

## What is the versioning scheme?
`cuprated` is currently in alpha (`0.0.x`).

After sufficient testing and development, `cuprated` will enter beta (`0.x.y`) then stable (`x.y.z`) releases.

See [this GitHub issue](https://github.com/Cuprate/cuprate/issues/374) for more details.

## What is the current progress?
See [this Reddit thread](https://www.reddit.com/r/Monero/comments/1ij2sw6/cuprate_2024_progress_report) for a brief report on Cuprate's progress throughout 2024.

Things are always changing so feel free to join our [Matrix channel](https://matrix.to/#/#cuprate:monero.social) and ask questions.

## What is the current roadmap?
See [this GitHub issue](https://github.com/Cuprate/cuprate/issues/376) for Cuprate's rough 2025 roadmap.
