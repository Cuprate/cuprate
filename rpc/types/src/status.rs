//! RPC response status type.

//---------------------------------------------------------------------------------------------------- Import
use std::fmt::Display;

use serde::{Deserialize, Serialize};

use cuprate_epee_encoding::{
    macros::bytes::{Buf, BufMut},
    EpeeValue, Marker,
};

use crate::constants::{
    CORE_RPC_STATUS_BUSY, CORE_RPC_STATUS_NOT_MINING, CORE_RPC_STATUS_OK,
    CORE_RPC_STATUS_PAYMENT_REQUIRED, CORE_RPC_STATUS_UNKNOWN,
};

//---------------------------------------------------------------------------------------------------- Status
/// RPC response status.
///
/// This type represents `monerod`'s frequently appearing string field, `status`.
///
/// This field appears within RPC [JSON response](crate::json) types.
///
/// Reference: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L78-L81>.
///
/// ## Serialization and string formatting
/// ```rust
/// use cuprate_rpc_types::{
///     Status,
///     CORE_RPC_STATUS_BUSY, CORE_RPC_STATUS_NOT_MINING, CORE_RPC_STATUS_OK,
///     CORE_RPC_STATUS_PAYMENT_REQUIRED, CORE_RPC_STATUS_UNKNOWN
/// };
/// use serde_json::to_string;
///
/// let unknown = Status::Unknown;
///
/// assert_eq!(to_string(&Status::Ok).unwrap(),              r#""OK""#);
/// assert_eq!(to_string(&Status::Busy).unwrap(),            r#""BUSY""#);
/// assert_eq!(to_string(&Status::NotMining).unwrap(),       r#""NOT MINING""#);
/// assert_eq!(to_string(&Status::PaymentRequired).unwrap(), r#""PAYMENT REQUIRED""#);
/// assert_eq!(to_string(&unknown).unwrap(),                 r#""UNKNOWN""#);
///
/// assert_eq!(Status::Ok.as_ref(),              CORE_RPC_STATUS_OK);
/// assert_eq!(Status::Busy.as_ref(),            CORE_RPC_STATUS_BUSY);
/// assert_eq!(Status::NotMining.as_ref(),       CORE_RPC_STATUS_NOT_MINING);
/// assert_eq!(Status::PaymentRequired.as_ref(), CORE_RPC_STATUS_PAYMENT_REQUIRED);
/// assert_eq!(unknown.as_ref(),                 CORE_RPC_STATUS_UNKNOWN);
///
/// assert_eq!(format!("{}", Status::Ok),              CORE_RPC_STATUS_OK);
/// assert_eq!(format!("{}", Status::Busy),            CORE_RPC_STATUS_BUSY);
/// assert_eq!(format!("{}", Status::NotMining),       CORE_RPC_STATUS_NOT_MINING);
/// assert_eq!(format!("{}", Status::PaymentRequired), CORE_RPC_STATUS_PAYMENT_REQUIRED);
/// assert_eq!(format!("{}", unknown),                 CORE_RPC_STATUS_UNKNOWN);
///
/// assert_eq!(format!("{:?}", Status::Ok),              "Ok");
/// assert_eq!(format!("{:?}", Status::Busy),            "Busy");
/// assert_eq!(format!("{:?}", Status::NotMining),       "NotMining");
/// assert_eq!(format!("{:?}", Status::PaymentRequired), "PaymentRequired");
/// assert_eq!(format!("{:?}", unknown),                 "Unknown");
/// ```
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Status {
    // FIXME:
    // `#[serde(rename = "")]` only takes raw string literals?
    // We have to re-type the constants here...
    /// Successful RPC response, everything is OK; [`CORE_RPC_STATUS_OK`].
    #[serde(rename = "OK")]
    #[default]
    Ok,

    /// The daemon is busy, try later; [`CORE_RPC_STATUS_BUSY`].
    #[serde(rename = "BUSY")]
    Busy,

    /// The daemon is not mining; [`CORE_RPC_STATUS_NOT_MINING`].
    #[serde(rename = "NOT MINING")]
    NotMining,

    /// Payment is required for RPC; [`CORE_RPC_STATUS_PAYMENT_REQUIRED`].
    #[serde(rename = "PAYMENT REQUIRED")]
    PaymentRequired,

    /// Some unknown other string; [`CORE_RPC_STATUS_UNKNOWN`].
    ///
    /// This exists to act as a catch-all if `monerod` adds
    /// a string and a Cuprate node hasn't updated yet.
    ///
    /// The reason this isn't `Unknown(String)` is because that
    /// disallows [`Status`] to be [`Copy`], and thus other types
    /// that contain it.
    #[serde(other)]
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

impl From<String> for Status {
    fn from(s: String) -> Self {
        match s.as_str() {
            CORE_RPC_STATUS_OK => Self::Ok,
            CORE_RPC_STATUS_BUSY => Self::Busy,
            CORE_RPC_STATUS_NOT_MINING => Self::NotMining,
            CORE_RPC_STATUS_PAYMENT_REQUIRED => Self::PaymentRequired,
            _ => Self::Unknown,
        }
    }
}

impl AsRef<str> for Status {
    fn as_ref(&self) -> &str {
        match self {
            Self::Ok => CORE_RPC_STATUS_OK,
            Self::Busy => CORE_RPC_STATUS_BUSY,
            Self::NotMining => CORE_RPC_STATUS_NOT_MINING,
            Self::PaymentRequired => CORE_RPC_STATUS_PAYMENT_REQUIRED,
            Self::Unknown => CORE_RPC_STATUS_UNKNOWN,
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
        // TODO: what is the default here?
        Some(Self::default())
    }

    fn write<B: BufMut>(self, w: &mut B) -> cuprate_epee_encoding::Result<()> {
        cuprate_epee_encoding::write_bytes(self.as_ref(), w)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
