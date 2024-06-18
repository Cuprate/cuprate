# Borromean Rules

## Introduction

These rules apply to all ringCT types that use Borromean ring signatures to prove an output amount is in the correct range.

## Rules

### Number Of Borromean Range Proofs

The amount of Borromean range proofs must be the same as the number of outputs.[^numb-borro]

### Ci Valid Points

Each Ci (bit commitment) must be canonically encoded points.[^ci-valid-points]

### Sum Ci

For a range proof at a certain index the sum of each Ci must equal the outPK at that index.[^sum-ci]

### Borromean Scalar Encoding

Monero does not check that the scalars `s0` and `s1` are reduced this leads to them, if not reduced, being interpreted as a different scalar by the `slide` function
which calculates the 5-NAF of the number. The `slide` function restricts it's output to 256 bytes however if the last bit is set on the input this could lead to the
5-NAF of the scalar being 257 bytes long. There are scalars on the chain which have this behavior.[^scalar-report]

The scalar `ee` must be a fully reduced scalar as it is compared against the raw bytes of an output from the `hash_to_scalar` function.[^s0-s1-ee-encoding]

### The Borromean Ring Must Be Valid

To verify a Borromean ring signature is valid you must first set up the public keys that the ring will be verified with, one member of the ring will be a Ci the
other will be (\\(Ci - H * 2^X \\)), where X is the index of the Ci. By setting up the ring like this the prover will only know the discreet log of a
ring member if either the Ci is a commitment to 0 or \\(2^X\\)[^public-key-setup].

After setting up the public keys the actual borromean rings must be valid.[^ring-valid]

---

[^numb-borro]: <https://github.com/monero-project/monero/blame/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctTypes.h#L480>

[^ci-valid-points]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L581>

[^sum-ci]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L590>

[^scalar-report]: <https://www.moneroinflation.com/static/data_py/report_scalars_df.pdf>

[^s0-s1-ee-encoding]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L213-L222>

[^public-key-setup]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L574-L577>

[^ring-valid]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L208>
