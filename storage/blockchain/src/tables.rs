//! Database tables.
//!
//! # Table marker structs
//! This module contains all the table definitions used by `cuprate_blockchain`.
//!
//! The zero-sized structs here represents the table type;
//! they all are essentially marker types that implement [`Table`].
//!
//! Table structs are `CamelCase`, and their static string
//! names used by the actual database backend are `snake_case`.
//!
//! For example: [`BlockBlobs`] -> `block_blobs`.
//!
//! # Traits
//! This module also contains a set of traits for
//! accessing _all_ tables defined here at once.
//!
//! For example, this is the object returned by [`OpenTables::open_tables`](crate::OpenTables::open_tables).

//---------------------------------------------------------------------------------------------------- Import
use cuprate_database::{DatabaseIter, DatabaseRo, DatabaseRw, Table};

use crate::types::{
    Amount, AmountIndex, AmountIndices, BlockBlob, BlockHash, BlockHeight, BlockInfo, KeyImage,
    Output, PreRctOutputId, PrunableBlob, PrunableHash, PrunedBlob, RctOutput, TxBlob, TxHash,
    TxId, UnlockTime,
};

//---------------------------------------------------------------------------------------------------- Sealed
/// Private module, should not be accessible outside this crate.
pub(super) mod private {
    /// Private sealed trait.
    ///
    /// Cannot be implemented outside this crate.
    pub trait Sealed {}
}

//---------------------------------------------------------------------------------------------------- `trait Tables[Mut]`
/// Creates:
/// - `pub trait Tables`
/// - `pub trait TablesIter`
/// - `pub trait TablesMut`
/// - Blanket implementation for `(tuples, containing, all, open, database, tables, ...)`
///
/// For why this exists, see: <https://github.com/Cuprate/cuprate/pull/102#pullrequestreview-1978348871>.
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
        /// [`OpenTables::open_tables`](crate::OpenTables::open_tables) rather
        /// than the tuple containing the tables itself.
        ///
        /// To replace `tuple.0` style indexing, `field_accessor_functions()`
        /// are provided on this trait, which essentially map the object to
        /// fields containing the particular database table, for example:
        /// ```rust,ignore
        /// let tables = open_tables();
        ///
        /// // The accessor function `block_infos()` returns the field
        /// // containing an open database table for `BlockInfos`.
        /// let _ = tables.block_infos();
        /// ```
        ///
        /// See also:
        /// - [`TablesMut`]
        /// - [`TablesIter`]
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
                /// Access an opened
                #[doc = concat!("[`", stringify!($table), "`]")]
                /// database.
                fn [<$table:snake>](&self) -> &impl DatabaseRo<$table>;
            )*

            /// This returns `true` if all tables are empty.
            ///
            /// # Errors
            /// This returns errors on regular database errors.
            fn all_tables_empty(&self) -> Result<bool, cuprate_database::RuntimeError>;
        }

        /// Object containing all opened [`Table`]s in read + iter mode.
        ///
        /// This is the same as [`Tables`] but includes `_iter()` variants.
        ///
        /// Note that this trait is a supertrait of `Tables`,
        /// as in it can use all of its functions as well.
        ///
        /// See [`Tables`] for documentation - this trait exists for the same reasons.
        pub trait TablesIter: private::Sealed + Tables {
            $(
                /// Access an opened read-only + iterable
                #[doc = concat!("[`", stringify!($table), "`]")]
                /// database.
                fn [<$table:snake _iter>](&self) -> &(impl DatabaseRo<$table> + DatabaseIter<$table>);
            )*
        }

        /// Object containing all opened [`Table`]s in write mode.
        ///
        /// This is the same as [`Tables`] but for mutable accesses.
        ///
        /// Note that this trait is a supertrait of `Tables`,
        /// as in it can use all of its functions as well.
        ///
        /// See [`Tables`] for documentation - this trait exists for the same reasons.
        pub trait TablesMut: private::Sealed + Tables {
            $(
                /// Access an opened
                #[doc = concat!("[`", stringify!($table), "`]")]
                /// database.
                fn [<$table:snake _mut>](&mut self) -> &mut impl DatabaseRw<$table>;
            )*
        }

        // Implement `Sealed` for all table types.
        impl<$([<$table:upper>]),*> private::Sealed for ($([<$table:upper>]),*) {}

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
                [<$table:upper>]: DatabaseRo<$table>,
            )*
        {
            $(
                // The function name of the accessor function is
                // the table type in `snake_case`, e.g., `block_info_v1s()`.
                #[inline]
                fn [<$table:snake>](&self) -> &impl DatabaseRo<$table> {
                    // The index of the database table in
                    // the tuple implements the table trait.
                    &self.$index
                }
            )*

            fn all_tables_empty(&self) -> Result<bool, cuprate_database::RuntimeError> {
                $(
                     if !DatabaseRo::is_empty(&self.$index)? {
                        return Ok(false);
                     }
                )*
                Ok(true)
            }
        }

        // This is the same as the above
        // `Tables`, but for `TablesIter`.
        impl<$([<$table:upper>]),*> TablesIter
            for ($([<$table:upper>]),*)
        where
            $(
                [<$table:upper>]: DatabaseRo<$table> + DatabaseIter<$table>,
            )*
        {
            $(
                // The function name of the accessor function is
                // the table type in `snake_case` + `_iter`, e.g., `block_info_v1s_iter()`.
                #[inline]
                fn [<$table:snake _iter>](&self) -> &(impl DatabaseRo<$table> + DatabaseIter<$table>) {
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
                // The function name of the mutable accessor function is
                // the table type in `snake_case` + `_mut`, e.g., `block_info_v1s_mut()`.
                #[inline]
                fn [<$table:snake _mut>](&mut self) -> &mut impl DatabaseRw<$table> {
                    &mut self.$index
                }
            )*
        }
    }};
}

// Input format: $table_type => $index
//
// The $index:
// - Simply increments by 1 for each table
// - Must be 0..
// - Must end at the total amount of table types - 1
//
// Compile errors will occur if these aren't satisfied.
//
// $index is just the `tuple.$index`, as the above [`define_trait_tables`]
// macro has a blanket impl for `(all, table, types, ...)` and we must map
// each type to a tuple index explicitly.
//
// FIXME: there's definitely an automatic way to this :)
define_trait_tables! {
    BlockInfos => 0,
    BlockBlobs => 1,
    BlockHeights => 2,
    KeyImages => 3,
    NumOutputs => 4,
    PrunedTxBlobs => 5,
    PrunableHashes => 6,
    Outputs => 7,
    PrunableTxBlobs => 8,
    RctOutputs => 9,
    TxBlobs => 10,
    TxIds => 11,
    TxHeights => 12,
    TxOutputs => 13,
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
            #[doc = concat!("- Key: [`", stringify!($key), "`]")]
            #[doc = concat!("- Value: [`", stringify!($value), "`]")]
            ///
            /// ## Table Name
            /// ```rust
            /// # use cuprate_blockchain::{*,tables::*};
            /// use cuprate_database::Table;
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
// - If adding/changing a table also edit:
//   a) the tests in `src/backend/tests.rs`
//   b) `Env::open` to make sure it creates the table (for all backends)
//   c) `call_fn_on_all_tables_or_early_return!()` macro defined in this file
tables! {
    /// Serialized block blobs (bytes).
    ///
    /// Contains the serialized version of all blocks.
    BlockBlobs,
    BlockHeight => BlockBlob,

    /// Block heights.
    ///
    /// Contains the height of all blocks.
    BlockHeights,
    BlockHash => BlockHeight,

    /// Block information.
    ///
    /// Contains metadata of all blocks.
    BlockInfos,
    BlockHeight => BlockInfo,

    /// Set of key images.
    ///
    /// Contains all the key images known to be spent.
    ///
    /// This table has `()` as the value type, as in,
    /// it is a set of key images.
    KeyImages,
    KeyImage => (),

    /// Maps an output's amount to the number of outputs with that amount.
    ///
    /// For example, if there are 5 outputs with `amount = 123`
    /// then calling `get(123)` on this table will return 5.
    NumOutputs,
    Amount => u64,

    /// Pre-RCT output data.
    Outputs,
    PreRctOutputId => Output,

    /// Pruned transaction blobs (bytes).
    ///
    /// Contains the pruned portion of serialized transaction data.
    PrunedTxBlobs,
    TxId => PrunedBlob,

    /// Prunable transaction blobs (bytes).
    ///
    /// Contains the prunable portion of serialized transaction data.
    // SOMEDAY: impl when `monero-serai` supports pruning
    PrunableTxBlobs,
    TxId => PrunableBlob,

    /// Prunable transaction hashes.
    ///
    /// Contains the prunable portion of transaction hashes.
    // SOMEDAY: impl when `monero-serai` supports pruning
    PrunableHashes,
    TxId => PrunableHash,

    // SOMEDAY: impl a properties table:
    // - db version
    // - pruning seed
    // Properties,
    // StorableString => StorableVec,

    /// RCT output data.
    RctOutputs,
    AmountIndex => RctOutput,

    /// Transaction blobs (bytes).
    ///
    /// Contains the serialized version of all transactions.
    // SOMEDAY: remove when `monero-serai` supports pruning
    TxBlobs,
    TxId => TxBlob,

    /// Transaction indices.
    ///
    /// Contains the indices all transactions.
    TxIds,
    TxHash => TxId,

    /// Transaction heights.
    ///
    /// Contains the block height associated with all transactions.
    TxHeights,
    TxId => BlockHeight,

    /// Transaction outputs.
    ///
    /// Contains the list of `AmountIndex`'s of the
    /// outputs associated with all transactions.
    TxOutputs,
    TxId => AmountIndices,

    /// Transaction unlock time.
    ///
    /// Contains the unlock time of transactions IF they have one.
    /// Transactions without unlock times will not exist in this table.
    TxUnlockTime,
    TxId => UnlockTime,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
