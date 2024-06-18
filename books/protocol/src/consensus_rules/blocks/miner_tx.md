# Miner Transaction Rules

## Introduction

Miner transactions are handled differently to normal transactions, see [here](../transactions.md) for the rules on normal transactions.

## Rules

### Version

The transactions version must be either 1 or 2[^versions-allowed].

The version can be 1 or 2 up to hard-fork 12 then it must be 2[^weird-version-rules].

### Input

The transaction must only have one input and it must be of type `txin_gen`[^input-type].

The height specified in the input must be the actual block height[^input-height].

### RingCT Type

From hard-fork 12 version 2 miner transactions must have a ringCT type of `Null`[^null-ringct].

### Unlock Time

The unlock time must be the current height + 60[^miner-unlock-time].

### Output Amounts

The output, when summed, must not overflow[^outputs-overflow].

For _only_ hard-fork 3 the output amount must be a valid decomposed amount[^decomposed-amount], which means the amount must be
in [this](https://github.com/monero-project/monero/blob/67d190ce7c33602b6a3b804f633ee1ddb7fbb4a1/src/cryptonote_basic/cryptonote_format_utils.cpp#L52) table.

### Total Outputs

The [reward from the block](./reward.md#calculating-block-reward) + the total fees must not be more than the summed output amount[^total-output-amount].

For hard-fork 1 and from 12 onwards the summed output amount must equal the reward + fees[^exact-output-amount] this means from 2 till 11 miners can collect
less if they want less dust.

### Output Type

The output type allowed depends on the hard-fork[^output-types]:

| hard-fork  | output type                          |
| ---------- | ------------------------------------ |
| 1 to 14    | txout_to_key                         |
| 15         | txout_to_key and txout_to_tagged_key |
| 16 onwards | txout_to_tagged_key                  |

> For hard-fork 15 both are allowed but the transactions outputs must all be the same type.

### Zero Amount V1 Output

Monero does not explicitly ban zero amount V1 outputs on miner transactions but the database throws an error if a 0 amount output doesn't have a commitment
[^zero-output] meaning they are baned.

### V2 Output Pool

When adding version 2 miner transactions to the blockchain put the outputs into the 0 amount pool and create dummy commitments of:[^v2-output]

\\(commitment = G + amount * H \\)

---

[^versions-allowed]: <https://github.com/monero-project/monero/blob/67d190ce7c33602b6a3b804f633ee1ddb7fbb4a1/src/cryptonote_basic/cryptonote_basic.h#L185>

[^weird-version-rules]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L1371>

[^input-type]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L1369-L1370>

[^input-height]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L1379>

[^null-ringct]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L1374>

[^miner-unlock-time]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L1385>

[^outputs-overflow]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L1388>

[^decomposed-amount]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L1409>

[^total-output-amount]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L1434>

[^exact-output-amount]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L1440-L1447>

[^output-types]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_basic/cryptonote_format_utils.cpp#L960>

[^zero-output]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/blockchain_db/lmdb/db_lmdb.cpp#L1069>

[^v2-output]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/blockchain_db/blockchain_db.cpp#L234-L241>
