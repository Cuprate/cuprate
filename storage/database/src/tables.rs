//! Database table definition macro.

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Table macro
/// Define all table types.
///
/// # Purpose
/// This macro allows you to define all database tables in one place.
///
/// A by-product of this macro is that it defines some
/// convenient traits specific to _your_ tables
/// (see [Output](#output)).
///
/// # Inputs
/// This macro expects a list of tables, and their key/value types.
///
/// This syntax is as follows:
///
/// ```rust
/// cuprate_database::define_tables! {
///     /// Any extra attributes you'd like to add to
///     /// this table type, e.g. docs or derives.
///
///     0 => TableName,
/// //  ▲    ▲
/// //  │    └─ Table struct name. The macro generates this for you.
/// //  │
/// // Incrementing index. This must start at 0
/// // and increment by 1 per table added.
///
///     u8 => u64,
/// //  ▲    ▲
/// //  │    └─ Table value type.
/// //  │
/// // Table key type.
///
///    // Another table.
///    1 => TableName2,
///    i8 => i64,
/// }
/// ```
///
/// An example:
/// ```rust
/// use cuprate_database::{
///     Table,
///     config::ConfigBuilder,
///     Env, EnvInner,
///     DatabaseRo, DatabaseRw, TxRo, TxRw,
/// };
///
/// #[cfg(feature = "heed")]
/// use cuprate_database::HeedEnv as ConcreteEnv;
/// #[cfg(all(feature = "redb", not(feature = "heed")))]
/// use cuprate_database::RedbEnv as ConcreteEnv;
///
/// // This generates `pub struct Table{1,2,3}`
/// // where all those implement `Table` with
/// // the defined name and key/value types.
/// //
/// // It also generate traits specific to our tables.
/// cuprate_database::define_tables! {
///     0 => Table1,
///     u32 => i32,
///
///     /// This one has extra docs.
///     1 => Table2,
///     u64 => (),
///
///     2 => Table3,
///     i32 => i32,
/// }
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let tmp_dir = tempfile::tempdir()?;
/// # let db_dir = tmp_dir.path().to_owned();
/// # let config = ConfigBuilder::new(db_dir.into()).build();
/// // Open the database.
/// let env = ConcreteEnv::open(config)?;
/// let env_inner = env.env_inner();
///
/// // Open the table we just defined.
/// {
///     let tx_rw = env_inner.tx_rw()?;
///     env_inner.create_db::<Table1>(&tx_rw)?;
///     let mut table = env_inner.open_db_rw::<Table1>(&tx_rw)?;
///
///     // Write data to the table.
///     table.put(&0, &1)?;
///
///     drop(table);
///     TxRw::commit(tx_rw)?;
/// }
///
/// // Read the data, assert it is correct.
/// {
///     let tx_ro = env_inner.tx_ro()?;
///     let table = env_inner.open_db_ro::<Table1>(&tx_ro)?;
///     assert_eq!(table.first()?, (0, 1));
/// }
///
/// // Create all tables at once using the
/// // `OpenTables` trait generated with the
/// // macro above.
/// {
///     let tx_rw = env_inner.tx_rw()?;
///     env_inner.create_tables(&tx_rw)?;
///     TxRw::commit(tx_rw)?;
/// }
///
/// // Open all tables at once.
/// {
///     let tx_ro = env_inner.tx_ro()?;
///     let all_tables = env_inner.open_tables(&tx_ro)?;
/// }
/// # Ok(()) }
/// ```
///
/// # Output
/// This macro:
/// 1. Implements [`Table`](crate::Table) on all your table types
/// 1. Creates a `pub trait Tables` trait (in scope)
/// 1. Creates a `pub trait TablesIter` trait (in scope)
/// 1. Creates a `pub trait TablesMut` trait (in scope)
/// 1. Blanket implements a `(tuples, containing, all, open, database, tables, ...)` for the above traits
/// 1. Creates a `pub trait OpenTables` trait (in scope)
///
/// All table types are zero-sized structs that implement the `Table` trait.
///
/// Table structs are automatically `CamelCase`, and their
/// static string names are automatically `snake_case`.
///
/// For why the table traits + blanket implementation on the tuple exists, see:
/// <https://github.com/Cuprate/cuprate/pull/102#pullrequestreview-1978348871>.
///
/// The `OpenTables` trait lets you open all tables you've defined, at once.
///
/// # Example
/// For examples of usage & output, see
/// [`cuprate_blockchain::tables`](https://github.com/Cuprate/cuprate/blob/main/storage/blockchain/src/tables.rs).
#[macro_export]
macro_rules! define_tables {
    (
        $(
            // Documentation and any `derive`'s.
            $(#[$attr:meta])*

            // The table name + doubles as the table struct name.
            $index:literal => $table:ident,

            // Key type => Value type.
            $key:ty => $value:ty
        ),* $(,)?
    ) => { $crate::paste::paste! {
        $(
            // Table struct.
            $(#[$attr])*
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

        /// Object containing all opened [`Table`](cuprate_database::Table)s in read-only mode.
        ///
        /// This is an encapsulated object that contains all
        /// available `Table`'s in read-only mode.
        ///
        /// It is a `Sealed` trait and is only implemented on a
        /// `(tuple, containing, all, table, types, ...)`.
        ///
        /// This is used to return a _single_ object from functions like
        /// [`OpenTables::open_tables`] rather than the tuple containing the tables itself.
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
                fn [<$table:snake>](&self) -> &impl $crate::DatabaseRo<$table>;
            )*

            /// This returns `true` if all tables are empty.
            ///
            /// # Errors
            /// This returns errors on regular database errors.
            fn all_tables_empty(&self) -> Result<bool, $crate::RuntimeError>;
        }

        /// Object containing all opened [`Table`](cuprate_database::Table)s in read + iter mode.
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
                fn [<$table:snake _iter>](&self) -> &(impl $crate::DatabaseRo<$table> + $crate::DatabaseIter<$table>);
            )*
        }

        /// Object containing all opened [`Table`](cuprate_database::Table)s in write mode.
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
                fn [<$table:snake _mut>](&mut self) -> &mut impl $crate::DatabaseRw<$table>;
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
                [<$table:upper>]: $crate::DatabaseRo<$table>,
            )*
        {
            $(
                // The function name of the accessor function is
                // the table type in `snake_case`, e.g., `block_info_v1s()`.
                #[inline]
                fn [<$table:snake>](&self) -> &impl $crate::DatabaseRo<$table> {
                    // The index of the database table in
                    // the tuple implements the table trait.
                    &self.$index
                }
            )*

            fn all_tables_empty(&self) -> Result<bool, $crate::RuntimeError> {
                $(
                     if !$crate::DatabaseRo::is_empty(&self.$index)? {
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
                [<$table:upper>]: $crate::DatabaseRo<$table> + $crate::DatabaseIter<$table>,
            )*
        {
            $(
                // The function name of the accessor function is
                // the table type in `snake_case` + `_iter`, e.g., `block_info_v1s_iter()`.
                #[inline]
                fn [<$table:snake _iter>](&self) -> &(impl $crate::DatabaseRo<$table> + $crate::DatabaseIter<$table>) {
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
                [<$table:upper>]: $crate::DatabaseRw<$table>,
            )*
        {
            $(
                // The function name of the mutable accessor function is
                // the table type in `snake_case` + `_mut`, e.g., `block_info_v1s_mut()`.
                #[inline]
                fn [<$table:snake _mut>](&mut self) -> &mut impl $crate::DatabaseRw<$table> {
                    &mut self.$index
                }
            )*
        }

        /// Open all tables at once.
        ///
        /// This trait encapsulates the functionality of opening all tables at once.
        /// It can be seen as the "constructor" for the [`Tables`] object.
        ///
        /// Note that this is already implemented on [`cuprate_database::EnvInner`], thus:
        /// - You don't need to implement this
        /// - It can be called using `env_inner.open_tables()` notation
        ///
        /// # Creation before opening
        /// As [`cuprate_database::EnvInner`] documentation states,
        /// tables must be created before they are opened.
        ///
        /// I.e. [`OpenTables::create_tables`] must be called before
        /// [`OpenTables::open_tables`] or else panics may occur.
        pub trait OpenTables<'env> {
            /// The read-only transaction type of the backend.
            type Ro<'tx>;
            /// The read-write transaction type of the backend.
            type Rw<'tx>;

            /// Open all tables in read/iter mode.
            ///
            /// This calls [`cuprate_database::EnvInner::open_db_ro`] on all database tables
            /// and returns a structure that allows access to all tables.
            ///
            /// # Errors
            /// This will only return [`cuprate_database::RuntimeError::Io`] if it errors.
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
            type Ro<'tx> = <Ei as $crate::EnvInner<'env>>::Ro<'tx>;
            type Rw<'tx> = <Ei as $crate::EnvInner<'env>>::Rw<'tx>;

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
