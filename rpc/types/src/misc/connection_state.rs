//! Types of network addresses; used in P2P.

use cuprate_epee_encoding::Marker;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::{
    error,
    macros::bytes::{Buf, BufMut},
    EpeeValue,
};

/// Used in [`crate::misc::ConnectionInfo::address_type`].
#[doc = crate::macros::monero_definition_link!(
    cc73fe71162d564ffda8e549b79a350bca53c454,
    "cryptonote_basic/connection_context.h",
    49..=56
)]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged, try_from = "u8", into = "u8"))]
#[repr(u8)]
pub enum ConnectionState {
    BeforeHandshake,
    Synchronizing,
    Standby,
    Idle,
    #[default]
    Normal,
}

impl ConnectionState {
    /// Convert [`Self`] to a [`u8`].
    ///
    /// ```rust
    /// use cuprate_p2p_core::types::ConnectionState as C;
    ///
    /// assert_eq!(C::BeforeHandshake.to_u8(), 0);
    /// assert_eq!(C::Synchronizing.to_u8(), 1);
    /// assert_eq!(C::Standby.to_u8(), 2);
    /// assert_eq!(C::Idle.to_u8(), 3);
    /// assert_eq!(C::Normal.to_u8(), 4);
    /// ```
    pub const fn to_u8(self) -> u8 {
        self as u8
    }

    /// Convert a [`u8`] to a [`Self`].
    ///
    /// # Errors
    /// This returns [`None`] if `u > 4`.
    ///
    /// ```rust
    /// use cuprate_p2p_core::types::ConnectionState as C;
    ///
    /// assert_eq!(C::from_u8(0), Some(C::BeforeHandShake));
    /// assert_eq!(C::from_u8(1), Some(C::Synchronizing));
    /// assert_eq!(C::from_u8(2), Some(C::Standby));
    /// assert_eq!(C::from_u8(3), Some(C::Idle));
    /// assert_eq!(C::from_u8(4), Some(C::Normal));
    /// assert_eq!(C::from_u8(5), None);
    /// ```
    pub const fn from_u8(u: u8) -> Option<Self> {
        Some(match u {
            0 => Self::BeforeHandshake,
            1 => Self::Synchronizing,
            2 => Self::Standby,
            3 => Self::Idle,
            4 => Self::Normal,
            _ => return None,
        })
    }
}

impl From<ConnectionState> for u8 {
    fn from(value: ConnectionState) -> Self {
        value.to_u8()
    }
}

impl TryFrom<u8> for ConnectionState {
    type Error = u8;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match Self::from_u8(value) {
            Some(s) => Ok(s),
            None => Err(value),
        }
    }
}

#[cfg(feature = "epee")]
impl EpeeValue for ConnectionState {
    const MARKER: Marker = u8::MARKER;

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> error::Result<Self> {
        let u = u8::read(r, marker)?;
        Self::from_u8(u).ok_or(error::Error::Format("u8 was greater than 4"))
    }

    fn write<B: BufMut>(self, w: &mut B) -> error::Result<()> {
        let u = self.to_u8();
        u8::write(u, w)?;
        Ok(())
    }
}
