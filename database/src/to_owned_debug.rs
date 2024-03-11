//! Borrowed/owned data abstraction; `trait ToOwnedDebug`.

//---------------------------------------------------------------------------------------------------- Import
use std::fmt::Debug;

use crate::{key::Key, storable::Storable};

//---------------------------------------------------------------------------------------------------- Table
/// TODO
pub trait ToOwnedDebug: Debug + ToOwned<Owned = Self::OwnedDebug> {
    ///  TODO
    type OwnedDebug: Debug;
}

// TODO
impl<O: Debug, T: ToOwned<Owned = O> + Debug + ?Sized> ToOwnedDebug for T {
    type OwnedDebug = O;
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
