//! Borrowed/owned data abstraction; `trait ToOwnedDebug`.

//---------------------------------------------------------------------------------------------------- Import
use std::fmt::Debug;

use crate::{key::Key, storable::Storable};

//---------------------------------------------------------------------------------------------------- Table
/// `T: Debug` and `T::Owned: Debug`.
///
/// This trait simply combines [`Debug`] and [`ToOwned`]
/// such that the `Owned` version must also be [`Debug`].
///
/// An example is `[u8]` which is [`Debug`], and
/// its owned version `Vec<u8>` is also [`Debug`].
///
/// # Explanation (not needed for practical use)
/// This trait solely exists due to the `redb` backend
/// requiring [`Debug`] bounds on keys and values.
///
/// As we have `?Sized` types like `[u8]`, and due to `redb` requiring
/// allocation upon deserialization, we must make our values `ToOwned`.
///
/// However, this requires that the `Owned` version is also `Debug`.
/// Combined with:
/// - [`Table::Key`](crate::Table::Key)
/// - [`Table::Value`](crate::Table::Value)
/// - [`Key::Primary`]
///
/// this quickly permutates into many many many `where` bounds on
/// each function that touchs any data that must be deserialized.
///
/// This trait and the blanket impl it provides get applied all these types
/// automatically, which means we don't have to write these bounds everywhere.
pub trait ToOwnedDebug: Debug + ToOwned<Owned = Self::OwnedDebug> {
    /// The owned version of [`Self`].
    ///
    /// Should be equal to `<T as ToOwned>::Owned`.
    type OwnedDebug: Debug;
}

// The blanket impl that covers all our types.
impl<O: Debug, T: ToOwned<Owned = O> + Debug + ?Sized> ToOwnedDebug for T {
    type OwnedDebug = O;
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
