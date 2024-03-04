//! Database tables.
//!
//! This module contains all the table definitions used by `cuprate-database`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    table::Table,
    types::{TestType, TestType2},
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
            $size:literal,    // Are the table's values all the same size?
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
            #[derive(Copy,Clone,Debug,PartialEq,PartialOrd,Eq,Ord,Hash)]
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
tables! {
    /// Test documentation.
    TestTable,
    true,
    i64 => TestType,

    /// Test documentation 2.
    TestTable2,
    true,
    u8 => TestType2,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
