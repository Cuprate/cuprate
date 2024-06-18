# Transaction Rules

## Introduction

This chapter does not include miner, coinbase, transactions as they are handled elsewhere, the rules for them are under [blocks](blocks.md)

## Index

1. [Miscellaneous Rules](#miscellaneous-rules)
2. [Input Rules](./transactions/inputs.md)
3. [Output Rules](./transactions/outputs.md)
4. [Unlock Time](./transactions/unlock_time.md)
5. [Ring Signatures](./transactions/ring_signatures.md)
6. [RingCT](./transactions/ring_ct.md)

## Miscellaneous Rules

### Version

Version 0 is never allowed[^tx-v0].

The max transaction version is 1 up to hard fork 4 then the max is 2[^max-tx-version].

The minimum tx version is 1 up till version 6 then if the [number of un-mixable inputs](#minimum-decoys)
is 0 the minimum is 2 otherwise 1[^min-tx-version] so a version 1 transaction is allowed if the amount
it's spending does not have enough outputs with the same amount to mix with.

### Transaction Size

The size of the `transaction blob` must not be bigger than 1 million bytes[^tx-size-limit].

From v8 the transactions _weight_ must not be bigger than half of the [block penalty free zone](./blocks/weights.md#penalty-free-zone) minus 600[^tx-weight_limit].

#### Calculating Transaction Weight

For all transactions that don't use bulletproofs or bulletproofs+ the weight is just the length of the transaction blob.[^weight-pre-bp]

For bulletproofs(+) transactions we add a "clawback" onto the transaction.

To calculate the "clawback" we fist define a `bpBase` which is the size of a 2 output proof, normalized to 1 proof by dividing by 2[^bp-base]:

for bulletproofs: \\(fields = 9\\)

for bulletproofs+: \\(fields = 6\\)

\\(bpBase = \frac{(32 * (fields + 7 * 2))}{2}\\)

Next we calculate the size of the bulletproofs(+) field by first getting the first power of 2 above or equal to the number of outputs: `firstPower2AboveNumbOuts`.

If `firstPower2AboveNumbOuts` is <= 2 then the \\(clawback = 0\\)[^fp2-less-than-2].

Next define the number of L and R elements[^lr-elements]: \\(nlr = firstPower2AboveNumbOuts + 6\\)

now the size of the bulletproofs(+) field is[^bp+-size]:

\\(bpSize = 32 * (fields + 2 * nlr)\\)

now the `clawback` is[^clawback]:

\\( clawback = \frac{(bpBase * firstPower2AboveNumbOuts - bpSize) * 4}{ 5} \\)

To get the transaction weight now you just get the length of the transaction blob and add this `clawback`[^bp-tx-weight].

---

[^tx-v0]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/tx_pool.cpp#L152>

[^max-tx-version]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L3418>

[^min-tx-version]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L3425>

[^tx-size-limit]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/cryptonote_core.cpp#L791>
and <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_basic/cryptonote_basic_impl.cpp#L78>

[^tx-weight_limit]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/tx_pool.cpp#L117> && <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/tx_pool.cpp#L221>

[^weight-pre-bp]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/cryptonote_basic/cryptonote_format_utils.cpp#L447-L453>

[^bp-base]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/cryptonote_basic/cryptonote_format_utils.cpp#L110C40-L110C40>

[^fp2-less-than-2]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/cryptonote_basic/cryptonote_format_utils.cpp#L112>

[^lr-elements]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/cryptonote_basic/cryptonote_format_utils.cpp#L117>

[^bp+-size]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/cryptonote_basic/cryptonote_format_utils.cpp#L118>

[^clawback]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/cryptonote_basic/cryptonote_format_utils.cpp#L122>

[^bp-tx-weight]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/cryptonote_basic/cryptonote_format_utils.cpp#L457>
