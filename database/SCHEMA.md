# Database Schema

This document contains Cuprates database schema, it may change a lot during this stage of development, nothing here is final.

## Transactions

### Tx IDs

This table will map a tx hash to tx id (u64).

| Key      | Value |
| -------- | ----- |
| [u8; 32] | u64   |

`Constant Size = true`

### Tx Heights

This table will map a tx to the block height it came from.

| Key        | Value        |
| ---------- | ------------ |
| TxID (u64) | Height (u64) |

`Constant Size = true`

### Tx Unlock Time

This table will store the unlock time of a tx (only if the tx has a non-zero lock-time).

| Key        | Value             |
| ---------- | ----------------- |
| TxID (u64) | unlock time (u64) |

`Constant Size = true`

### Pruned Tx Blobs

This table will contain pruned tx blobs (even if the DB is not pruned).

| Key        | Value                 |
| ---------- | --------------------- |
| TxID (u64) | Pruned Blob (Vec<u8>) |

`Constant Size = false`

### Prunable Tx Blobs

This table will contain the prunable part of a tx.

| Key        | Value                   |
| ---------- | ----------------------- |
| TxID (u64) | Prunable Blob (Vec<u8>) |

`Constant Size = false`

### Prunable Hash

This table will contain the hash of the prunable part of a tx.

| Key        | Value                    |
| ---------- | ------------------------ |
| TxID (u64) | Prunable Hash ([u8; 32]) |

`Constant Size = true`

## Tx Outputs

This table gives the amount idxs of the transaction outputs.

| Key        | Value                  |
| ---------- | ---------------------- |
| TxID (u64) | Amount Idxs (Vec<u64>) |

`Constant Size = false`

## Outputs

### CryptoNote Outputs

This table will contain legacy CryptoNote outputs which have clear amounts.
This table will not contain an output with 0 amount.

| Primary Key  | Secondary Key      | Value              |
| ------------ | ------------------ | ------------------ |
| Amount (u64) | Amount Index (u32) | Output (See below) |

> This table stores the amount idex as a u32 to save space as the creation of v1 txs is limited, u32::MAX should never be hit.

```rust
struct Output{
    key: [u8; 32],
    // We could get this from the tx_idx with the Tx Heights table but that would require another look up per out.
    height: u64, 
    tx_idx: u64,
    // For if the tx that created this out has a time-lock - this means we only need to look on Tx Unlock Time if this is true.
    locked: bool 
}
// TODO: local_index?

```

`Constant Size = true`

### RingCT Outputs

| Key                | Value                 |
| ------------------ | --------------------- |
| Amount Index (u64) | RctOutput (see below) |

```rust
struct RctOutput{
    key: [u8; 32],
    // We could get this from the tx_idx with the Tx Heights table but that would require another look up per out.
    height: u64, 
    tx_idx: u64,
    // For if the tx that created this out has a time-lock - this means we only need to look on Tx Unlock Time if this is true.
    locked: bool,
    // The amount commitment of this output.
    commitment: [u8; 32]
}
// TODO: local_index?

```

`Constant Size = true`

> TODO: We could split this table again into `RingCT (non-miner) Outputs` and `RingCT (miner) Outputs` as for miner outputs we can
> store the amount instead of commitment saving 24 bytes per miner output.

## Key Images

### Key Images

This table stores tx key images

| Key                  | Value |
| -------------------- | ----- |
| Key Image ([u8; 32]) | ()    |

`Constant Size = true`

## Blocks

### Block Heights

Maps a block hash to a height.

| Key                   | Value              |
| --------------------- | ------------------ |
| Block Hash ([u8; 32]) | Block Height (u64) |

`Constant Size = true`

### Block Blob

Stores the blocks blob.

| Key                | Value                |
| ------------------ | -------------------- |
| Block Height (u64) | Block Blob (Vec<u8>) |

`Constant Size = false`

### Block Info V1

Stores info about blocks up to HF 4

| Key                | Value                   |
| ------------------ | ----------------------- |
| Block Height (u64) | BlockInfoV1 (see below) |

```rust
struct BlockInfoV1 {
    timestamp: u64,
    total_generated_coins: u64,
    weight: u64,
    cumulative_difficulty: u64,
    block_hash: [u8; 32],
}
```

`Constant Size = true`

### Block Info V2

Stores info about blocks between HF 4 and 10

| Key                | Value                   |
| ------------------ | ----------------------- |
| Block Height (u64) | BlockInfoV2 (see below) |

```rust
struct BlockInfoV2 {
    timestamp: u64,
    total_generated_coins: u64,
    weight: u64,
    cumulative_difficulty: u64,
    block_hash: [u8; 32],
    cumulative_rct_outs: u32
}
```

`Constant Size = true`

### Block Info V3

Stores info about blocks from HF 10 onwards

| Key                | Value                   |
| ------------------ | ----------------------- |
| Block Height (u64) | BlockInfoV3 (see below) |

```rust
struct BlockInfoV2 {
    timestamp: u64,
    total_generated_coins: u64,
    weight: u64,
    cumulative_difficulty: u128,
    block_hash: [u8; 32],
    cumulative_rct_outs: u64,
    long_term_weight: u64
}
```

`Constant Size = true`

> When getting a blocks info start on table V1 and if a block isn't there move to the next or keep a cache of numb blocks in each table
> so we know what table to look in by the blocks height.