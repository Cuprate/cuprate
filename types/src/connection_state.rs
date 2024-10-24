//! [`ConnectionState`].

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::{
    error,
    macros::bytes::{Buf, BufMut},
    EpeeValue, Marker,
};

use strum::{
    AsRefStr, Display, EnumCount, EnumIs, EnumString, FromRepr, IntoStaticStr, VariantArray,
};

/// An enumeration of P2P connection states.
///
/// Used in `cuprate_p2p` and `cuprate_rpc_types`.
///
/// Original definition:
/// - <https://github.com/monero-project/monero/blob/893916ad091a92e765ce3241b94e706ad012b62a/src/cryptonote_basic/connection_context.h#L49>
///
/// # Serde
/// This type's `serde` implementation depends on `snake_case`.
///
/// ```rust
/// use cuprate_types::ConnectionState as C;
/// use serde_json::to_string;
///
/// assert_eq!(to_string(&C::BeforeHandshake).unwrap(), r#""before_handshake""#);
/// assert_eq!(to_string(&C::Synchronizing).unwrap(), r#""synchronizing""#);
/// assert_eq!(to_string(&C::Standby).unwrap(), r#""standby""#);
/// assert_eq!(to_string(&C::Idle).unwrap(), r#""idle""#);
/// assert_eq!(to_string(&C::Normal).unwrap(), r#""normal""#);
///
/// assert_eq!(C::BeforeHandshake.to_string(), "before_handshake");
/// assert_eq!(C::Synchronizing.to_string(), "synchronizing");
/// assert_eq!(C::Standby.to_string(), "standby");
/// assert_eq!(C::Idle.to_string(), "idle");
/// assert_eq!(C::Normal.to_string(), "normal");
/// ```
#[derive(
    Copy,
    Clone,
    Default,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    AsRefStr,
    Display,
    EnumCount,
    EnumIs,
    EnumString,
    FromRepr,
    IntoStaticStr,
    VariantArray,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))] // cuprate-rpc-types depends on snake_case
#[strum(serialize_all = "snake_case")]
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
    /// use cuprate_types::ConnectionState as C;
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
    /// use cuprate_types::ConnectionState as C;
    ///
    /// assert_eq!(C::from_u8(0), Some(C::BeforeHandshake));
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
