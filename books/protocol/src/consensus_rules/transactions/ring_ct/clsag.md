# CLSAG Rules

## Introduction

These rules apply to all ringCT types that use CLSAG signatures.

## Rules

### Number Of CLSAGs

There must be the same number of CLSAG signatures as there are inputs.[^numb-clsags]

### `s` Size

The `s` field must have has many elements as the amount of ring members.[^s-size]

### Canonical Encoding

All `s` scalars must be fully reduced, the `c1` scalar must be fully reduced[^scalars-reduced] and the `D` point must be canonically encoded.[^D-canonical]

### Key Images Not Identity

The key image and 8 * `D`, the commitment key image, must both not be the identity point.[^kis-not-identity]

### The CLSAG Signature Must Be Correctly Formed

The signature must be valid.[^clsag-valid]

---

[^numb-clsags]: <https://github.com/monero-project/monero/blame/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctTypes.h#L496>

[^s-size]: <https://github.com/monero-project/monero/blame/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L880>

[^scalars-reduced]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L881-L883>

[^D-canonical]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L894>

[^kis-not-identity]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L895> and <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L884>

[^clsag-valid]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L872>
