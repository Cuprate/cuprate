//! Database tables.
//!
//! # Table marker structs
//! This module contains all the table definitions used by `cuprate_blockchain`.
//!
//! The zero-sized structs here represents the table type;
//! they all are essentially marker types that implement `Table`.
//!
//! Table structs are `CamelCase`, and their static string
//! names used by the actual database backend are `snake_case`.
//!
//! For example: `BlockBlobs` -> `block_blobs`.
//!
//! # Traits
//! This module also contains a set of traits for
//! accessing _all_ tables defined here at once.
//!
//! For example, this is the object returned by `OpenTables::open_tables`.

//---------------------------------------------------------------------------------------------------- Import

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
///
/// // --- TODO
///
/// Creates:
/// - `pub trait Tables`
/// - `pub trait TablesIter`
/// - `pub trait TablesMut`
/// - Blanket implementation for `(tuples, containing, all, open, database, tables, ...)`
///
/// For why this exists, see: <https://github.com/Cuprate/cuprate/pull/102#pullrequestreview-1978348871>.
#[macro_export]
macro_rules! define_tables {
    (
        $(
            $(#[$attr:meta])* // Documentation and any `derive`'s.
            $index:literal => $table:ident, // The table name + doubles as the table struct name.
            $key:ty =>        // Key type.
            $value:ty         // Value type.
        ),* $(,)?
    ) => { $crate::paste::paste! {
        $(
            // Table struct.
            $(#[$attr])*
            // The below test show the `snake_case` table name in cargo docs.
            #[doc = concat!("- Key: [`", stringify!($key), "`]")]
            #[doc = concat!("- Value: [`", stringify!($value), "`]")]
            #[doc = concat!("- Name: `", stringify!([<$table:snake>]), "`")]
            #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
            #[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
            pub struct [<$table:camel>];

            // Table trait impl.
            impl $crate::Table for [<$table:camel>] {
                const NAME: &'static str = stringify!([<$table:snake>]);
                type Key = $key;
                type Value = $value;
            }
        )*

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
        pub trait Tables {
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
        pub trait TablesIter: Tables {
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
        pub trait TablesMut: Tables {
            $(
                /// Access an opened
                #[doc = concat!("[`", stringify!($table), "`]")]
                /// database.
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

        /// Open all tables at once.
        ///
        /// This trait encapsulates the functionality of opening all tables at once.
        /// It can be seen as the "constructor" for the `Tables` object.
        ///
        /// Note that this is already implemented on [`cuprate_database::EnvInner`], thus:
        /// - You don't need to implement this
        /// - It can be called using `env_inner.open_tables()` notation
        pub trait OpenTables<'env> {
            /// The read-only transaction type of the backend.
            type Ro<'a>;
            /// The read-write transaction type of the backend.
            type Rw<'a>;

            /// Open all tables in read/iter mode.
            ///
            /// This calls [`EnvInner::open_db_ro`] on all database tables
            /// and returns a structure that allows access to all tables.
            ///
            /// # Errors
            /// This will only return [`cuprate_database::RuntimeError::Io`] if it errors.
            ///
            /// # Invariant
            /// All tables should be created with a crate-specific open function.
            ///
            /// TODO: explain why
            fn open_tables(&self, tx_ro: &Self::Ro<'_>) -> Result<impl TablesIter, $crate::RuntimeError>;

            /// Open all tables in read-write mode.
            ///
            /// This calls [`cuprate_database::EnvInner::open_db_rw`] on all database tables
            /// and returns a structure that allows access to all tables.
            ///
            /// # Errors
            /// This will only return [`cuprate_database::RuntimeError::Io`] on errors.
            fn open_tables_mut(&self, tx_rw: &Self::Rw<'_>) -> Result<impl TablesMut, $crate::RuntimeError>;

            /// Create all database tables.
            ///
            /// This will create all the defined [`Table`](cuprate_database::Table)s.
            ///
            /// # Errors
            /// This will only return [`cuprate_database::RuntimeError::Io`] on errors.
            fn create_tables(&self, tx_rw: &Self::Rw<'_>) -> Result<(), $crate::RuntimeError>;
        }

        impl<'env, Ei> OpenTables<'env> for Ei
        where
            Ei: $crate::EnvInner<'env>,
        {
            type Ro<'a> = <Ei as $crate::EnvInner<'env>>::Ro<'a>;
            type Rw<'a> = <Ei as $crate::EnvInner<'env>>::Rw<'a>;

            fn open_tables(&self, tx_ro: &Self::Ro<'_>) -> Result<impl TablesIter, $crate::RuntimeError> {
                Ok(($(
                    Self::open_db_ro::<[<$table:camel>]>(self, tx_ro)?,
                )*))
            }

            fn open_tables_mut(&self, tx_rw: &Self::Rw<'_>) -> Result<impl TablesMut, $crate::RuntimeError> {
                Ok(($(
                    Self::open_db_rw::<[<$table:camel>]>(self, tx_rw)?,
                )*))
            }

            fn create_tables(&self, tx_rw: &Self::Rw<'_>) -> Result<(), $crate::RuntimeError> {
                let result = Ok(($(
                    Self::create_db::<[<$table:camel>]>(self, tx_rw),
                )*));

                match result {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e),
                }
            }
        }
    }};
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
