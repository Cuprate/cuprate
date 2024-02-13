//! Database key abstraction; `trait Key`.

//---------------------------------------------------------------------------------------------------- Import
#[allow(unused_imports)] // docs
use crate::table::Table;

//---------------------------------------------------------------------------------------------------- Table
/// Database [`Table`] key metadata.
///
/// Purely compile time information for database table keys, supporting duplicate keys.
pub trait Key {
    /// Does this [`Key`] require multiple keys to reach a value?
    ///
    /// If [`Key::DUPLICATE`] is `true`, [`Key::Secondary`] will contain
    /// the "subkey", or secondary key needed to access the actual value.
    ///
    /// If [`Key::DUPLICATE`] is `false`, [`Key::Secondary`]
    /// will just be the same type as [`Key::Primary`].
    const DUPLICATE: bool;

    /// The primary key type.
    type Primary;

    /// The secondary key type.
    ///
    /// Only needs to be different than [`Key::Primary`]
    /// if [`Key::DUPLICATE`] is `true`.
    type Secondary;

    /// Acquire [`Key::Primary`].
    fn primary(self) -> Self::Primary;

    /// Acquire [`Self::Primary`] & [`Self::Secondary`].
    ///
    /// This only needs to be implemented on types that are [`Self::DUPLICATE`].
    ///
    /// It is `unreachable!()` on non-duplicate key tables.
    fn primary_secondary(self) -> (Self::Primary, Self::Secondary);
}

/// Duplicate key container.
///
/// This is a generic container to use alongside [`Key`] to support
/// tables that require more than 1 key to access the value.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct DupKey<P, S> {
    /// Primary key type.
    pub primary: P,
    /// Secondary key type.
    pub secondary: S,
}

//---------------------------------------------------------------------------------------------------- Impl
/// Implement `Key` on most primitive types.
///
/// `Key::DUPLICATE` is always `false`.
macro_rules! impl_key {
    (
        $(
            $t:ident // Key type.
        ),* $(,)?
    ) => {
        $(
            impl Key for $t {
                const DUPLICATE: bool = false;
                type Primary = $t;
                type Secondary = $t;

                #[inline(always)]
                fn primary(self) -> Self::Primary {
                    self
                }

                #[cold] #[inline(never)]
                fn primary_secondary(self) -> (Self::Primary, Self::Secondary) {
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

// Implement `Key` for any [`DupKey`] using [`Copy`] types.
impl<P, S> Key for DupKey<P, S>
where
    P: Key + Copy,
    S: Key + Copy,
{
    const DUPLICATE: bool = true;

    type Primary = Self;

    type Secondary = S;

    #[inline]
    fn primary(self) -> Self::Primary {
        self
    }

    #[inline]
    fn primary_secondary(self) -> (Self::Primary, Self::Secondary) {
        (self, self.secondary)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
