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
//! For example: [`BlockHeaderBlobs`] -> `block_header_blobs`.
//!
//! # Traits
//! This module also contains a set of traits for
//! accessing _all_ tables defined here at once.

use cuprate_database::StorableStr;
//---------------------------------------------------------------------------------------------------- Import
use crate::types::{AltBlockHeight, AltChainInfo, AltTransactionInfo, Amount, AmountIndex, AmountIndices, BlobTapeEnd, BlockBlob, BlockHash, BlockHeaderBlob, BlockHeight, BlockInfo, BlockTxHashes, CompactAltBlockInfo, KeyImage, Output, PreRctOutputId, PreRctOutputTableKey, PrunableBlob, PrunableHash, PrunedBlob, RawChainId, RctOutput, TxBlob, TxHash, TxId, TxInfo, UnlockTime};

//---------------------------------------------------------------------------------------------------- Tables
// Notes:
// - Keep this sorted A-Z (by table name)
// - Tables are defined in plural to avoid name conflicts with types
// - If adding/changing a table also edit:
//   - the tests in `src/backend/tests.rs`
cuprate_database::define_tables! {
    /// Block heights.
    ///
    /// Contains the height of all blocks.
    0 => BlockHeights,
    BlockHash => BlockHeight,

    /// Set of key images.
    ///
    /// Contains all the key images known to be spent.
    ///
    /// This table has `()` as the value type, as in,
    /// it is a set of key images.
    1 => KeyImages,
    KeyImage => (),

    /// Maps an output's amount to the number of outputs with that amount.
    ///
    /// For example, if there are 5 outputs with `amount = 123`
    /// then calling `get(123)` on this table will return 5.
    2 => NumOutputs,
    Amount => u64,

    /// Pre-RCT output data.
    3 => Outputs,
    PreRctOutputTableKey => Output,

    // SOMEDAY: impl a properties table:
    // - db version
    // - pruning seed
    // Properties,
    // StorableString => StorableVec,


    /// Transaction indices.
    ///
    /// Contains the indices all transactions.
    4 => TxIds,
    TxHash => TxId,

    /// Transaction outputs.
    ///
    /// Contains the list of `AmountIndex`'s of the
    /// outputs associated with all transactions.
    5 => TxOutputs,
    TxId => AmountIndices,

    /// Transaction unlock time.
    ///
    /// Contains the unlock time of transactions IF they have one.
    /// Transactions without unlock times will not exist in this table.
    6 => TxUnlockTime,
    TxId => UnlockTime,

    /// Information on alt-chains.
    7 => AltChainInfos,
    RawChainId => AltChainInfo,

    /// Alt-block heights.
    ///
    /// Contains the height of all alt-blocks.
    8 => AltBlockHeights,
    BlockHash => AltBlockHeight,

    /// Alt-block information.
    ///
    /// Contains information on all alt-blocks.
    9 => AltBlocksInfo,
    AltBlockHeight => CompactAltBlockInfo,

    /// Alt-block blobs.
    ///
    /// Contains the raw bytes of all alt-blocks.
    10 => AltBlockBlobs,
    AltBlockHeight => BlockBlob,

    /// Alt-block transaction blobs.
    ///
    /// Contains the raw bytes of alt transactions, if those transactions are not in the main-chain.
    11 => AltTransactionBlobs,
    TxHash => TxBlob,

    /// Alt-block transaction information.
    ///
    /// Contains information on all alt transactions, even if they are in the main-chain.
    12 => AltTransactionInfos,
    TxHash => AltTransactionInfo,

    13 => BlobTapeEnds,
    u8 => BlobTapeEnd,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
