//! Database table abstraction; `trait Table`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{key::Key, storable::Storable};

//---------------------------------------------------------------------------------------------------- Table
/// Database table metadata.
///
/// Purely compile time information for database tables.
///
/// ## Sealed
/// This trait is [`Sealed`](https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed).
///
/// It is, and can only be implemented on the types inside [`tables`][crate::tables].
pub trait Table: crate::tables::private::Sealed {
    /// Name of the database table.
    const NAME: &'static str;

    /// Whether the table's values are all the same size or not.
    const CONSTANT_SIZE: bool;

    /// Primary key type.
    type Key: Key;

    /// Value type.
    type Value: Storable + ?Sized;
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
