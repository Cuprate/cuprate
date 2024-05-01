//! Wrapper type for partially-`unsafe` usage of `T: !Send`.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::Borrow,
    ops::{Deref, DerefMut},
};

use bytemuck::TransparentWrapper;

use crate::storable::StorableVec;

//---------------------------------------------------------------------------------------------------- Aliases
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, TransparentWrapper)]
#[repr(transparent)]
/// A wrapper type that `unsafe`ly implements `Send` for any `T`.
///
/// This is a marker/wrapper type that allows wrapping
/// any type `T` such that it implements `Send`.
///
/// This is to be used when `T` is `Send`, but only in certain
/// situations not provable to the compiler, or is otherwise a
/// a pain to prove and/or less efficient.
///
/// It is up to the users of this type to ensure their
/// usage of `UnsafeSendable` are actually safe.
///
/// Notably, `heed`'s table type uses this inside `service`.
pub(crate) struct UnsafeSendable<T>(T);

#[allow(clippy::non_send_fields_in_send_ty)]
// SAFETY: Users ensure that their usage of this type is safe.
unsafe impl<T> Send for UnsafeSendable<T> {}

impl<T> UnsafeSendable<T> {
    /// Create a new [`UnsafeSendable`].
    ///
    /// # Safety
    /// By constructing this type, you must ensure the usage
    /// of the resulting `Self` is follows all the [`Send`] rules.
    pub(crate) const unsafe fn new(t: T) -> Self {
        Self(t)
    }

    /// Extract the inner `T`.
    pub(crate) fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Borrow<T> for UnsafeSendable<T> {
    fn borrow(&self) -> &T {
        &self.0
    }
}

impl<T> AsRef<T> for UnsafeSendable<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> AsMut<T> for UnsafeSendable<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> Deref for UnsafeSendable<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for UnsafeSendable<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
