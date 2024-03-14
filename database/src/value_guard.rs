//! Database table value "guard" abstraction; `trait ValueGuard`.

//---------------------------------------------------------------------------------------------------- Import
use std::borrow::{Borrow, Cow};

use crate::{table::Table, Storable, ToOwnedDebug};

//---------------------------------------------------------------------------------------------------- Table
/// A guard that allows you to access a value.
///
/// This trait acts as an object that must be kept alive,
/// and will give you access to a [`Table`]'s value.
///
/// # Explanation (not needed for practical use)
/// This trait solely exists due to the `redb` backend
/// not _directly_ returning the value, but a
/// [guard object](https://docs.rs/redb/1.5.0/redb/struct.AccessGuard.html)
/// that has a lifetime attached to the key.
/// It does not implement `Deref` or `Borrow` and such.
///
/// Also, due to `redb` requiring `Cow`, this object builds on that.
///
/// - `heed` will always be `Cow::Borrowed`
/// - `redb` will always be `Cow::Borrowed` for `[u8]`
///   or any type where `Storable::ALIGN == 1`
/// - `redb` will always be `Cow::Owned` for everything else
pub trait ValueGuard<T: ToOwnedDebug + ?Sized> {
    /// Retrieve the data from the guard.
    fn unguard(&self) -> Cow<'_, T>;
}

impl<T: ToOwnedDebug + ?Sized> ValueGuard<T> for Cow<'_, T> {
    #[inline]
    fn unguard(&self) -> Cow<'_, T> {
        Cow::Borrowed(self.borrow())
    }
}

// HACK:
// This is implemented for `redb::AccessGuard<'_>` in
// `src/backend/redb/storable.rs` due to struct privacy.

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
