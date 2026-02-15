//! RPC response status type.

//---------------------------------------------------------------------------------------------------- Import
use std::{borrow::Cow, fmt::Display};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::{
    macros::bytes::{Buf, BufMut},
    EpeeValue, Marker,
};

use crate::constants::{
    CORE_RPC_STATUS_BUSY, CORE_RPC_STATUS_FAILED, CORE_RPC_STATUS_NOT_MINING, CORE_RPC_STATUS_OK,
    CORE_RPC_STATUS_PAYMENT_REQUIRED,
};

//---------------------------------------------------------------------------------------------------- Status
// TODO: this type needs to expand more.
// There are a lot of RPC calls that will return a random
// string inside, which isn't compatible with [`Status`].

/// RPC response status.
///
/// This type represents `monerod`'s frequently appearing string field, `status`.
///
/// Reference: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L78-L81>.
///
/// ## Serialization and string formatting
/// ```rust
/// use cuprate_rpc_types::{
///     misc::Status,
///     CORE_RPC_STATUS_BUSY, CORE_RPC_STATUS_NOT_MINING, CORE_RPC_STATUS_OK,
///     CORE_RPC_STATUS_PAYMENT_REQUIRED, CORE_RPC_STATUS_FAILED
/// };
/// use serde_json::{to_string, from_str};
///
/// let other = Status::Other("OTHER".into());
///
/// assert_eq!(to_string(&Status::Ok).unwrap(),              r#""OK""#);
/// assert_eq!(to_string(&Status::Failed).unwrap(),          r#""Failed""#);
/// assert_eq!(to_string(&Status::Busy).unwrap(),            r#""BUSY""#);
/// assert_eq!(to_string(&Status::NotMining).unwrap(),       r#""NOT MINING""#);
/// assert_eq!(to_string(&Status::PaymentRequired).unwrap(), r#""PAYMENT REQUIRED""#);
/// assert_eq!(to_string(&other).unwrap(),                   r#""OTHER""#);
///
/// assert_eq!(from_str::<Status>(r#""Ok""#).unwrap(),               Status::Ok);
/// assert_eq!(from_str::<Status>(r#""OK""#).unwrap(),               Status::Ok);
/// assert_eq!(from_str::<Status>(r#""Failed""#).unwrap(),           Status::Failed);
/// assert_eq!(from_str::<Status>(r#""FAILED""#).unwrap(),           Status::Failed);
/// assert_eq!(from_str::<Status>(r#""Busy""#).unwrap(),             Status::Busy);
/// assert_eq!(from_str::<Status>(r#""BUSY""#).unwrap(),             Status::Busy);
/// assert_eq!(from_str::<Status>(r#""NOT MINING""#).unwrap(),       Status::NotMining);
/// assert_eq!(from_str::<Status>(r#""PAYMENT REQUIRED""#).unwrap(), Status::PaymentRequired);
/// assert_eq!(from_str::<Status>(r#""OTHER""#).unwrap(),            other);
///
/// assert_eq!(Status::Ok.as_ref(),              CORE_RPC_STATUS_OK);
/// assert_eq!(Status::Failed.as_ref(),          CORE_RPC_STATUS_FAILED);
/// assert_eq!(Status::Busy.as_ref(),            CORE_RPC_STATUS_BUSY);
/// assert_eq!(Status::NotMining.as_ref(),       CORE_RPC_STATUS_NOT_MINING);
/// assert_eq!(Status::PaymentRequired.as_ref(), CORE_RPC_STATUS_PAYMENT_REQUIRED);
/// assert_eq!(other.as_ref(),                   "OTHER");
///
/// assert_eq!(format!("{}", Status::Ok),              CORE_RPC_STATUS_OK);
/// assert_eq!(format!("{}", Status::Failed),          CORE_RPC_STATUS_FAILED);
/// assert_eq!(format!("{}", Status::Busy),            CORE_RPC_STATUS_BUSY);
/// assert_eq!(format!("{}", Status::NotMining),       CORE_RPC_STATUS_NOT_MINING);
/// assert_eq!(format!("{}", Status::PaymentRequired), CORE_RPC_STATUS_PAYMENT_REQUIRED);
/// assert_eq!(format!("{}", other),                   "OTHER");
///
/// assert_eq!(format!("{:?}", Status::Ok),              "Ok");
/// assert_eq!(format!("{:?}", Status::Failed),          "Failed");
/// assert_eq!(format!("{:?}", Status::Busy),            "Busy");
/// assert_eq!(format!("{:?}", Status::NotMining),       "NotMining");
/// assert_eq!(format!("{:?}", Status::PaymentRequired), "PaymentRequired");
/// assert_eq!(format!("{:?}", other),                   r#"Other("OTHER")"#);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Status {
    // FIXME:
    // `#[serde(rename = "")]` only takes raw string literals?
    // We have to re-type the constants here...
    /// Successful RPC response, everything is OK; [`CORE_RPC_STATUS_OK`].
    #[cfg_attr(feature = "serde", serde(rename = "OK", alias = "Ok"))]
    #[default]
    Ok,

    /// Generic request failure.
    #[cfg_attr(feature = "serde", serde(alias = "FAILED"))]
    Failed,

    /// The daemon is busy, try later; [`CORE_RPC_STATUS_BUSY`].
    #[cfg_attr(feature = "serde", serde(rename = "BUSY", alias = "Busy"))]
    Busy,

    /// The daemon is not mining; [`CORE_RPC_STATUS_NOT_MINING`].
    #[cfg_attr(feature = "serde", serde(rename = "NOT MINING"))]
    NotMining,

    /// Payment is required for RPC; [`CORE_RPC_STATUS_PAYMENT_REQUIRED`].
    #[cfg_attr(feature = "serde", serde(rename = "PAYMENT REQUIRED"))]
    PaymentRequired,

    /// Some unknown other string.
    ///
    /// This exists to act as a catch-all for all of
    /// `monerod`'s other strings it puts in the `status` field.
    #[cfg_attr(feature = "serde", serde(rename = "OTHER"), serde(untagged))]
    Other(Cow<'static, str>),
}

impl From<String> for Status {
    fn from(s: String) -> Self {
        match s.as_str() {
            CORE_RPC_STATUS_OK => Self::Ok,
            CORE_RPC_STATUS_BUSY => Self::Busy,
            CORE_RPC_STATUS_NOT_MINING => Self::NotMining,
            CORE_RPC_STATUS_PAYMENT_REQUIRED => Self::PaymentRequired,
            CORE_RPC_STATUS_FAILED => Self::Failed,
            _ => Self::Other(Cow::Owned(s)),
        }
    }
}

impl AsRef<str> for Status {
    fn as_ref(&self) -> &str {
        match self {
            Self::Ok => CORE_RPC_STATUS_OK,
            Self::Failed => CORE_RPC_STATUS_FAILED,
            Self::Busy => CORE_RPC_STATUS_BUSY,
            Self::NotMining => CORE_RPC_STATUS_NOT_MINING,
            Self::PaymentRequired => CORE_RPC_STATUS_PAYMENT_REQUIRED,
            Self::Other(s) => s,
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

// [`Status`] is essentially a [`String`] when it comes to
// (de)serialization, except when writing we usually have
// access to a `&'static str` and don't need to allocate.
//
// See below for more impl info:
// <https://github.com/Cuprate/cuprate/blob/bef2a2cbd4e1194991751d1fbc96603cba8c7a51/net/epee-encoding/src/value.rs#L366-L392>.
#[cfg(feature = "epee")]
impl EpeeValue for Status {
    const MARKER: Marker = <String as EpeeValue>::MARKER;

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> cuprate_epee_encoding::Result<Self> {
        let string = <String as EpeeValue>::read(r, marker)?;
        Ok(Self::from(string))
    }

    fn should_write(&self) -> bool {
        true
    }

    fn epee_default_value() -> Option<Self> {
        // <https://github.com/Cuprate/cuprate/pull/147#discussion_r1654992559>
        Some(Self::Other(Cow::Borrowed("")))
    }

    fn write<B: BufMut>(self, w: &mut B) -> cuprate_epee_encoding::Result<()> {
        cuprate_epee_encoding::write_bytes(self.as_ref(), w)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    use super::*;

    // Test epee (de)serialization works.
    #[test]
    #[cfg(feature = "epee")]
    fn epee() {
        for status in [
            Status::Ok,
            Status::Busy,
            Status::NotMining,
            Status::PaymentRequired,
            Status::Other(Cow::Borrowed("")),
        ] {
            let mut buf = vec![];

            <Status as EpeeValue>::write(status.clone(), &mut buf).unwrap();
            let status2 =
                <Status as EpeeValue>::read(&mut buf.as_slice(), &<Status as EpeeValue>::MARKER)
                    .unwrap();

            assert_eq!(status, status2);
        }
    }
}
