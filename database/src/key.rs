//! Database key abstraction; `trait Key`.

//---------------------------------------------------------------------------------------------------- Import
use std::cmp::Ordering;

use bytemuck::{CheckedBitPattern, NoUninit};

use crate::storable::Storable;

//---------------------------------------------------------------------------------------------------- Table
/// Database [`Table`](crate::table::Table) key metadata.
///
/// Purely compile time information for database table keys, supporting duplicate keys.
pub trait Key: Storable + Sized {
    /// Does this [`Key`] require multiple keys to reach a value?
    ///
    /// # Invariant
    /// - If [`Key::DUPLICATE`] is `true`, [`Key::primary_secondary`] MUST be re-implemented.
    /// - If [`Key::DUPLICATE`] is `true`, [`Key::new_with_max_secondary`] MUST be re-implemented.
    const DUPLICATE: bool;

    /// Does this [`Key`] have a custom comparison function?
    ///
    /// # Invariant
    /// If [`Key::CUSTOM_COMPARE`] is `true`, [`Key::compare`] MUST be re-implemented.
    const CUSTOM_COMPARE: bool;

    /// The primary key type.
    type Primary: Storable;

    /// Acquire [`Self::Primary`] & the secondary key.
    fn primary_secondary(self) -> (Self::Primary, u64) {
        unreachable!()
    }

    /// Compare 2 [`Key`]'s against each other.
    ///
    /// By default, this does a straight byte comparison.
    fn compare(left: &[u8], right: &[u8]) -> Ordering {
        left.cmp(right)
    }

    /// Create a new [`Key`] from the [`Key::Primary`] type,
    /// with the secondary key type set to the maximum value.
    ///
    /// # Invariant
    /// Secondary key must be the max value of the type.
    fn new_with_max_secondary(primary: Self::Primary) -> Self {
        unreachable!()
    }
}

//---------------------------------------------------------------------------------------------------- Impl
/// Implement `Key` on most primitive types.
///
/// - `Key::DUPLICATE` is always `false`.
/// - `Key::CUSTOM_COMPARE` is always `false`.
macro_rules! impl_key {
    (
        $(
            $t:ident // Key type.
        ),* $(,)?
    ) => {
        $(
            impl Key for $t {
                const DUPLICATE: bool = false;
                const CUSTOM_COMPARE: bool = false;

                type Primary = $t;
            }
        )*
    };
}

// Implement `Key` for primitives.
impl_key! {
    u8,
    u16,
    u32,
    u64,
    i8,
    i16,
    i32,
    i64,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
