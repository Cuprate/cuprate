//! Types of network addresses; used in P2P.

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

/// An enumeration of address types.
///
/// Used in `cuprate_p2p` and `cuprate_types`
///
/// Original definition:
/// - <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/epee/include/net/enums.h/#L49>
///
/// # Serde
/// This type's `serde` implementation (de)serializes from a [`u8`].
///
/// ```rust
/// use cuprate_types::AddressType as A;
/// use serde_json::{to_string, from_str};
///
/// assert_eq!(from_str::<A>(&"0").unwrap(), A::Invalid);
/// assert_eq!(from_str::<A>(&"1").unwrap(), A::Ipv4);
/// assert_eq!(from_str::<A>(&"2").unwrap(), A::Ipv6);
/// assert_eq!(from_str::<A>(&"3").unwrap(), A::I2p);
/// assert_eq!(from_str::<A>(&"4").unwrap(), A::Tor);
///
/// assert_eq!(to_string(&A::Invalid).unwrap(), "0");
/// assert_eq!(to_string(&A::Ipv4).unwrap(), "1");
/// assert_eq!(to_string(&A::Ipv6).unwrap(), "2");
/// assert_eq!(to_string(&A::I2p).unwrap(), "3");
/// assert_eq!(to_string(&A::Tor).unwrap(), "4");
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
#[cfg_attr(feature = "serde", serde(untagged, try_from = "u8", into = "u8"))]
#[repr(u8)]
pub enum AddressType {
    #[default]
    Invalid,
    Ipv4,
    Ipv6,
    I2p,
    Tor,
}

impl AddressType {
    /// Convert [`Self`] to a [`u8`].
    ///
    /// ```rust
    /// use cuprate_types::AddressType as A;
    ///
    /// assert_eq!(A::Invalid.to_u8(), 0);
    /// assert_eq!(A::Ipv4.to_u8(), 1);
    /// assert_eq!(A::Ipv6.to_u8(), 2);
    /// assert_eq!(A::I2p.to_u8(), 3);
    /// assert_eq!(A::Tor.to_u8(), 4);
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
    /// use cuprate_types::AddressType as A;
    ///
    /// assert_eq!(A::from_u8(0), Some(A::Invalid));
    /// assert_eq!(A::from_u8(1), Some(A::Ipv4));
    /// assert_eq!(A::from_u8(2), Some(A::Ipv6));
    /// assert_eq!(A::from_u8(3), Some(A::I2p));
    /// assert_eq!(A::from_u8(4), Some(A::Tor));
    /// assert_eq!(A::from_u8(5), None);
    /// ```
    pub const fn from_u8(u: u8) -> Option<Self> {
        Some(match u {
            0 => Self::Invalid,
            1 => Self::Ipv4,
            2 => Self::Ipv6,
            3 => Self::I2p,
            4 => Self::Tor,
            _ => return None,
        })
    }
}

impl From<AddressType> for u8 {
    fn from(value: AddressType) -> Self {
        value.to_u8()
    }
}

impl TryFrom<u8> for AddressType {
    type Error = u8;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match Self::from_u8(value) {
            Some(s) => Ok(s),
            None => Err(value),
        }
    }
}

#[cfg(feature = "epee")]
impl EpeeValue for AddressType {
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
