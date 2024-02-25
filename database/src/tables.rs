//! Database tables.
//!
//! This module contains all the table definitions used by `cuprate-database`
//! and [`Tables`], an `enum` containing all [`Table`]s.

//---------------------------------------------------------------------------------------------------- Import
use crate::table::Table;

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

//---------------------------------------------------------------------------------------------------- Tables
/// An enumeration of _all_ database tables.
///
/// TODO: I don't think we need this.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[allow(missing_docs)]
pub enum Tables {
    TestTable(TestTable),
    TestTable2(TestTable2),
}

impl Tables {
    /// Get the [`Table::NAME`].
    pub const fn name(&self) -> &'static str {
        /// Hack to access associated trait constant via a variable.
        const fn get<T: Table>(t: &T) -> &'static str {
            T::NAME
        }

        match self {
            Self::TestTable(t) => get(t),
            Self::TestTable2(t) => get(t),
        }
    }

    /// Get the [`Table::CONSTANT_SIZE`].
    pub const fn constant_size(&self) -> bool {
        /// Hack to access associated trait constant via a variable.
        const fn get<T: Table>(t: &T) -> bool {
            T::CONSTANT_SIZE
        }

        match self {
            Self::TestTable(t) => get(t),
            Self::TestTable2(t) => get(t),
        }
    }
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
            #[cfg_attr(feature = "borsh", derive(borsh::BorshSerialize, borsh::BorshDeserialize))]
            #[derive(Copy,Clone,Debug,PartialEq,PartialOrd,Eq,Ord,Hash)]
            pub struct [<$table:camel>];

            // Implement the `Sealed` in this file.
            // Required by `Table`.
            impl private::Sealed for [<$table:camel>] {}

            // Table trait impl.
            impl Table for [<$table:camel>] {
                const NAME: &'static str = stringify!([<$table:snake>]);
                const CONSTANT_SIZE: bool = $size;
                type Key = $key;
                type Value = $value;
            }

            // Table enum.
            impl From<[<$table:camel>]> for Tables {
                fn from(table: [<$table:camel>]) -> Self {
                    Self::[<$table:camel>](table)
                }
            }
        )* }
    };
}

//---------------------------------------------------------------------------------------------------- Tables
tables! {
    /// Test documentation.
    TestTable,
    true,
    i64 => u64,

    /// Test documentation 2.
    TestTable2,
    true,
    u8 => i8,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
