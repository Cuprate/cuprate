//! Database key abstraction; `trait Key`.

//---------------------------------------------------------------------------------------------------- Import
use bytemuck::{CheckedBitPattern, NoUninit};

//---------------------------------------------------------------------------------------------------- Table
/// Database [`Table`](crate::table::Table) key metadata.
///
/// Purely compile time information for database table keys, supporting duplicate keys.
pub trait Key {
    /// Does this [`Key`] require multiple keys to reach a value?
    ///
    /// If [`Key::DUPLICATE`] is `true`, [`Key::Secondary`] will contain
    /// the "subkey", or secondary key needed to access the actual value.
    ///
    /// If [`Key::DUPLICATE`] is `false`, [`Key::Secondary`] is ignored.
    /// Consider using [`std::convert::Infallible`] as the type.
    const DUPLICATE: bool;

    /// The primary key type.
    type Primary: Key;

    /// The secondary key type.
    type Secondary: CheckedBitPattern + NoUninit;

    /// Acquire [`Key::Primary`].
    fn primary(self) -> Self::Primary;

    /// Acquire [`Self::Primary`] & [`Self::Secondary`].
    ///
    /// This only needs to be implemented on types that are [`Self::DUPLICATE`].
    ///
    /// Consider using [`unreachable!()`] on non-duplicate key tables.
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

                // This 0 variant enum is unconstructable,
                // and "has the same role as the ! “never” type":
                // <https://doc.rust-lang.org/std/convert/enum.Infallible.html#future-compatibility>.
                //
                // FIXME: Use the `!` type when stable.
                //
                // FIXME:
                // `bytemuck::Pod` cannot be implemented on `std::convert::Infallible`:
                // <https://docs.rs/bytemuck/1.14.3/bytemuck/trait.Pod.html>
                // so this invariant is pushed to runtime panics in `primary_secondary()`.
                type Secondary = ();

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
    P: Key,
    S: CheckedBitPattern + NoUninit,
{
    const DUPLICATE: bool = true;
    type Primary = P;
    type Secondary = S;

    #[inline]
    fn primary(self) -> Self::Primary {
        self.primary
    }

    #[inline]
    fn primary_secondary(self) -> (Self::Primary, Self::Secondary) {
        (self.primary, self.secondary)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
