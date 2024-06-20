# Unlock Time

To spend an output the output's unlock time must have passed.

## Interpreting An Unlock Time

The unlock time is just a 64 bit unsigned number. It is interpreted as a block height if less than 500,000,000 otherwise it's a Unix timestamp[^interpreting-unlock-time].

## Checking The Output Is Unlocked

### Block Height

First you get the top blocks height and add one, we do this because we are checking if
the transaction is allowed in the next block not the last.

We now check if this height is greater than or equal to the unlock time if it is then
accept the block[^height-accepting].

### Timestamp

#### Getting The Current Time

Before hard-fork 13, this was done by just getting the computer's time, from hf 13 onwards, we use
an average over the last blocks[^getting-time].

Monero uses the last 60 blocks to get an average, if the `chain height` is less than
60, just use the current time[^height-less-60].

First you get the median timestamp of the last 60 blocks. We then project this
timestamp to match approximately when the block being validated will appear, to do
this we do[^median-timestamp]:

\\(adjustedMedian = median + \frac{(TimestampWindow + 1) * DifficultyTarget}{2} \\)

where:

\\(TimestampWindow = 60\\)

\\(DifficultyTarget = 120\\)

You then get the top block's timestamp and add the target seconds per block[^adjusting-top-block].

The timestamp we use is then the minimum out of the adjusted median and adjusted most
recent timestamp[^minimum-timestamp].

### Checking Timestamp Has Passed

Now with our timestamp we add the [target seconds](../blocks/difficulty.md#target-seconds)
per block and check if this is more than or equal to the unlock
time[^checking-timestamp].

---

[^interpreting-unlock-time]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L3921>

[^height-accepting]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L3925>

[^getting-time]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L3933>

[^height-less-60]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L4011>

[^median-timestamp]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L4024-L4028>

[^adjusting-top-block]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L4032>

[^minimum-timestamp]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L4036>

[^checking-timestamp]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L3934>
