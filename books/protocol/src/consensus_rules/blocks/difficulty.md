# Difficulty

Difficulty is a measure used to keep block production at a constant rate, it is the average amount of hashes before a solution
is found.

## Checking A Blocks Proof Of Work

To check a blocks `POW hash` you interpret the hash as a little endian integer and multiply it by the difficulty, if the result
does not overflow the hash is valid[^check-pow]:

\\(Hash * difficulty <= MAXu256 \\)

## Calculating Difficulty

To calculate difficulty, Monero keeps a window of that last 735[^diff-blocks-count] timestamps and cumulative difficulties,
if there are not enough blocks, then you just use as many as possible.

> The genesis block is skipped for these calculations[^skip-genesis] so should not be included in the timestamp/ CD list but it is
> included in the cumulative difficulty of the chain.

If the amount of blocks is less than or equal to 1 then 1 is returned as the difficulty[^amt-blocks-1].

The timestamps and cumulative difficulties are then shortened to 720[^diff-window] blocks so that calculations lag 15 blocks behind
the chain.

The timestamps are then sorted, in ascending order. We now need to get a time span value to do this we first remove the outliers:

If the number of timestamps is less than or equal to the amount of blocks we are accounting for (600: 720 - 2 * 60) then the lower
is set to 0 and the upper is set to the length of timestamps. Otherwise, if we have enough timestamps, the lower and upper is calculated
by[^calculating-lower-upper]:

\\(lower = \frac{len(timestamps) - 600+1}{2} \\)

\\(upper = lower + 600 \\)

We then get the timestamp at position `lower` and take this away from the timestamp at position `upper -1` to get `timeSpan`.
If `timeSpan` is 0 we set it to 1[^timespan0].

We also get the cumulative difficulty at position `lower` and take this away from the cumulative difficulty at position `upper -1` to get `totalWork`.

The next difficulty is then calculated by[^final-diff-cal]:

\\(difficulty = \frac{totalWork * targetSeconds + timeSpan -1}{timeSpan} \\)

## Target Seconds

For hard-fork v1 the target seconds is 60, so one block a minute. For hard-fork 2 onwards block time is 120[^target-block-time].

---

[^check-pow]: <https://github.com/monero-project/monero/blob/67d190ce7c33602b6a3b804f633ee1ddb7fbb4a1/src/cryptonote_basic/difficulty.cpp#L196>

[^diff-blocks-count]: <https://github.com/monero-project/monero/blob/67d190ce7c33602b6a3b804f633ee1ddb7fbb4a1/src/cryptonote_config.h#L84>

[^skip-genesis]: <https://github.com/monero-project/monero/blob/67d190ce7c33602b6a3b804f633ee1ddb7fbb4a1/src/cryptonote_core/blockchain.cpp#L849C40-L849C65>

[^amt-blocks-1]: <https://github.com/monero-project/monero/blob/67d190ce7c33602b6a3b804f633ee1ddb7fbb4a1/src/cryptonote_basic/difficulty.cpp#L214>

[^diff-window]: <https://github.com/monero-project/monero/blob/67d190ce7c33602b6a3b804f633ee1ddb7fbb4a1/src/cryptonote_config.h#L81> && <https://github.com/monero-project/monero/blob/67d190ce7c33602b6a3b804f633ee1ddb7fbb4a1/src/cryptonote_basic/difficulty.cpp#L205>

[^calculating-lower-upper]: <https://github.com/monero-project/monero/blob/67d190ce7c33602b6a3b804f633ee1ddb7fbb4a1/src/cryptonote_basic/difficulty.cpp#L222>

[^timespan0]: <https://github.com/monero-project/monero/blob/67d190ce7c33602b6a3b804f633ee1ddb7fbb4a1/src/cryptonote_basic/difficulty.cpp#L231>

[^final-diff-cal]: <https://github.com/monero-project/monero/blob/67d190ce7c33602b6a3b804f633ee1ddb7fbb4a1/src/cryptonote_basic/difficulty.cpp#L236>

[^target-block-time]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L5512>
