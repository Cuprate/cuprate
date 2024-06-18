# Transaction Outputs

## Introduction

These rules apply to transaction outputs, excluding miner transaction outputs.

## Rules

### Outputs Must Not Overflow

The and the outputs when summed must not overflow a u64[^amount-overflow].

### Output Amount

For version 1 txs sum of the outputs must be less than the sum of the inputs, the difference between the
inputs and the outputs is then the fee.[^more-in-than-out] The amount of each outputs must also not be zero.[^zero-output]

From hard-fork 2 version 1 transaction output amounts also must be validly decomposed[^decomposed-amounts].
A valid decomposed amount is an amount contained in [this table](https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_basic/cryptonote_format_utils.cpp#L52)

For version 2 txs all outputs must have a zero amount.[^v2-output-amount]

### Output Keys Canonical

All output public keys must be `canonical points`[^output-key-canonical].

> This was added as a rule in hard-fork 4 but that check is redundant as it was done before that.
> So how did invalid keys get on the chain? [miner txs](./blocks/miner_tx.md).

### Output Type

The output type allowed depends on the hard-fork[^output-types]:

| hard-fork  | output type                          |
| ---------- | ------------------------------------ |
| 1 to 14    | txout_to_key                         |
| 15         | txout_to_key and txout_to_tagged_key |
| 16 onwards | txout_to_tagged_key                  |

> For hard-fork 15 both are allowed but the transactions outputs must all be the same type[^same-output-type].

### 2 Outputs

From hard-fork 12 version (RCT) 2 transactions must have 2 outputs[^minimum-2-outs].

---

[^amount-overflow]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_basic/cryptonote_format_utils.cpp#L871>

[^more-in-than-out]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/cryptonote_core.cpp#L1163> and <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/tx_pool.cpp#L190-L204>

[^zero-output]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_basic/cryptonote_format_utils.cpp#L862>

[^decomposed-amounts]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L3048>

[^v2-output-amount]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/cryptonote_core/blockchain.cpp#L3059>

[^output-key-canonical]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_basic/cryptonote_format_utils.cpp#L865>

[^output-types]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_basic/cryptonote_format_utils.cpp#L960>

[^same-output-type]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_basic/cryptonote_format_utils.cpp#L984>

[^minimum-2-outs]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/cryptonote_core/blockchain.cpp#L3324>
