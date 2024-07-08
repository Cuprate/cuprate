//! TODO

//---------------------------------------------------------------------------------------------------- Use
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::{
    macros::bytes::{Buf, BufMut},
    EpeeValue, Marker,
};

//---------------------------------------------------------------------------------------------------- KeyImageSpentStatus
/// Used in [`crate::other::IsKeyImageSpentResponse`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[repr(u8)]
pub enum KeyImageSpentStatus {
    Unspent = 0,
    SpentInBlockchain = 1,
    SpentInPool = 2,
}

#[cfg(feature = "epee")]
impl EpeeValue for KeyImageSpentStatus {
    const MARKER: Marker = <String as EpeeValue>::MARKER;

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> cuprate_epee_encoding::Result<Self> {
        todo!()
    }

    fn should_write(&self) -> bool {
        todo!()
    }

    fn epee_default_value() -> Option<Self> {
        todo!()
    }

    fn write<B: BufMut>(self, w: &mut B) -> cuprate_epee_encoding::Result<()> {
        todo!()
    }
}
