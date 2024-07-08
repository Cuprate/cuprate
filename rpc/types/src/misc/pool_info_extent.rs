//! TODO

//---------------------------------------------------------------------------------------------------- Use
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::{
    macros::bytes::{Buf, BufMut},
    EpeeValue, Marker,
};

//---------------------------------------------------------------------------------------------------- PoolInfoExtent
/// Used in [`crate::bin::GetBlocksResponse`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[repr(u8)]
pub enum PoolInfoExtent {
    None = 0,
    Incremental = 1,
    Full = 2,
}

// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_defs.h#L138-L163>
#[cfg(feature = "epee")]
impl EpeeValue for PoolInfoExtent {
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
