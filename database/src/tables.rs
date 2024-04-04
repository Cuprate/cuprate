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

//---------------------------------------------------------------------------------------------------- Sealed
/// Private module, should not be accessible outside this crate.
pub(super) mod private {
    /// Private sealed trait.
    ///
    /// Cannot be implemented outside this crate.
    pub trait Sealed {}
}

//---------------------------------------------------------------------------------------------------- Tables
/// Creates:
/// - `pub trait Tables`
/// - `pub trait TablesMut`
/// - Blanket implementation for `(tuples, containing, all, open, database, tables, ...)`
macro_rules! define_trait_tables {
    ($(
        // The `T: Table` type     The index in a tuple
        // |                       containing all tables
        // v                         v
        $table:ident => $index:literal
    ),* $(,)?) => { paste::paste! {
        /// Object containing all opened [`Table`]s in read-only mode.
        ///
        /// This is an encapsulated object that contains all
        /// available [`Table`]'s in read-only mode.
        ///
        /// It is a `Sealed` trait and is only implemented on a
        /// `(tuple, containing, all, table, types, ...)`.
        ///
        /// This is used to return a _single_ object from functions like
        /// [`EnvInner::open_tables`](crate::EnvInner::open_tables) rather
        /// than the tuple containing the tables itself.
        ///
        /// To replace `tuple.0` style indexing, `field_accessor_functions()`
        /// are provided on this trait, which essentially map the object to
        /// fields containing the particular database table, for example:
        /// ```rust,ignore
        /// let tables = open_tables();
        ///
        /// // The accessor function `block_info_v1s()` returns the field
        /// // containing an open database table for  `BlockInfoV1s`.
        /// let _ = tables.block_info_v1s();
        /// ```
        #[allow(missing_docs)] // No documentation needed for `field_accessor_functions()`.
        pub trait Tables: private::Sealed {
            // This expands to creating `fn field_accessor_functions()`
            // for each passed `$table` type.
            //
            // It is essentially a mapping to the field
            // containing the proper opened database table.
            //
            // The function name of the function is
            // the table type in `snake_case`, e.g., `block_info_v1s()`.
            $(
                fn [<$table:snake>](&self) -> &(impl DatabaseRo<$table> + DatabaseIter<$table>);
            )*
        }

        /// Object containing all opened [`Table`]s in write mode.
        ///
        /// This is the same as [`Tables`] but for mutable accesses.
        ///
        /// See [`Tables`] for documentation - this trait exists for the same reasons.
        #[allow(missing_docs)]
        pub trait TablesMut: private::Sealed {
            $(
                fn [<$table:snake _mut>](&mut self) -> &mut impl DatabaseRw<$table>;
            )*
        }

        // This creates a blanket-implementation for
        // `(tuple, containing, all, table, types)`.
        //
        // There is a generic defined here _for each_ `$table` input.
        // Specifically, the generic letters are just the table types in UPPERCASE.
        // Concretely, this expands to something like:
        // ```rust
        // impl<BLOCKINFOSV1S, BLOCKINFOSV2S, BLOCKINFOSV3S, [...]>
        // ```
        impl<$([<$table:upper>]),*> Tables
            // We are implementing `Tables` on a tuple that
            // contains all those generics specified, i.e.,
            // a tuple containing all open table types.
            //
            // Concretely, this expands to something like:
            // ```rust
            // (BLOCKINFOSV1S, BLOCKINFOSV2S, BLOCKINFOSV3S, [...])
            // ```
            // which is just a tuple of the generics defined above.
            for ($([<$table:upper>]),*)
        where
            // This expands to a where bound that asserts each element
            // in the tuple implements some database table type.
            //
            // Concretely, this expands to something like:
            // ```rust
            // BLOCKINFOSV1S: DatabaseRo<BlockInfoV1s> + DatabaseIter<BlockInfoV1s>,
            // BLOCKINFOSV2S: DatabaseRo<BlockInfoV2s> + DatabaseIter<BlockInfoV2s>,
            // [...]
            // ```
            $(
                [<$table:upper>]: DatabaseRo<$table> + DatabaseIter<$table>,
            )*
        {
            $(
                // The function name of the accessor function is
                // the table type in `snake_case`, e.g., `block_info_v1s()`.
                #[inline]
                fn [<$table:snake>](&self) -> &(impl DatabaseRo<$table> + DatabaseIter<$table>) {
                    // The index of the database table in
                    // the tuple implements the table trait.
                    &self.$index
                }
            )*
        }

        // This is the same as the above
        // `Tables`, but for `TablesMut`.
        impl<$([<$table:upper>]),*> TablesMut
            for ($([<$table:upper>]),*)
        where
            $(
                [<$table:upper>]: DatabaseRw<$table>,
            )*
        {
            $(
                // The function name of the accessor function is
                // the table type in `snake_case` + `_mut`, e.g., `block_info_v1s_mut()`.
                #[inline]
                fn [<$table:snake _mut>](&mut self) -> &mut impl DatabaseRw<$table> {
                    &mut self.$index
                }
            )*
        }

        impl<$([<$table:upper>]),*> private::Sealed for ($([<$table:upper>]),*) {}
    }};
}

// Format: $table_type => $index
//
// The $index:
// - Simply increments by 1 for each table
// - Must be 0..
// - Must end at the total amount of table types
//
// Compile errors will occur if these aren't satisfied.
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
