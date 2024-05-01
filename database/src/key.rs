//! Database key abstraction; `trait Key`.

//---------------------------------------------------------------------------------------------------- Import
use std::cmp::Ordering;

use crate::storable::Storable;

//---------------------------------------------------------------------------------------------------- Table
/// Database [`Table`](crate::table::Table) key metadata.
///
/// Purely compile time information for database table keys.
pub trait Key: Storable + Sized {
    /// The primary key type.
    type Primary: Storable;

    /// Compare 2 [`Key`]'s against each other.
    ///
    /// By default, this does a straight _byte_ comparison,
    /// not a comparison of the key's value.
    ///
    /// ```rust
    /// # use cuprate_database::*;
    /// assert_eq!(
    ///     <u64 as Key>::compare([0].as_slice(), [1].as_slice()),
    ///     std::cmp::Ordering::Less,
    /// );
    /// assert_eq!(
    ///     <u64 as Key>::compare([1].as_slice(), [1].as_slice()),
    ///     std::cmp::Ordering::Equal,
    /// );
    /// assert_eq!(
    ///     <u64 as Key>::compare([2].as_slice(), [1].as_slice()),
    ///     std::cmp::Ordering::Greater,
    /// );
    /// ```
    #[inline]
    fn compare(left: &[u8], right: &[u8]) -> Ordering {
        left.cmp(right)
    }
}

//---------------------------------------------------------------------------------------------------- Impl
impl<T> Key for T
where
    T: Storable + Sized,
{
    type Primary = Self;
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
