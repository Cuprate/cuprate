//! Database tables.
//!
//! This module contains all the table definitions used by `cuprate-database`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    database::{DatabaseIter, DatabaseRo, DatabaseRw},
    table::Table,
    types::{
        Amount, AmountIndex, AmountIndices, BlockBlob, BlockHash, BlockHeight, BlockInfo, KeyImage,
        Output, PreRctOutputId, PrunableBlob, PrunableHash, PrunedBlob, RctOutput, TxBlob, TxHash,
        TxId, UnlockTime,
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

//---------------------------------------------------------------------------------------------------- `trait Tables[Mut]`
/// Creates:
/// - `pub trait Tables`
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
            fn all_tables_empty(&self) -> Result<bool, $crate::error::RuntimeError>;
        }

        /// Object containing all opened [`Table`]s in read + iter mode.
        ///
        /// This is the same as [`Tables`] but includes `_iter()` variants.
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

            fn all_tables_empty(&self) -> Result<bool, $crate::error::RuntimeError> {
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

// Format: $table_type => $index
//
// The $index:
// - Simply increments by 1 for each table
// - Must be 0..
// - Must end at the total amount of table types
//
// Compile errors will occur if these aren't satisfied.
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
    TxUnlockTime => 13,
}

//---------------------------------------------------------------------------------------------------- Table function macro
/// `crate`-private macro for callings functions on all tables.
///
/// This calls the function `$fn` with the optional
/// arguments `$args` on all tables - returning early
/// (within whatever scope this is called) if any
/// of the function calls error.
///
/// Else, it evaluates to an `Ok((tuple, of, all, table, types, ...))`,
/// i.e., an `impl Table[Mut]` wrapped in `Ok`.
macro_rules! call_fn_on_all_tables_or_early_return {
    (
        $($fn:ident $(::)?)*
        (
            $($arg:ident),* $(,)?
        )
    ) => {{
        Ok((
            $($fn ::)*<$crate::tables::BlockInfos>($($arg),*)?,
            $($fn ::)*<$crate::tables::BlockBlobs>($($arg),*)?,
            $($fn ::)*<$crate::tables::BlockHeights>($($arg),*)?,
            $($fn ::)*<$crate::tables::KeyImages>($($arg),*)?,
            $($fn ::)*<$crate::tables::NumOutputs>($($arg),*)?,
            $($fn ::)*<$crate::tables::PrunedTxBlobs>($($arg),*)?,
            $($fn ::)*<$crate::tables::PrunableHashes>($($arg),*)?,
            $($fn ::)*<$crate::tables::Outputs>($($arg),*)?,
            $($fn ::)*<$crate::tables::PrunableTxBlobs>($($arg),*)?,
            $($fn ::)*<$crate::tables::RctOutputs>($($arg),*)?,
            $($fn ::)*<$crate::tables::TxBlobs>($($arg),*)?,
            $($fn ::)*<$crate::tables::TxIds>($($arg),*)?,
            $($fn ::)*<$crate::tables::TxHeights>($($arg),*)?,
            $($fn ::)*<$crate::tables::TxUnlockTime>($($arg),*)?,
        ))
    }};
}
pub(crate) use call_fn_on_all_tables_or_early_return;

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
// - If adding/changing a table also edit:
//   a) the tests in `src/backend/tests.rs`
//   b) `Env::open` to make sure it creates the table (for all backends)
//   c) `call_fn_on_all_tables_or_early_return!()` macro defined in this file
tables! {
    /// TODO
    BlockBlobs,
    BlockHeight => BlockBlob,

    /// TODO
    BlockHeights,
    BlockHash => BlockHeight,

    /// TODO
    BlockInfos,
    BlockHeight => BlockInfo,

    /// TODO
    KeyImages,
    KeyImage => (),

    /// Maps an output's amount to the number of outputs with that amount.
    ///
    /// For a new output the `AmountIndex` value from this
    /// table will be its index in a list of duplicate outputs.
    NumOutputs,
    Amount => AmountIndex,

    /// TODO
    PrunedTxBlobs,
    TxId => PrunedBlob,

    /// TODO
    Outputs,
    PreRctOutputId => Output,

    // SOMEDAY: impl when `monero-serai` supports pruning
    PrunableTxBlobs,
    TxId => PrunableBlob,

    // SOMEDAY: impl when `monero-serai` supports pruning
    PrunableHashes,
    TxId => PrunableHash,

    // SOMEDAY: impl a properties table:
    // - db version
    // - pruning seed
    // Properties,
    // StorableString => StorableVec,

    /// TODO
    RctOutputs,
    AmountIndex => RctOutput,

    /// SOMEDAY: remove when `monero-serai` supports pruning
    TxBlobs,
    TxId => TxBlob,

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
