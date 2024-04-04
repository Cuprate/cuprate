//! Database tables.
//!
//! This module contains all the table definitions used by `cuprate-database`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    database::{DatabaseIter, DatabaseRo, DatabaseRw},
    table::Table,
    types::{
        Amount, AmountIndex, AmountIndices, BlockBlob, BlockHash, BlockHeight, BlockInfoV1,
        BlockInfoV2, BlockInfoV3, KeyImage, Output, PreRctOutputId, PrunableBlob, PrunableHash,
        PrunedBlob, RctOutput, TxHash, TxId, UnlockTime,
    },
};

//---------------------------------------------------------------------------------------------------- Tables
/// TODO
#[allow(missing_docs)]
pub trait Tables {
    fn block_info_v1s(&self) -> &(impl DatabaseRo<BlockInfoV1s> + DatabaseIter<BlockInfoV1s>);
    fn block_info_v2s(&self) -> &(impl DatabaseRo<BlockInfoV2s> + DatabaseIter<BlockInfoV2s>);
    fn block_info_v3s(&self) -> &(impl DatabaseRo<BlockInfoV3s> + DatabaseIter<BlockInfoV3s>);
    fn block_blobs(&self) -> &(impl DatabaseRo<BlockBlobs> + DatabaseIter<BlockBlobs>);
    fn block_heights(&self) -> &(impl DatabaseRo<BlockHeights> + DatabaseIter<BlockHeights>);
    fn key_images(&self) -> &(impl DatabaseRo<KeyImages> + DatabaseIter<KeyImages>);
    fn num_outputs(&self) -> &(impl DatabaseRo<NumOutputs> + DatabaseIter<NumOutputs>);
    fn pruned_tx_blobs(&self) -> &(impl DatabaseRo<PrunedTxBlobs> + DatabaseIter<PrunedTxBlobs>);
    fn prunable_hashes(&self) -> &(impl DatabaseRo<PrunableHashes> + DatabaseIter<PrunableHashes>);
    fn outputs(&self) -> &(impl DatabaseRo<Outputs> + DatabaseIter<Outputs>);
    fn prunable_tx_blobs(
        &self,
    ) -> &(impl DatabaseRo<PrunableTxBlobs> + DatabaseIter<PrunableTxBlobs>);
    fn rct_outputs(&self) -> &(impl DatabaseRo<RctOutputs> + DatabaseIter<RctOutputs>);
    fn tx_ids(&self) -> &(impl DatabaseRo<TxIds> + DatabaseIter<TxIds>);
    fn tx_heights(&self) -> &(impl DatabaseRo<TxHeights> + DatabaseIter<TxHeights>);
    fn tx_unlock_time(&self) -> &(impl DatabaseRo<TxUnlockTime> + DatabaseIter<TxUnlockTime>);
}

impl<A, B, C, D, E, F, G, H, I, J, K, L, M, N, O> Tables
    for (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O)
where
    A: DatabaseRo<BlockInfoV1s> + DatabaseIter<BlockInfoV1s>,
    B: DatabaseRo<BlockInfoV2s> + DatabaseIter<BlockInfoV2s>,
    C: DatabaseRo<BlockInfoV3s> + DatabaseIter<BlockInfoV3s>,
    D: DatabaseRo<BlockBlobs> + DatabaseIter<BlockBlobs>,
    E: DatabaseRo<BlockHeights> + DatabaseIter<BlockHeights>,
    F: DatabaseRo<KeyImages> + DatabaseIter<KeyImages>,
    G: DatabaseRo<NumOutputs> + DatabaseIter<NumOutputs>,
    H: DatabaseRo<PrunedTxBlobs> + DatabaseIter<PrunedTxBlobs>,
    I: DatabaseRo<PrunableHashes> + DatabaseIter<PrunableHashes>,
    J: DatabaseRo<Outputs> + DatabaseIter<Outputs>,
    K: DatabaseRo<PrunableTxBlobs> + DatabaseIter<PrunableTxBlobs>,
    L: DatabaseRo<RctOutputs> + DatabaseIter<RctOutputs>,
    M: DatabaseRo<TxIds> + DatabaseIter<TxIds>,
    N: DatabaseRo<TxHeights> + DatabaseIter<TxHeights>,
    O: DatabaseRo<TxUnlockTime> + DatabaseIter<TxUnlockTime>,
{
    fn block_info_v1s(&self) -> &(impl DatabaseRo<BlockInfoV1s> + DatabaseIter<BlockInfoV1s>) {
        &self.0
    }
    fn block_info_v2s(&self) -> &(impl DatabaseRo<BlockInfoV2s> + DatabaseIter<BlockInfoV2s>) {
        &self.1
    }
    fn block_info_v3s(&self) -> &(impl DatabaseRo<BlockInfoV3s> + DatabaseIter<BlockInfoV3s>) {
        &self.2
    }
    fn block_blobs(&self) -> &(impl DatabaseRo<BlockBlobs> + DatabaseIter<BlockBlobs>) {
        &self.3
    }
    fn block_heights(&self) -> &(impl DatabaseRo<BlockHeights> + DatabaseIter<BlockHeights>) {
        &self.4
    }
    fn key_images(&self) -> &(impl DatabaseRo<KeyImages> + DatabaseIter<KeyImages>) {
        &self.5
    }
    fn num_outputs(&self) -> &(impl DatabaseRo<NumOutputs> + DatabaseIter<NumOutputs>) {
        &self.6
    }
    fn pruned_tx_blobs(&self) -> &(impl DatabaseRo<PrunedTxBlobs> + DatabaseIter<PrunedTxBlobs>) {
        &self.7
    }
    fn prunable_hashes(&self) -> &(impl DatabaseRo<PrunableHashes> + DatabaseIter<PrunableHashes>) {
        &self.8
    }
    fn outputs(&self) -> &(impl DatabaseRo<Outputs> + DatabaseIter<Outputs>) {
        &self.9
    }
    fn prunable_tx_blobs(
        &self,
    ) -> &(impl DatabaseRo<PrunableTxBlobs> + DatabaseIter<PrunableTxBlobs>) {
        &self.10
    }
    fn rct_outputs(&self) -> &(impl DatabaseRo<RctOutputs> + DatabaseIter<RctOutputs>) {
        &self.11
    }
    fn tx_ids(&self) -> &(impl DatabaseRo<TxIds> + DatabaseIter<TxIds>) {
        &self.12
    }
    fn tx_heights(&self) -> &(impl DatabaseRo<TxHeights> + DatabaseIter<TxHeights>) {
        &self.13
    }
    fn tx_unlock_time(&self) -> &(impl DatabaseRo<TxUnlockTime> + DatabaseIter<TxUnlockTime>) {
        &self.14
    }
}

//---------------------------------------------------------------------------------------------------- TablesMut
/// TODO
#[allow(missing_docs)]
pub trait TablesMut {
    fn block_info_v1s_mut(&mut self) -> &mut impl DatabaseRw<BlockInfoV1s>;
    fn block_info_v2s_mut(&mut self) -> &mut impl DatabaseRw<BlockInfoV2s>;
    fn block_info_v3s_mut(&mut self) -> &mut impl DatabaseRw<BlockInfoV3s>;
    fn block_blobs_mut(&mut self) -> &mut impl DatabaseRw<BlockBlobs>;
    fn block_heights_mut(&mut self) -> &mut impl DatabaseRw<BlockHeights>;
    fn key_images_mut(&mut self) -> &mut impl DatabaseRw<KeyImages>;
    fn num_outputs_mut(&mut self) -> &mut impl DatabaseRw<NumOutputs>;
    fn pruned_tx_blobs_mut(&mut self) -> &mut impl DatabaseRw<PrunedTxBlobs>;
    fn prunable_hashes_mut(&mut self) -> &mut impl DatabaseRw<PrunableHashes>;
    fn outputs_mut(&mut self) -> &mut impl DatabaseRw<Outputs>;
    fn prunable_tx_blobs_mut(&mut self) -> &mut impl DatabaseRw<PrunableTxBlobs>;
    fn rct_outputs_mut(&mut self) -> &mut impl DatabaseRw<RctOutputs>;
    fn tx_ids_mut(&mut self) -> &mut impl DatabaseRw<TxIds>;
    fn tx_heights_mut(&mut self) -> &mut impl DatabaseRw<TxHeights>;
    fn tx_unlock_time_mut(&mut self) -> &mut impl DatabaseRw<TxUnlockTime>;
}

impl<A, B, C, D, E, F, G, H, I, J, K, L, M, N, O> TablesMut
    for (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O)
where
    A: DatabaseRw<BlockInfoV1s>,
    B: DatabaseRw<BlockInfoV2s>,
    C: DatabaseRw<BlockInfoV3s>,
    D: DatabaseRw<BlockBlobs>,
    E: DatabaseRw<BlockHeights>,
    F: DatabaseRw<KeyImages>,
    G: DatabaseRw<NumOutputs>,
    H: DatabaseRw<PrunedTxBlobs>,
    I: DatabaseRw<PrunableHashes>,
    J: DatabaseRw<Outputs>,
    K: DatabaseRw<PrunableTxBlobs>,
    L: DatabaseRw<RctOutputs>,
    M: DatabaseRw<TxIds>,
    N: DatabaseRw<TxHeights>,
    O: DatabaseRw<TxUnlockTime>,
{
    fn block_info_v1s_mut(&mut self) -> &mut impl DatabaseRw<BlockInfoV1s> {
        &mut self.0
    }
    fn block_info_v2s_mut(&mut self) -> &mut impl DatabaseRw<BlockInfoV2s> {
        &mut self.1
    }
    fn block_info_v3s_mut(&mut self) -> &mut impl DatabaseRw<BlockInfoV3s> {
        &mut self.2
    }
    fn block_blobs_mut(&mut self) -> &mut impl DatabaseRw<BlockBlobs> {
        &mut self.3
    }
    fn block_heights_mut(&mut self) -> &mut impl DatabaseRw<BlockHeights> {
        &mut self.4
    }
    fn key_images_mut(&mut self) -> &mut impl DatabaseRw<KeyImages> {
        &mut self.5
    }
    fn num_outputs_mut(&mut self) -> &mut impl DatabaseRw<NumOutputs> {
        &mut self.6
    }
    fn pruned_tx_blobs_mut(&mut self) -> &mut impl DatabaseRw<PrunedTxBlobs> {
        &mut self.7
    }
    fn prunable_hashes_mut(&mut self) -> &mut impl DatabaseRw<PrunableHashes> {
        &mut self.8
    }
    fn outputs_mut(&mut self) -> &mut impl DatabaseRw<Outputs> {
        &mut self.9
    }
    fn prunable_tx_blobs_mut(&mut self) -> &mut impl DatabaseRw<PrunableTxBlobs> {
        &mut self.10
    }
    fn rct_outputs_mut(&mut self) -> &mut impl DatabaseRw<RctOutputs> {
        &mut self.11
    }
    fn tx_ids_mut(&mut self) -> &mut impl DatabaseRw<TxIds> {
        &mut self.12
    }
    fn tx_heights_mut(&mut self) -> &mut impl DatabaseRw<TxHeights> {
        &mut self.13
    }
    fn tx_unlock_time_mut(&mut self) -> &mut impl DatabaseRw<TxUnlockTime> {
        &mut self.14
    }
}

//---------------------------------------------------------------------------------------------------- Tables
/// Private module, should not be accessible outside this crate.
///
/// Used to block outsiders implementing [`Table`].
/// All [`Table`] types must also implement [`Sealed`].
pub(super) mod private {
    /// Private sealed trait.
    ///
    /// Cannot be implemented outside this crate.
    pub trait Sealed {}
}

//---------------------------------------------------------------------------------------------------- Table macro
/// Create all tables, should be used _once_.
///
/// Generating this macro once and using `$()*` is probably
/// faster for compile times than calling the macro _per_ table.
///
/// All tables are zero-sized table structs, and implement the `Table` trait.
///
/// Table structs are automatically `CamelCase`,
/// and their static string names are automatically `snake_case`.
macro_rules! tables {
    (
        $(
            $(#[$attr:meta])* // Documentation and any `derive`'s.
            $table:ident,     // The table name + doubles as the table struct name.
            $key:ty =>        // Key type.
            $value:ty         // Value type.
        ),* $(,)?
    ) => {
        paste::paste! { $(
            // Table struct.
            $(#[$attr])*
            // The below test show the `snake_case` table name in cargo docs.
            /// ## Table Name
            /// ```rust
            /// # use cuprate_database::{*,tables::*};
            #[doc = concat!(
                "assert_eq!(",
                stringify!([<$table:camel>]),
                "::NAME, \"",
                stringify!([<$table:snake>]),
                "\");",
            )]
            /// ```
            #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
            #[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
            pub struct [<$table:camel>];

            // Implement the `Sealed` in this file.
            // Required by `Table`.
            impl private::Sealed for [<$table:camel>] {}

            // Table trait impl.
            impl Table for [<$table:camel>] {
                const NAME: &'static str = stringify!([<$table:snake>]);
                type Key = $key;
                type Value = $value;
            }
        )* }
    };
}

//---------------------------------------------------------------------------------------------------- Tables
// Notes:
// - Keep this sorted A-Z (by table name)
// - Tables are defined in plural to avoid name conflicts with types
// - If adding/changing a table, also edit the tests in `src/backend/tests.rs`
//   and edit `Env::open` to make sure it creates the table
tables! {
    /// TODO
    BlockBlobs,
    BlockHeight => BlockBlob,

    /// TODO
    BlockHeights,
    BlockHash => BlockHeight,

    /// TODO
    BlockInfoV1s,
    BlockHeight => BlockInfoV1,

    /// TODO
    BlockInfoV2s,
    BlockHeight => BlockInfoV2,

    /// TODO
    BlockInfoV3s,
    BlockHeight => BlockInfoV3,

    /// TODO
    KeyImages,
    KeyImage => (),

    /// TODO
    NumOutputs,
    Amount => AmountIndex,

    /// TODO
    PrunedTxBlobs,
    TxId => PrunedBlob,

    /// TODO
    Outputs,
    PreRctOutputId => Output,

    /// TODO
    PrunableTxBlobs,
    TxId => PrunableBlob,

    /// TODO
    PrunableHashes,
    TxId => PrunableHash,

    /// TODO
    RctOutputs,
    AmountIndex => RctOutput,

    /// TODO
    TxIds,
    TxHash => TxId,

    /// TODO
    TxHeights,
    TxId => BlockHeight,

    /// TODO
    TxUnlockTime,
    TxId => UnlockTime,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
