//! Database tables.

//---------------------------------------------------------------------------------------------------- Import
use crate::table::Table;

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- Table macro
/// Create a zero-sized table struct, and implement the `Table` trait on it.
macro_rules! table {
    (
        $(#[$attr:meta])*  // Documentation and any `derive`'s.
        $table:ident,      // The table name + doubles as the table struct name.
        $key:ty,           // Key type.
        $value:ty,         // Value type.
    ) => {
        paste::paste! {
            // Table struct.
            // TODO: add serde?
            #[derive(Copy,Clone,Debug,PartialEq,PartialOrd,Eq,Ord,Hash)]
            $(#[$attr])*
            pub struct [<$table:camel>];

            // Table trait impl.
            impl Table for [<$table:camel>] {
                const NAME: &'static str = stringify!([<$table:lower>]);
                type Key = $key;
                type Value = $value;
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
