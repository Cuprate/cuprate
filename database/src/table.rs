//! Database table abstraction; `trait Table`.

//---------------------------------------------------------------------------------------------------- Import
use std::fmt::Debug;

use crate::{key::Key, storable::Storable};

//---------------------------------------------------------------------------------------------------- Table
/// Database table metadata.
///
/// Purely compile time information for database tables.
///
/// TODO: document these trait bounds...
///
/// ## Sealed
/// This trait is [`Sealed`](https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed).
///
/// It is, and can only be implemented on the types inside [`tables`][crate::tables].
pub trait Table: crate::tables::private::Sealed + 'static
where
    <<Self as Table>::Key as ToOwned>::Owned: Debug,
    <<Self as Table>::Value as ToOwned>::Owned: Debug,
    <<<Self as Table>::Key as Key>::Primary as ToOwned>::Owned: Debug,
{
    /// Name of the database table.
    const NAME: &'static str;

    // TODO:
    //
    // `redb` requires `K/V` is `'static`:
    // - <https://docs.rs/redb/1.5.0/redb/struct.ReadOnlyTable.html>
    // - <https://docs.rs/redb/1.5.0/redb/struct.Table.html>
    //
    // ...but kinda not really?
    //   "Note that the lifetime of the K and V type parameters does not impact
    //   the lifetimes of the data that is stored or retrieved from the table"
    //   <https://docs.rs/redb/1.5.0/redb/struct.TableDefinition.html>
    //
    // This might be incompatible with `heed`. We'll see
    // after function bodies are actually implemented...

    /// Primary key type.
    type Key: Key + 'static;

    /// Value type.
    type Value: Storable + ?Sized + 'static;
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
