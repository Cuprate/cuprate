# Block Weights

Monero's blockchain, unlike other blockchains, has dynamic block sizes which means blocks expand to handle demand.
However Monero does not allow unrestricted block growth, miners will face a penalty for expanding blocks and miners
are restricted by how much they can expand a block.

## Index

1. [Penalty Free Zone](weights.md#penalty-free-zone)
2. [Blocks Weight](#blocks-weight)
3. [Long Term Block Weight](#long-term-block-weight)
4. [Effective Median Weight](#effective-median-weight)
5. [Median Weight For Coinbase Checks](#median-weight-for-coinbase-checks)

## Penalty Free Zone

Monero sets a minimum max block weight so that miners don't get punished for expanding small blocks.

For hf 1 this is 20000 bytes, for hf 2-4 this is 60000 and from 5 onwards this is 300000 bytes[^minimum-max-weight].

## Blocks Weight

A blocks weight is the sum of all the transactions weights in a block, including the miner transaction. The block header
and transaction hashes are not included[^calculating-bw].

## Long Term Block Weight

The blocks long term weight is the blocks weight adjusted with previous blocks weights.

### Calculating A Blocks Long Term Weight

Up till hard-fork 10 the blocks long term weight is just the blocks weight[^pre-hf-10-long-weight].

From hard-fork 10 onwards we first get the median long term weight over the last 100000 blocks, if this is less than
the [penalty free zone](#penalty-free-zone) then set the median long term weight to this instead[^ltw-median].

Now we need to set a shot term constraint and adjusted block weight, the way we do this is different depending on the hard-fork.

From hard-fork 10 to 14[^hf-10-14-stc]:

\\(adjustedBlockWeight = blockWeight\\)

\\(shortTermConstraint = medianLongTermWeight * 1.4\\)

From 15 onwards[^hf-15-adjustments]:

\\(adjustedBlockWeight = max(blockWeight, \frac{medianLongTermWeight}{1.7})\\)

\\(shortTermConstraint = medianLongTermWeight * 1.7\\)

Now the long term weight is defined as `min(adjustedBlockWeight, shortTermConstraint)`[^long-term-weight].

## Effective Median Weight

The effective median weight is used to calculate block reward and to limit block size.

### Calculating Effective Median Weight

For any hard-fork the minimum this can be is the [penalty free zone](#penalty-free-zone)[^minimum-effective-median].

Up till hard-fork 10 this is done by just getting the median **block weight** over the last 100 blocks[^pre-hf-10-effective-median], if
there are less than 100 blocks just get the median over all the blocks.

For hf 10 onwards we first get the median **long term weight** over the last 100000 blocks[^hf-10+-effective-median-step-1], if this median
is less than the hf 5 [penalty free zone](#penalty-free-zone) set the median to that, this is the long term median.

Now get the median **block weight** over the last 100 blocks, this is the short term median.

Now we can calculate the effective median, for hard-forks 10 to 14 this is done by[^effective-median]:

\\(effectiveMedian = min(max(hf5PenaltyFreeZone, shortTermMedian), 50 * longTermMedian) \\)

From 15 onwards this is done by:

\\(effectiveMedian = min(max(longTermMedian, shortTermMedian), 50 * longTermMedian) \\)

## Median Weight For Coinbase Checks

When checking coinbase transactions and block weight Monero uses yet another median weight :).

### Calculating Median Weight For Coinbase Checks

Before hf 12 this is the median block weight over the last 100 blocks[^median-weight-coinbase-before-v12].

From hf 12 this is the [effective median weight](#effective-median-weight)[^median-weight-coinbase-from-v12]

---

[^minimum-max-weight]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_basic/cryptonote_basic_impl.cpp#L69>

[^calculating-bw]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L4289> and <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L4408>

[^pre-hf-10-long-weight]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L4577>

[^ltw-median]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L4581>

[^hf-10-14-stc]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L4593>

[^hf-15-adjustments]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L4587>

[^long-term-weight]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L4595>

[^minimum-effective-median]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L4676>

[^pre-hf-10-effective-median]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L4611>

[^hf-10+-effective-median-step-1]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L4651>

[^effective-median]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L4659-L4671>

[^median-weight-coinbase-before-v12]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L1425-L1427>

[^median-weight-coinbase-from-v12]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L1421>
