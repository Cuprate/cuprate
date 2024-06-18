# Consensus Rules

This chapter contains all of Monero's consensus rules, from genesis to now. Some rules
are complex so have been split into their own chapter.

Rules that are not bound to consensus (relay rules) are not included here. Also we have not documented "rules" which are enforced by
(de)serialization, for example it's impossible to have a ringCT signature in a version 1 transaction, rules that are unclear if they
can be omitted or not should _always_ be included.

## Index

1. [The Genesis Block](./consensus_rules/genesis_block.md)
2. [Hard Forks](./consensus_rules/hardforks.md)
3. [Blocks](./consensus_rules/blocks.md)
4. [Transactions](./consensus_rules/transactions.md)

## Definitions

Canonically Encoded Scalar:
an Ed25519 scalar which is fully reduced mod l, where \\(l = 2^{252} + 27742317777372353535851937790883648493 \\).

Canonically Encoded Point:
an Ed25519 point which is not the negative identity and with y coordinate fully reduced mod p, where \\(p = 2^{255} - 19 \\).

Prime Order Point:
a point in the prime subgroup.

POW Hash:
the hash calculated by using the active proof of work function.

Block Hash:
the keccak hash of the block.

Transaction Blob:
the raw bytes of a serialized transaction.

Block Blob:
the raw bytes of a serialized block.

Chain Height:
the amount of blocks in the chain, this is different to the height of the top block as
blocks start counting at 0.

Ring (transactions inputs):
the group of potential outputs of which one is the true spend.

Decoys (transactions inputs):
the fake outputs used to hide the true spend, the length of this is equal to one minus the `Rings` length.

MixIns (transactions inputs):
another term for `Decoys`
