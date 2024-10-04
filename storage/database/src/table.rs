//! Database table abstraction; `trait Table`.

//---------------------------------------------------------------------------------------------------- Import

use crate::{key::Key, storable::Storable};

//---------------------------------------------------------------------------------------------------- Table
/// Database table metadata.
///
/// Purely compile time information for database tables.
///
/// See [`crate::define_tables`] for bulk table generation.
pub trait Table: 'static {
    /// Name of the database table.
    const NAME: &'static str;

    /// Primary key type.
    type Key: Key + 'static;

    /// Value type.
    type Value: Storable + 'static;
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
