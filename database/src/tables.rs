//! Database tables.

//---------------------------------------------------------------------------------------------------- Import
use crate::table::Table;

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- Tables
/// An enumeration of _all_ database tables.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum Tables {
    /// TODO
    TestTable(TestTable),
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
        }
    }
}

//---------------------------------------------------------------------------------------------------- Table macro
/// Create a zero-sized table struct, and implement the `Table` trait on it.
///
/// Table struct are automatically `CamelCase`,
/// and their static string names are automatically `snake_case`.
macro_rules! table {
    (
        $(#[$attr:meta])*  // Documentation and any `derive`'s.
        $table:ident,      // The table name + doubles as the table struct name.
        $key:ty,           // Key type.
        $value:ty,         // Value type.
    ) => {
        paste::paste! {
            // Table struct.
            $(#[$attr])*
            // The below test show the `snake_case` table name in cargo docs.
            /// ## Table Name
            /// ```rust
            /// # use cuprate_database::tables::*;
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

            // Table trait impl.
            impl Table for [<$table:camel>] {
                const NAME: &'static str = stringify!([<$table:snake>]);
                type Key = $key;
                type Value = $value;
            }

            // Table enum.
            impl From<[<$table:camel>]> for Tables {
                fn from(table: [<$table:camel>]) -> Self {
                    Self::[<$table:camel>](table)
                }
            }
        }
    };
}

//---------------------------------------------------------------------------------------------------- Tables
// This should create:
// ```rust
// /// Test documentation.
// pub struct TestTable;
//
// impl Table for TestTable {
//     const NAME = testtable;
//     type Key = usize;
//     type Value = String;
// }
// ```
table! {
    /// Test documentation.
    TestTable,
    usize,
    u64,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
