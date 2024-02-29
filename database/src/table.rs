//! Database table abstraction; `trait Table`.

//---------------------------------------------------------------------------------------------------- Import
use crate::key::Key;

use bytemuck::{CheckedBitPattern, NoUninit};

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
    // TODO:
    //
    // Add K/V comparison `type`s that define
    // how this table will be stored.
    //
    // type KeyComparator: fn(&Self::Key, &Self::Key) -> Ordering;
    // type ValueComparator: fn(&Self::Value, &Self::Value) -> Ordering;

    /// Name of the database table.
    const NAME: &'static str;

    /// Whether the table's values are all the same size or not.
    const CONSTANT_SIZE: bool;

    /// Primary key type.
    type Key: Key;

    /// Value type.
    type Value: CheckedBitPattern + NoUninit;
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
