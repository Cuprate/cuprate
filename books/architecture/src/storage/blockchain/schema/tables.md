# Tables

> See also: <https://doc.cuprate.org/cuprate_blockchain/tables> & <https://doc.cuprate.org/cuprate_blockchain/types>.

The `CamelCase` names of the table headers documented here (e.g. `TxIds`) are the actual type name of the table within `cuprate_blockchain`.

Note that words written within `code blocks` mean that it is a real type defined and usable within `cuprate_blockchain`. Other standard types like u64 and type aliases (TxId) are written normally.

Within `cuprate_blockchain::tables`, the below table is essentially defined as-is with [a macro](https://github.com/Cuprate/cuprate/blob/31ce89412aa174fc33754f22c9a6d9ef5ddeda28/database/src/tables.rs#L369-L470).

Many of the data types stored are the same data types, although are different semantically, as such, a map of aliases used and their real data types is also provided below.

| Alias                                              | Real Type |
|----------------------------------------------------|-----------|
| BlockHeight, Amount, AmountIndex, TxId, UnlockTime | u64
| BlockHash, KeyImage, TxHash, PrunableHash          | [u8; 32]

---

| Table             | Key                  | Value              | Description |
|-------------------|----------------------|--------------------|-------------|
| `BlockBlobs`      | BlockHeight          | `StorableVec<u8>`  | Maps a block's height to a serialized byte form of a block
| `BlockHeights`    | BlockHash            | BlockHeight        | Maps a block's hash to its height
| `BlockInfos`      | BlockHeight          | `BlockInfo`        | Contains metadata of all blocks
| `KeyImages`       | KeyImage             | ()                 | This table is a set with no value, it stores transaction key images
| `NumOutputs`      | Amount               | u64                | Maps an output's amount to the number of outputs with that amount
| `Outputs`         | `PreRctOutputId`     | `Output`           | This table contains legacy CryptoNote outputs which have clear amounts. This table will not contain an output with 0 amount.
| `PrunedTxBlobs`   | TxId                 | `StorableVec<u8>`  | Contains pruned transaction blobs (even if the database is not pruned)
| `PrunableTxBlobs` | TxId                 | `StorableVec<u8>`  | Contains the prunable part of a transaction
| `PrunableHashes`  | TxId                 | PrunableHash       | Contains the hash of the prunable part of a transaction
| `RctOutputs`      | AmountIndex          | `RctOutput`        | Contains RingCT outputs mapped from their global RCT index
| `TxBlobs`         | TxId                 | `StorableVec<u8>`  | Serialized transaction blobs (bytes)
| `TxIds`           | TxHash               | TxId               | Maps a transaction's hash to its index/ID
| `TxHeights`       | TxId                 | BlockHeight        | Maps a transaction's ID to the height of the block it comes from
| `TxOutputs`       | TxId                 | `StorableVec<u64>` | Gives the amount indices of a transaction's outputs
| `TxUnlockTime`    | TxId                 | UnlockTime         | Stores the unlock time of a transaction (only if it has a non-zero lock time)

<!-- TODO(Boog900): We could split this table again into `RingCT (non-miner) Outputs` and `RingCT (miner) Outputs` as for miner outputs we can store the amount instead of commitment saving 24 bytes per miner output. -->