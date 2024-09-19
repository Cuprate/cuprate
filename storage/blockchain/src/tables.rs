//! Database tables.
//!
//! # Table marker structs
//! This module contains all the table definitions used by `cuprate_blockchain`.
//!
//! The zero-sized structs here represents the table type;
//! they all are essentially marker types that implement [`cuprate_database::Table`].
//!
//! Table structs are `CamelCase`, and their static string
//! names used by the actual database backend are `snake_case`.
//!
//! For example: [`BlockBlobs`] -> `block_blobs`.
//!
//! # Traits
//! This module also contains a set of traits for
//! accessing _all_ tables defined here at once.

//---------------------------------------------------------------------------------------------------- Import
use crate::types::{
    AltBlockHeight, AltChainInfo, AltTransactionInfo, Amount, AmountIndex, AmountIndices,
    BlockBlob, BlockHash, BlockHeight, BlockInfo, CompactAltBlockInfo, KeyImage, Output,
    PreRctOutputId, PrunableBlob, PrunableHash, PrunedBlob, RawChainId, RctOutput, TxBlob, TxHash,
    TxId, UnlockTime,
};

//---------------------------------------------------------------------------------------------------- Tables
// Notes:
// - Keep this sorted A-Z (by table name)
// - Tables are defined in plural to avoid name conflicts with types
// - If adding/changing a table also edit:
//   - the tests in `src/backend/tests.rs`
cuprate_database::define_tables! {
    /// Serialized block blobs (bytes).
    ///
    /// Contains the serialized version of all blocks.
    0 => BlockBlobs,
    BlockHeight => BlockBlob,

    /// Block heights.
    ///
    /// Contains the height of all blocks.
    1 => BlockHeights,
    BlockHash => BlockHeight,

    /// Block information.
    ///
    /// Contains metadata of all blocks.
    2 => BlockInfos,
    BlockHeight => BlockInfo,

    /// Set of key images.
    ///
    /// Contains all the key images known to be spent.
    ///
    /// This table has `()` as the value type, as in,
    /// it is a set of key images.
    3 => KeyImages,
    KeyImage => (),

    /// Maps an output's amount to the number of outputs with that amount.
    ///
    /// For example, if there are 5 outputs with `amount = 123`
    /// then calling `get(123)` on this table will return 5.
    4 => NumOutputs,
    Amount => u64,

    /// Pre-RCT output data.
    5 => Outputs,
    PreRctOutputId => Output,

    /// Pruned transaction blobs (bytes).
    ///
    /// Contains the pruned portion of serialized transaction data.
    6 => PrunedTxBlobs,
    TxId => PrunedBlob,

    /// Prunable transaction blobs (bytes).
    ///
    /// Contains the prunable portion of serialized transaction data.
    // SOMEDAY: impl when `monero-serai` supports pruning
    7 => PrunableTxBlobs,
    TxId => PrunableBlob,

    /// Prunable transaction hashes.
    ///
    /// Contains the prunable portion of transaction hashes.
    // SOMEDAY: impl when `monero-serai` supports pruning
    8 => PrunableHashes,
    TxId => PrunableHash,

    // SOMEDAY: impl a properties table:
    // - db version
    // - pruning seed
    // Properties,
    // StorableString => StorableVec,

    /// RCT output data.
    9 => RctOutputs,
    AmountIndex => RctOutput,

    /// Transaction blobs (bytes).
    ///
    /// Contains the serialized version of all transactions.
    // SOMEDAY: remove when `monero-serai` supports pruning
    10 => TxBlobs,
    TxId => TxBlob,

    /// Transaction indices.
    ///
    /// Contains the indices all transactions.
    11 => TxIds,
    TxHash => TxId,

    /// Transaction heights.
    ///
    /// Contains the block height associated with all transactions.
    12 => TxHeights,
    TxId => BlockHeight,

    /// Transaction outputs.
    ///
    /// Contains the list of `AmountIndex`'s of the
    /// outputs associated with all transactions.
    13 => TxOutputs,
    TxId => AmountIndices,

    /// Transaction unlock time.
    ///
    /// Contains the unlock time of transactions IF they have one.
    /// Transactions without unlock times will not exist in this table.
    14 => TxUnlockTime,
    TxId => UnlockTime,

    /// Information on alt-chains.
    15 => AltChainInfos,
    RawChainId => AltChainInfo,

    /// Alt-block heights.
    ///
    /// Contains the height of all alt-blocks.
    16 => AltBlockHeights,
    BlockHash => AltBlockHeight,

    /// Alt-block information.
    ///
    /// Contains information on all alt-blocks.
    17 => AltBlocksInfo,
    AltBlockHeight => CompactAltBlockInfo,

    /// Alt-block blobs.
    ///
    /// Contains the raw bytes of all alt-blocks.
    18 => AltBlockBlobs,
    AltBlockHeight => BlockBlob,

    /// Alt-block transaction blobs.
    ///
    /// Contains the raw bytes of alt transactions, if those transactions are not in the main-chain.
    19 => AltTransactionBlobs,
    TxHash => TxBlob,

    /// Alt-block transaction information.
    ///
    /// Contains information on all alt transactions, even if they are in the main-chain.
    20 => AltTransactionInfos,
    TxHash => AltTransactionInfo,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
