//! RPC response status type.

//---------------------------------------------------------------------------------------------------- Import
use std::fmt::Display;

use serde::{Deserialize, Serialize};

// TODO: impl epee
// use epee_encoding::{EpeeObject, EpeeObjectBuilder};

use crate::constants::{
    CORE_RPC_STATUS_BUSY, CORE_RPC_STATUS_NOT_MINING, CORE_RPC_STATUS_OK,
    CORE_RPC_STATUS_PAYMENT_REQUIRED,
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
///     CORE_RPC_STATUS_PAYMENT_REQUIRED,
/// };
/// use serde_json::to_string;
/// use strum::AsRefStr;
///
/// let other = Status::Other("hello".into());
///
/// assert_eq!(to_string(&Status::Ok).unwrap(),              r#""OK""#);
/// assert_eq!(to_string(&Status::Busy).unwrap(),            r#""BUSY""#);
/// assert_eq!(to_string(&Status::NotMining).unwrap(),       r#""NOT MINING""#);
/// assert_eq!(to_string(&Status::PaymentRequired).unwrap(), r#""PAYMENT REQUIRED""#);
/// assert_eq!(to_string(&other).unwrap(),                   r#""hello""#);
///
/// assert_eq!(Status::Ok.as_ref(),              CORE_RPC_STATUS_OK);
/// assert_eq!(Status::Busy.as_ref(),            CORE_RPC_STATUS_BUSY);
/// assert_eq!(Status::NotMining.as_ref(),       CORE_RPC_STATUS_NOT_MINING);
/// assert_eq!(Status::PaymentRequired.as_ref(), CORE_RPC_STATUS_PAYMENT_REQUIRED);
/// assert_eq!(other.as_ref(),                   "hello");
///
/// assert_eq!(format!("{}", Status::Ok),              CORE_RPC_STATUS_OK);
/// assert_eq!(format!("{}", Status::Busy),            CORE_RPC_STATUS_BUSY);
/// assert_eq!(format!("{}", Status::NotMining),       CORE_RPC_STATUS_NOT_MINING);
/// assert_eq!(format!("{}", Status::PaymentRequired), CORE_RPC_STATUS_PAYMENT_REQUIRED);
/// assert_eq!(format!("{}", other),                   "hello");
///
/// assert_eq!(format!("{:?}", Status::Ok),              "Ok");
/// assert_eq!(format!("{:?}", Status::Busy),            "Busy");
/// assert_eq!(format!("{:?}", Status::NotMining),       "NotMining");
/// assert_eq!(format!("{:?}", Status::PaymentRequired), "PaymentRequired");
/// assert_eq!(format!("{:?}", other),                   "Other(\"hello\")");
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Status {
    // FIXME:
    // `#[serde(rename = "")]` onlys takes raw string literals?
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

    #[serde(untagged)]
    /// Some unknown other string.
    ///
    /// This exists to act as a catch-all if `monerod` adds
    /// a string and a Cuprate node hasn't updated yet.
    Other(String),
}

impl AsRef<str> for Status {
    fn as_ref(&self) -> &str {
        match self {
            Self::Ok => CORE_RPC_STATUS_OK,
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

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
