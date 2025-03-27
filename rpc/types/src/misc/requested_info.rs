//! [`RequestedInfo`]

//---------------------------------------------------------------------------------------------------- Use
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::{
    error,
    macros::bytes::{Buf, BufMut},
    EpeeValue, Marker,
};

//---------------------------------------------------------------------------------------------------- RequestedInfo
#[doc = crate::macros::monero_definition_link!(
    "cc73fe71162d564ffda8e549b79a350bca53c454",
    "rpc/core_rpc_server_commands_defs.h",
    178..=183
)]
/// Used in [`crate::bin::GetBlocksRequest`].
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "u8", into = "u8"))]
#[repr(u8)]
pub enum RequestedInfo {
    #[default]
    BlocksOnly = 0,
    BlocksAndPool = 1,
    PoolOnly = 2,
}

impl RequestedInfo {
    /// Convert [`Self`] to a [`u8`].
    ///
    /// ```rust
    /// use cuprate_rpc_types::misc::RequestedInfo as R;
    ///
    /// assert_eq!(R::BlocksOnly.to_u8(), 0);
    /// assert_eq!(R::BlocksAndPool.to_u8(), 1);
    /// assert_eq!(R::PoolOnly.to_u8(), 2);
    /// ```
    pub const fn to_u8(self) -> u8 {
        match self {
            Self::BlocksOnly => 0,
            Self::BlocksAndPool => 1,
            Self::PoolOnly => 2,
        }
    }

    /// Convert a [`u8`] to a [`Self`].
    ///
    /// # Errors
    /// This returns [`None`] if `u > 2`.
    ///
    /// ```rust
    /// use cuprate_rpc_types::misc::RequestedInfo as R;
    ///
    /// assert_eq!(R::from_u8(0), Some(R::BlocksOnly));
    /// assert_eq!(R::from_u8(1), Some(R::BlocksAndPool));
    /// assert_eq!(R::from_u8(2), Some(R::PoolOnly));
    /// assert_eq!(R::from_u8(3), None);
    /// ```
    pub const fn from_u8(u: u8) -> Option<Self> {
        Some(match u {
            0 => Self::BlocksOnly,
            1 => Self::BlocksAndPool,
            2 => Self::PoolOnly,
            _ => return None,
        })
    }
}

impl From<RequestedInfo> for u8 {
    fn from(value: RequestedInfo) -> Self {
        value.to_u8()
    }
}

impl TryFrom<u8> for RequestedInfo {
    type Error = u8;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_u8(value).ok_or(value)
    }
}

#[cfg(feature = "epee")]
impl EpeeValue for RequestedInfo {
    const MARKER: Marker = u8::MARKER;

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
