//! Database table abstraction; `trait Table`.

//---------------------------------------------------------------------------------------------------- Import
use std::borrow::{Borrow, Cow};

use crate::{table::Table, Storable, ToOwnedDebug};

//---------------------------------------------------------------------------------------------------- Table
/// TODO
pub trait ValueGuard<T: ToOwnedDebug> {
    /// TODO
    fn unguard(&self) -> Cow<'_, T>;
}

impl<T: ToOwnedDebug> ValueGuard<T> for Cow<'_, T> {
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
