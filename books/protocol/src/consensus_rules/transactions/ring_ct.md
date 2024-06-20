# Ring Confidential Transactions

## Introduction

Ring confidential transactions are version 2 Monero transactions which keep amounts hidden. They were activated at hard-fork 4. There are multiple
types of RingCT transactions that were activated and deprecated at different hard-forks.

## Definitions

OutPK:
A pedersen commitment to the output amount.

Pseudo-outs:
A pedersen commitment to the true spends amount with a different mask, such that the sum of the pseudo-outs is the same as the sum of the outPKs + fee * H.

## Index

1. [Rules That Apply To All Types](#rules-that-apply-to-all-types)
2. [Simple Types Rules](#simple-types-rules)
3. [Borromean Rules](./ring_ct/borromean.md)
4. [MLSAG Rules](./ring_ct/mlsag.md)
5. [Bulletproofs Rules](./ring_ct/bulletproofs.md)
6. [CLSAG Rules](./ring_ct/clsag.md)
7. [Bulletproofs+ Rules](./ring_ct/bulletproofs+.md)

## Rules That Apply To All Types

### Type

RingCT type define the proofs used in the transaction, the ringCT types allowed depend on the hard-fork:

| Type (Name)      | Short description                                                     | Hard Fork allowed                                          | Hard Fork disallowed                                                  |
| ---------------- | --------------------------------------------------------------------- | ---------------------------------------------------------- | --------------------------------------------------------------------- |
| 0 (NULL)         | No ringCT signatures, used for coinbase transactions                  | 4 (only miner transactions) [^first-three-type-activation] | Still allowed                                                         |
| 1 (Full)         | A single aggregate MLSAG signature with borromean range proofs        | 4 [^first-three-type-activation]                           | 9 [^bulletproof-activated-borromean-disallowed]                       |
| 2 (Simple)       | MLSAG signatures per input with borromean range proofs                | 4 [^first-three-type-activation]                           | 9 [^bulletproof-activated-borromean-disallowed]                       |
| 3 (Bulletproof)  | MLSAG signatures per input with a single bulletproof for all outputs  | 8 [^bulletproof-activated-borromean-disallowed]            | 11 [^bulletproof2-activated-bulletproof-disallowed]                   |
| 4 (Bulletproof2) | Uses the same signatures as type 3                                    | 10 [^bulletproof2-activated-bulletproof-disallowed]        | 14 (except 2 transactions) [^clsag-activated-bulletproof2-disallowed] |
| 5 (CLSAG)        | CLSAG signatures per input with a single bulletproof for all outputs  | 13 [^clsag-activated-bulletproof2-disallowed]              | 16 [^bulletproof+-activated-clsag-disallowed]                         |
| 6 (Bulletproof+) | CLSAG signatures per input with a single bulletproof+ for all outputs | 15 [^bulletproof+-activated-clsag-disallowed]              | Still allowed                                                         |
| 6+               | Future type not currently allowed                                     | Not allowed [^future-rct-types]                            | Not allowed                                                           |

There are 2 type 4 RCT transactions that are allowed after hard-fork 13, this was due to a bug in which transactions added to the txpool before a fork
were not being checked for new fork rules they are:
`c5151944f0583097ba0c88cd0f43e7fabb3881278aa2f73b3b0a007c5d34e910` and `6f2f117cde6fbcf8d4a6ef8974fcac744726574ac38cf25d3322c996b21edd4c`[^grandfathered-txs].

### OutPKs Valid Points

All outPKs must be canonically encoded points[^outPKs-valid-points].

## Simple Types Rules

These rules apply to all RCT "simple" types, which are all except type "FULL".

### Pseudo-outs Valid Points

This rule applies to the pseudo-outs, from type 3 (Bulletproof) the pseudo-outs field moved to the prunable RCT section from the non-prunable section.

The pseudo-outs must all be canonically encoded points[^pseudo-outs-valid-points].

### Pseudo-outs OutPKs Balance

The sum of the pseudo-outs must equal the sum of the OutPKs + fee * H:[^simple-amounts-balance]

\\(\sum PseudoOuts == \sum outPK + fee * H \\)

---

[^first-three-type-activation]: There is no direct code allowing these types of RingCT, these are the original types that got activated when version 2 transactions
got activated

[^bulletproof-activated-borromean-disallowed]: <https://github.com/monero-project/monero/blob/master/src/cryptonote_core/blockchain.cpp#L3083-L3107>

[^bulletproof2-activated-bulletproof-disallowed]: <https://github.com/monero-project/monero/blob/master/src/cryptonote_core/blockchain.cpp#L3108-L3130>

[^clsag-activated-bulletproof2-disallowed]: <https://github.com/monero-project/monero/blob/master/src/cryptonote_core/blockchain.cpp#L3132-L3166>

[^bulletproof+-activated-clsag-disallowed]: <https://github.com/monero-project/monero/blob/master/src/cryptonote_core/blockchain.cpp#L3168-L3193>

[^future-rct-types]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctTypes.h#L335>

[^grandfathered-txs]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/cryptonote_core/blockchain.cpp#L3150>

[^outPKs-valid-points]: For simple types: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L1444>,
For type FULL: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L829-L829>

[^pseudo-outs-valid-points]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L1449>

[^simple-amounts-balance]: <https://github.com/monero-project/monero/blob/ac02af92867590ca80b2779a7bbeafa99ff94dcb/src/ringct/rctSigs.cpp#L1453>
