# Blocks

## Introduction

This chapter contains all the rules that apply to a block. Miner transactions are included in this section as the rules that apply to them
are different to normal transactions.

## Index

1. [Block Rules](./blocks.md#block-rules)
2. [Difficulty](./blocks/difficulty.md)
3. [Weights](./blocks/weights.md)
4. [Block Reward](./blocks/reward.md)
5. [Miner Transaction](./blocks/miner_tx.md)

## Block Rules

### Block Weight And Size

The `block blob` must not be bigger than (2 * the [effective median weight](./blocks/weights.md#effective-median-weight) + 100)[^block-size-check].

The [block weight](./blocks/weights.md#block-weights) must not be more than 2 *
[the median weight for coinbase checks](./blocks/weights.md#median-weight-for-coinbase-checks)[^block-weight-limit].

### Amount Of Transactions

The amount of transactions in a block (including the miner transaction) must be less than `0x10000000`[^max-amount-of-txs].

### No Duplicate Transactions

There must be no duplicate transactions in the block or the blockchain[^no-duplicate-txs].

### Key Images

There must be no duplicate key images in the block[^no-duplicate-ki], or the whole chain.

### Previous ID

The blocks `prev_id` must equal the `block hash` of the last block[^prev_id].

### POW Function

The proof of work function used depends on the hard-fork[^pow-func]:

| hard-fork  | POW function   |
| ---------- | -------------- |
| 1 to 6     | CryptoNight v0 |
| 7          | CryptoNight v1 |
| 8 to 9     | CryptoNight v2 |
| 10 to 11   | CryptoNight R  |
| 12 onwards | RandomX        |

> For block 202612 always return the same POW hash, no matter the network[^202612-pow-hash].
>
> POW hash: `84f64766475d51837ac9efbef1926486e58563c95a19fef4aec3254f03000000`

### Checking POW Hash

See [checking POW in the difficulty chapter](./blocks/difficulty.md#checking-a-blocks-proof-of-work).

### RandomX Seed

The RandomX seed, which is used to set up the dataset, is a previous block hash in the blockchain.

The seed height is 0 if the current height is below or equal to \\( 2048 + 64 \\) otherwise is got by:

\\( seedHeight = (height - 64 - 1) \land \lnot(2048 - 1) \\)

with \\( \land \\) being a bit-and and \\( \lnot \\) being a bit-not.

You then get the block hash at `seedHeight` which is then the RandomX seed.[^rx-seed]

### Version And Vote

The block's major version must equal the current hard-fork and the vote must be equal to or greater than the current hard-fork[^version-vote].

> Vote is not always the same as the minor version, see [here](./hardforks.md#blocks-version-and-vote).

### Timestamp

The block's timestamp must not be more than the current UNIX time + 2 hours[^timestamp-upper-limit] and the timestamp must not be less than
the median timestamp over the last 60 blocks[^timestamp-lower-limit], if there are less than 60 blocks in the chain then the timestamp is always valid.

---

[^block-size-check]: <https://github.com/monero-project/monero/blob/67d190ce7c33602b6a3b804f633ee1ddb7fbb4a1/src/cryptonote_core/cryptonote_core.cpp#L1684>

[^block-weight-limit]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L1418-L1428> && <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_basic/cryptonote_basic_impl.cpp#L107>

[^max-amount-of-txs]: <https://github.com/monero-project/monero/blob/67d190ce7c33602b6a3b804f633ee1ddb7fbb4a1/src/crypto/tree-hash.c#L55>

[^no-duplicate-txs]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L5267> && <https://github.com/monero-project/monero/blob/67d190ce7c33602b6a3b804f633ee1ddb7fbb4a1/src/cryptonote_core/blockchain.cpp#L4319>

[^no-duplicate-ki]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L5281>

[^prev_id]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L4150>

[^pow-func]: <https://github.com/monero-project/monero/blob/67d190ce7c33602b6a3b804f633ee1ddb7fbb4a1/src/cryptonote_core/cryptonote_tx_utils.cpp#L689-L704>

[^202612-pow-hash]: <https://github.com/monero-project/monero/blob/67d190ce7c33602b6a3b804f633ee1ddb7fbb4a1/src/cryptonote_core/cryptonote_tx_utils.cpp#L683>

[^rx-seed]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/crypto/rx-slow-hash.c#L179-L186>

[^version-vote]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_basic/hardfork.cpp#L109>

[^timestamp-upper-limit]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L4064>

[^timestamp-lower-limit]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L4045>
