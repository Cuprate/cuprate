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
macro_rules! define_trait_tables {
    ($(
        $table:ident => $index:literal
    ),* $(,)?) => { paste::paste! {
        /// TODO
        #[allow(missing_docs)]
        pub trait Tables {
            $(
                fn [<$table:snake>](&self) -> &(impl DatabaseRo<$table> + DatabaseIter<$table>);
            )*
        }

        /// TODO
        #[allow(missing_docs)]
        pub trait TablesMut {
            $(
                fn [<$table:snake _mut>](&mut self) -> &mut impl DatabaseRw<$table>;
            )*
        }

        impl<$([<$table:upper>]),*> Tables
            for ($([<$table:upper>]),*)
        where
            $(
                [<$table:upper>]: DatabaseRo<$table> + DatabaseIter<$table>,
            )*
        {
            $(
                fn [<$table:snake>](&self) -> &(impl DatabaseRo<$table> + DatabaseIter<$table>) {
                    &self.$index
                }
            )*
        }

        impl<$([<$table:upper>]),*> TablesMut
            for ($([<$table:upper>]),*)
        where
            $(
                [<$table:upper>]: DatabaseRw<$table>,
            )*
        {
            $(
                fn [<$table:snake _mut>](&mut self) -> &mut impl DatabaseRw<$table> {
                    &mut self.$index
                }
            )*
        }
    }};
}

define_trait_tables! {
    BlockInfoV1s => 0,
    BlockInfoV2s => 1,
    BlockInfoV3s => 2,
    BlockBlobs => 3,
    BlockHeights => 4,
    KeyImages => 5,
    NumOutputs => 6,
    PrunedTxBlobs => 7,
    PrunableHashes => 8,
    Outputs => 9,
    PrunableTxBlobs => 10,
    RctOutputs => 11,
    TxIds => 12,
    TxHeights => 13,
    TxUnlockTime => 14,
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
