# Bulletproofs+ Rules

## Introduction

These rules apply to all ringCT types that use bulletproofs+.

## Rules

### L & R Length

The Length of the L & R fields must be the same, they must both be equal to \\( 6 + log_2(firstPower2AboveNumbOuts) \\).[^L-R-Size]

Where `firstPower2AboveNumbOuts` is the first power of 2 above or equal to the amount of outputs in the transaction, so:

If outputs = 3, firstPower2AboveNumbOuts = 4.

If outputs = 8, firstPower2AboveNumbOuts = 8.

### Number Of Bulletproofs

There must only be one bulletproof in a transaction.[^one-bulletproof+]

### Max Outputs

The amount of outputs in the transaction must not be more than 16 [^max-outputs]

### Canonical Encoding

`r1`, `s2`, `d1` must all be canonically encoded, reduced, scalars.[^scalars-reduced] All the points of `V`, `L` and `R` must be canonically encoded and `A1`, `B` and
`A` must canonically encoded points.[^canonical-points]

### At Least One Output

There must be at least one element of V, which is constructed from the outPKs which must have the same number of elements as the outputs.[^one-out]

### The Bulletproof Must Be Valid

The bulletproof must pass verification. [^bulletproof+-valid]

[^L-R-Size]: <https://github.com/monero-project/monero/blob/master/src/ringct/rctTypes.cpp#L300-L304> && <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/bulletproofs_plus.cc#L850>

[^one-bulletproof+]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/cryptonote_basic/cryptonote_format_utils.cpp#L173>

[^max-outputs]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/cryptonote_core/cryptonote_core.cpp#L887>

[^scalars-reduced]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/bulletproofs_plus.cc#L825-L827>

[^canonical-points]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/bulletproofs_plus.cc#L931-L939> && <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctOps.cpp#L415>

[^one-out]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/bulletproofs_plus.cc#L829>

[^bulletproof+-valid]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/bulletproofs_plus.cc#L799>
