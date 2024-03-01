//! Database key abstraction; `trait Key`.

//---------------------------------------------------------------------------------------------------- Import
use std::cmp::Ordering;

use bytemuck::{CheckedBitPattern, NoUninit};

use crate::storable::Storable;

//---------------------------------------------------------------------------------------------------- Table
/// Database [`Table`](crate::table::Table) key metadata.
///
/// Purely compile time information for database table keys, supporting duplicate keys.
pub trait Key: Storable {
    /// Does this [`Key`] require multiple keys to reach a value?
    const DUPLICATE: bool;

    /// If this is `true`, it means this key MUST
    /// re-implement and use [`Key::compare`].
    const CUSTOM_COMPARE: bool;

    /// The primary key type.
    type Primary: Storable;

    /// Acquire [`Self::Primary`] & the secondary key.
    ///
    /// This only needs to be implemented on types that are [`Self::DUPLICATE`].
    ///
    /// Consider using [`unreachable!()`] on non-duplicate key tables.
    fn primary_secondary(self) -> (Self::Primary, u64);

    /// Compare 2 [`Key`]'s against each other.
    ///
    /// By default, this does a straight byte comparison.
    ///
    /// # Invariant
    /// If [`Key::CUSTOM_COMPARE`] is `true`, this MUST be re-implemented.
    fn compare(left: &[u8], right: &[u8]) -> Ordering {
        left.cmp(right)
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

                #[cold] #[inline(never)]
                fn primary_secondary(self) -> (Self::Primary, u64) {
                    unreachable!();
                }
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
