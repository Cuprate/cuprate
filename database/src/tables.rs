//! Database tables.
//!
//! This module contains all the table definitions used by `cuprate-database`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    table::Table,
    types::{
        Amount, AmountIndex, AmountIndices, BlockBlob, BlockHash, BlockHeight, BlockInfoV1,
        BlockInfoV2, BlockInfoV3, KeyImage, Output, PrunableBlob, PrunableHash, PrunedBlob,
        RctOutput, TxHash, TxId, UnlockTime,
    },
};

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
// - Keep this sorted A-Z
// - Tables are defined in plural to avoid name conflicts with types
// - If adding/changing a table, also edit the tests in `src/backend/tests.rs`
tables! {
    /// TODO
    TxIds,
    TxHash => TxId,

    /// TODO
    TxHeights,
    TxId => BlockHeight,

    /// TODO
    TxUnlockTime,
    TxId => UnlockTime,

    /// TODO
    PrunedTxBlobs,
    TxId => PrunedBlob,

    /// TODO
    PrunableTxBlobs,
    TxId => PrunableBlob,

    /// TODO
    PrunableHashes,
    TxId => PrunableHash,

    /// TODO
    Outputs,
    Amount => Output, // FIXME: `Amount | AmountIndex` key

    /// TODO
    RctOutputs,
    AmountIndex => RctOutput,

    /// TODO
    KeyImages,
    KeyImage => (),

    /// TODO
    BlockHeights,
    BlockHash => BlockHeight,

    /// TODO
    BlockBlobs,
    BlockHeight => BlockBlob,

    /// TODO
    BlockInfoV1s,
    BlockHeight => BlockInfoV1,

    /// TODO
    BlockInfoV2s,
    BlockHeight => BlockInfoV2,

    /// TODO
    BlockInfoV3s,
    BlockHeight => BlockInfoV3,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
