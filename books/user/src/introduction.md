<div align="center">
<img src="images/CuprateLogo.svg" width="50%"/>

[Cuprate](https://github.com/Cuprate/cuprate) is a [Monero](https://getmonero.org) node implementation that is focused on being fast, user-friendly, and backwards compatible with [`monerod`](https://github.com/monero-project/monero).

This project is currently a work-in-progress; the `cuprated` node can be ran by users although it is not yet ready for production. This book contains brief sections documenting `cuprated` usage, however, be aware that it is **incomplete** and missing some sections.

To get started, see: [`Getting started`](./getting-started/intro.md).

To learn more, feel free to join our [Matrix channel](https://matrix.to/#/#cuprate:monero.social).

</div>

---

# FAQ
Frequently asked questions about Cuprate.

## Why?
TODO

## Is it safe to run `cuprated`?
**⚠️ This project is still in development; do NOT use `cuprated` for any serious purposes ⚠️**

With that said, `cuprated` is currently fine to run for hobbyist purposes and has a similar attack surface to other network connected service.

See [`Resources`] for information on what resources `cuprated` will use (ports, directories, etc).

## What can `cuprated` currently do?
Cuprate's node (`cuprated`) can currently:

- Sync the blockchain and transaction pool
- Broadcast and relay blocks and transactions
- Help other peers sync their blockchain

## How fast does `cuprated` sync?
The current full verification sync timings are around 1.4x~3x faster than `monerod`.

In real terms, 20 hour full verification syncs and 4 hour fast-sync syncs have been reported on consumer grade hardware. Various testing results can be found [here](https://github.com/Cuprate/cuprate/issues/195).

## How to tell `cuprated` is fully synced?
TODO

## Where are files located?
See [`File structure`](./file-structure.md) for information on what files `cuprated` generates and where they are located.

## How big is the database?
As of March 4th 2025, `cuprated`'s database is ~240GB in size.

For reference, `monerod`'s database is ~200GB in size.

## Is the database compatible with `monerod`?
No.

The database `cuprated` generates and uses cannot be used by `monerod` and vice-versa. Supporting this is possible but there are no current plans to do so.

## Can I connect a wallet to `cuprated`?
Not yet.

Wallets require the [daemon RPC API](https://docs.getmonero.org/rpc-library/monerod-rpc). This is actively being worked on to be backwards compatible with `monerod`, although it is not yet complete.

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

Things are always changing so feel free to join our Matrix channel and ask questions.

## What is the current roadmap?
See [this GitHub issue](https://github.com/Cuprate/cuprate/issues/376) for Cuprate's rough 2025 roadmap.