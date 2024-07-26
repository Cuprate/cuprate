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

//---------------------------------------------------------------------------------------------------- KeyImageSpentStatus
#[doc = crate::macros::monero_definition_link!(
    cc73fe71162d564ffda8e549b79a350bca53c454,
    "rpc/core_rpc_server_commands_defs.h",
    456..=460
)]
/// Used in [`crate::other::IsKeyImageSpentResponse`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[repr(u8)]
pub enum KeyImageSpentStatus {
    Unspent = 0,
    SpentInBlockchain = 1,
    SpentInPool = 2,
}

impl KeyImageSpentStatus {
    /// Convert [`Self`] to a [`u8`].
    ///
    /// ```rust
    /// use cuprate_rpc_types::misc::KeyImageSpentStatus as K;
    ///
    /// assert_eq!(K::Unspent.to_u8(), 0);
    /// assert_eq!(K::SpentInBlockchain.to_u8(), 1);
    /// assert_eq!(K::SpentInPool.to_u8(), 2);
    /// ```
    pub const fn to_u8(self) -> u8 {
        match self {
            Self::Unspent => 0,
            Self::SpentInBlockchain => 1,
            Self::SpentInPool => 2,
        }
    }

    /// Convert a [`u8`] to a [`Self`].
    ///
    /// # Errors
    /// This returns [`None`] if `u > 2`.
    ///
    /// ```rust
    /// use cuprate_rpc_types::misc::KeyImageSpentStatus as K;
    ///
    /// assert_eq!(K::from_u8(0), Some(K::Unspent));
    /// assert_eq!(K::from_u8(1), Some(K::SpentInBlockchain));
    /// assert_eq!(K::from_u8(2), Some(K::SpentInPool));
    /// assert_eq!(K::from_u8(3), None);
    /// ```
    pub const fn from_u8(u: u8) -> Option<Self> {
        Some(match u {
            0 => Self::Unspent,
            1 => Self::SpentInBlockchain,
            2 => Self::SpentInPool,
            _ => return None,
        })
    }
}

#[cfg(feature = "epee")]
impl EpeeValue for KeyImageSpentStatus {
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
