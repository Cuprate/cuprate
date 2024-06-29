# Block Reward

The block reward is the amount paid out to a miner for mining a block.

## Calculating Base Block Reward

The base block reward is the reward before factoring in the potential penalty for expanding blocks.

To calculate the base block reward you first need the total amount of coins already generated, then define:

[^money-supply] \\(moneySupply = 2^{64} -1 \\)

[^emission-speed-factor] \\(emissionSpeedFactor = 20 - (targetMinutes - 1) \\)

where `targetMinutes` is the [target block time](./difficulty.md#target-seconds) in minutes.

The `baseReward` is then calculated by:

[^base-reward] \\(baseReward = (moneySupply - alreadyGeneratedCoins) >> emissionSpeedFactor \\)

If `baseReward` falls below the final subsidy (0.3 XMR / minute) them set the `baseReward` to that instead [^final-base-reward].

## Calculating Block Reward

First calculate the [base block reward](#calculating-base-block-reward).

Now we need to get the [median weight for block rewards](weights.md#median-weight-for-coinbase-checks)

If the current block weight is not more than the median weight then the block reward is the base reward.

Otherwise the block reward is:[^block-reward]

\\(blockReward = baseReward * (1 - (\frac{blockWeight}{effectiveMedianWeight} -1)^2) \\)

---

[^money-supply]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_config.h#L53>

[^emission-speed-factor]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_basic/cryptonote_basic_impl.cpp#L87>

[^base-reward]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_basic/cryptonote_basic_impl.cpp#L89>

[^final-base-reward]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_basic/cryptonote_basic_impl.cpp#L90-L93>

[^block-reward]: <https://web.getmonero.org/library/Zero-to-Monero-2-0-0.pdf#subsection.7.3.3> && <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_basic/cryptonote_basic_impl.cpp#L111-L127>
