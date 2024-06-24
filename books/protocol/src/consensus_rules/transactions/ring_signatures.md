# Transaction Version 1 Rules

## Introduction

These rules apply only to version 1, pre-ringCT, transactions.

## Rules

### Amount Of Ring Signatures

The amount of ring signatures must be the same as the number of inputs[^amt-of-ring-sigs].

### Amount Of Signatures In A Ring

For a ring signature at a certain index, the input at that same index must have the same amount of ring members as the ring signature has signatures[^amt-of-sigs].

### Signatures Must Be Canonical

Every signatures c and r value must be `canonical scalars`[^canonical-sig].

### Ring Members Must Be Valid Points

All outputs used as ring members must be valid canonical points[^valid-members].

### The Ring Signature Must Be Valid

The ring signature must be correctly formed[^ring-sig-correct].

---

[^amt-of-ring-sigs]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L3485> and <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_basic/cryptonote_basic.h#L266>

[^amt-of-sigs]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L3999> and <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_basic/cryptonote_basic.h#L271-L282>

[^canonical-sig]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/crypto/crypto.cpp#L735>

[^valid-members]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/crypto/crypto.cpp#L738>

[^ring-sig-correct]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/crypto/crypto.cpp#L711>
