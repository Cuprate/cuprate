# Transaction Inputs

## Introduction

These rules apply to transaction inputs, excluding miner transactions.

## Index

1. [Necessary Functions/Definitions](#functionsdefinitions)
2. [Rules](#rules)

## Necessary Functions/Definitions

### Default Minimum Decoys

This is the default number of decoys an input must at least have.

> There are exceptions to this being the minimum decoy size for all transactions. See further down in [Rules](#rules).

| Hard-Fork | Minimum Decoys[^min-decoys] |
| --------- | --------------------------- |
| 1         | N/A                         |
| 2 to 5    | 2                           |
| 6         | 4                           |
| 7         | 6                           |
| 8 to 14   | 10                          |
| 15+       | 15                          |

### Minimum And Maximum Decoys Used

To check a transaction input's `ring` size we must first get the minimum and maximum number of `decoys`
used in the transactions inputs[^min-max-decoys].

So if this was our transactions:

| Input     | 1  | 2 | 3  |
| --------- | -- | - | -- |
| Ring size | 12 | 8 | 16 |

The minimum and maximum amount of decoys would be 7 and 15 respectively.

### Mixable And Un-Mixable Inputs

A mixable input is one that has enough outputs on the chain with the same amount to be able to build a ring with the
minimum amount of decoys needed.

A ringCT input, aka an output with 0 amount, is always considered mixable[^0-amt-mixable].

For other inputs you first get the amount of outputs on chain with that amount and check if that's less than or equal
to the [default minimum amount of decoys](#default-minimum-decoys) if it is then the input is un-mixable otherwise it is
mixable[^check-mixability].

## Rules

### No Empty Inputs

The transaction must have at least 1 input[^no-empty-ins].

### No Empty decoys

All inputs must have decoys[^empty-decoys].

### Input Type

All inputs must be of type `txin_to_key`[^input-types].

### Inputs Must Not Overflow

The inputs when summed must not overflow a `u64` and the outputs when summed must not either[^amount-overflow].

### Unique Ring Members

From hard-fork 6, all ring members in an input must be unique, this is done by checking that
no `key_offset` after the first is 0[^unique-ring].

### Unique Key Image

The key image must be unique in a transaction[^key-images-in-tx] and the whole chain [^key-images-in-chain].

### Torsion Free Key Image

The key image must be a canonical prime order point[^torsion-free-keyimage].

### Minimum Decoys

These rules are in effect from hard fork 2.

First you get the [minimum number of decoys used in the transaction](#minimum-and-maximum-decoys-used).

Then you get the [amount of mixable and un-mixable inputs](#mixable-and-unmixable-inputs).

Now get the [default minimum decoys allowed for the current hard-fork](#default-minimum-decoys).

If the minimum amount of decoys used in the transaction is less than the default minimum decoys allowed then the transaction is only
allowed if there is at least one input which is un-mixable[^tx-without-minimum-decoys].

If there is an un-mixable then the transaction is not allowed to have more than 1 mixable input as well.

Special rules[^min-decoys-special-rules]:

- For hard-fork 15, both 10 and 15 decoys are allowed.
- From hard-fork 8 upwards, the minimum amount of decoys used in a transaction must be equal to the minimum allowed.

### Equal Number Of Decoys

From hard-fork 12, all inputs must have the same number of decoys[^equal-decoys].

### Sorted Inputs

From hard-fork 7, the inputs must be sorted by key image, in descending lexicographic order[^sorted-kis].

### 10 Block Lock

From hard-fork 12, all ring members must be at least 10 blocks old[^minimum-out-age].

### The Output Must Exist

The output a transaction references must exist in the chain[^output-must-exist].

### The Output Must Not Be Locked

The outputs, which are referenced in the inputs, unlock time must have passed, see the [chapter on unlock time](./unlock_time.md).

---

[^min-decoys]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L3345>

[^min-max-decoys]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L3369-L3373>

[^0-amt-mixable]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L3357>

[^check-mixability]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L3361-L3367>

[^no-empty-ins]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/cryptonote_core.cpp#L1125>

[^empty-decoys]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L3473>

[^input-types]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_basic/cryptonote_format_utils.cpp#L844>

[^amount-overflow]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_basic/cryptonote_format_utils.cpp#L871>

[^unique-ring]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/cryptonote_core.cpp#L1309>

[^key-images-in-tx]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/cryptonote_core.cpp#L1297>

[^key-images-in-chain]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L3475>

[^torsion-free-keyimage]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/cryptonote_core.cpp#L1324>

[^tx-without-minimum-decoys]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L3392>

[^min-decoys-special-rules]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L3406-L3410>

[^equal-decoys]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L3378>

[^sorted-kis]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L3435>

[^minimum-out-age]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L3533>

[^output-must-exist]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L3995>
