<div align="center">
<img src="images/CuprateLogo.svg" width="50%"/>

[Cuprate](https://github.com/Cuprate/cuprate) is a [Monero](https://getmonero.org) node implementation that is focused on being fast, user-friendly, and backwards compatible with [`monerod`](https://github.com/monero-project/monero).

This project is currently a work-in-progress; the `cuprated` node can be ran by users although it is not yet ready for production. This book contains brief sections documenting `cuprated` usage, however, be aware that it is **incomplete** and missing sections.

To get started, see: [`Getting started`](./getting-started/intro.md).

</div>

---

# FAQ
Frequently asked questions about Cuprate.

## Who?
Cuprate was started by [SyntheticBird45](https://github.com/SyntheticBird45) in [early 2023](https://github.com/Cuprate/cuprate/commit/2c7cb27548c727550ce4684cb31d0eafcf852c8e) and was later joined by [boog900](https://github.com/boog900), [hinto-janai](https://github.com/hinto-janai), and [other contributors](https://github.com/Cuprate/cuprate/graphs/contributors).

## Why?
TODO

- clearing technical debt
- modern programming language improvements
- modular libraries
- node decentralization

## Is it safe to run `cuprated`?
**⚠️ This project is still in development; do NOT use `cuprated` for any serious purposes ⚠️**

With that said, `cuprated` is fine to run currently for casual purposes and has a similar attack surface to other network connected services.

See [`Resources`](./resources/intro.md) for information on what system resources `cuprated` will use.

## Where are files located?
See [`Resources/Disk`](./resources/disk.md).

## What can `cuprated` currently do?
Cuprate's node (`cuprated`) can currently:

- Sync the blockchain and transaction pool
- Broadcast and relay blocks and transactions
- Help other peers sync their blockchain

## How fast does `cuprated` sync?
The current full verification sync timings are around 1.4x~3x faster than `monerod`.

In real terms, 20 hour full verification syncs and 4 hour fast-sync syncs have been reported on consumer grade hardware. Various testing results can be found [here](https://github.com/Cuprate/cuprate/issues/195).

## How to tell `cuprated` is fully synced?
`cuprated` does not currently emit a message indicating it is finished syncing, although it does log its block height status when syncing, for example:

```text
2025-03-01T22:15:52.516944Z  INFO incoming_block_batch{start_height=3362022 len=29}: Successfully added block batch
```

- `start_height` is the height `cuprated` was previously at
- `len` is how many blocks have been added to the blockchain

`start_height` can be compared to a block height from `monerod`
or a block explorer to see if `cuprated` is near synced.

## How big is the database?
As of March 4th 2025, `cuprated`'s database is ~240GB in size.

For reference, `monerod`'s database is ~200GB in size.

This is planned to be improved in the future after other big features have been added.

## Is the database compatible with `monerod`?
No.

The database `cuprated` generates and uses cannot directly be used by `monerod` and vice-versa. Supporting this is possible but there are no current plans to do so.

## Can I connect a wallet to `cuprated`?
Not yet.

Wallets require the [daemon RPC API](https://docs.getmonero.org/rpc-library/monerod-rpc). This is actively being worked on to be backwards compatible with `monerod`, although this is not yet available.

## Can `cuprated` be used with an anonymity network like Tor?
Not yet (directly).

Tor is planned to be integrated into `cuprated` via [`arti`](https://arti.torproject.org), although this is not yet available.

In the meanwhile, solutions like [`torsocks`](https://github.com/dgoulet/torsocks)
can redirect any program's networking through Tor, including `cuprated`.
Note that this will slow down syncing speeds heavily.

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