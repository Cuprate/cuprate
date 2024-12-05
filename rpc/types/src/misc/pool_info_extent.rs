//! TODO

//---------------------------------------------------------------------------------------------------- Use
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::{
    error,
    macros::bytes::{Buf, BufMut},
    EpeeValue, Marker,
};

//---------------------------------------------------------------------------------------------------- PoolInfoExtent
#[doc = crate::macros::monero_definition_link!(
    cc73fe71162d564ffda8e549b79a350bca53c454,
    "rpc/core_rpc_server_commands_defs.h",
    223..=228
)]
/// Used in [`crate::bin::GetBlocksResponse`].
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[repr(u8)]
pub enum PoolInfoExtent {
    #[default]
    None = 0,
    Incremental = 1,
    Full = 2,
}

impl PoolInfoExtent {
    /// Convert [`Self`] to a [`u8`].
    ///
    /// ```rust
    /// use cuprate_rpc_types::misc::PoolInfoExtent as P;
    ///
    /// assert_eq!(P::None.to_u8(), 0);
    /// assert_eq!(P::Incremental.to_u8(), 1);
    /// assert_eq!(P::Full.to_u8(), 2);
    /// ```
    pub const fn to_u8(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Incremental => 1,
            Self::Full => 2,
        }
    }

    /// Convert a [`u8`] to a [`Self`].
    ///
    /// # Errors
    /// This returns [`None`] if `u > 2`.
    ///
    /// ```rust
    /// use cuprate_rpc_types::misc::PoolInfoExtent as P;
    ///
    /// assert_eq!(P::from_u8(0), Some(P::None));
    /// assert_eq!(P::from_u8(1), Some(P::Incremental));
    /// assert_eq!(P::from_u8(2), Some(P::Full));
    /// assert_eq!(P::from_u8(3), None);
    /// ```
    pub const fn from_u8(u: u8) -> Option<Self> {
        Some(match u {
            0 => Self::None,
            1 => Self::Incremental,
            2 => Self::Full,
            _ => return None,
        })
    }
}

#[cfg(feature = "epee")]
impl EpeeValue for PoolInfoExtent {
    const MARKER: Marker = <u8 as EpeeValue>::MARKER;

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> error::Result<Self> {
        let u = u8::read(r, marker)?;
        Self::from_u8(u).ok_or(error::Error::Format("u8 was greater than 2"))
    }

    fn write<B: BufMut>(self, w: &mut B) -> error::Result<()> {
        let u = self.to_u8();
        u8::write(u, w)?;
        Ok(())
    }
}
