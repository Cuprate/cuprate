//! RPC response status type.

//---------------------------------------------------------------------------------------------------- Import
use serde::{Deserialize, Serialize};
use strum::{
    AsRefStr, Display, EnumCount, EnumIs, EnumIter, EnumMessage, EnumProperty, EnumString,
    EnumTryAs, FromRepr, IntoStaticStr, VariantArray, VariantNames,
};

//---------------------------------------------------------------------------------------------------- TODO
/// RPC response status.
///
/// This type represents `monerod`'s frequently appearing string field, `status`.
///
/// This field appears within RPC [JSON response](crate::json) types.
///
/// Reference: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/message.cpp#L40-L44>.
///
/// ## Serialization and string formatting
/// ```rust
/// # use cuprate_rpc_types::*;
/// use serde_json::to_string;
/// use strum::AsRefStr;
///
/// assert_eq!(to_string(&Status::Ok).unwrap(),         r#""OK""#);
/// assert_eq!(to_string(&Status::Retry).unwrap(),      r#""Retry""#);
/// assert_eq!(to_string(&Status::Failed).unwrap(),     r#""Failed""#);
/// assert_eq!(to_string(&Status::BadRequest).unwrap(), r#""Invalid request type""#);
/// assert_eq!(to_string(&Status::BadJson).unwrap(),    r#""Malformed json""#);
///
/// assert_eq!(Status::Ok.as_ref(),         "OK");
/// assert_eq!(Status::Retry.as_ref(),      "Retry");
/// assert_eq!(Status::Failed.as_ref(),     "Failed");
/// assert_eq!(Status::BadRequest.as_ref(), "Invalid request type");
/// assert_eq!(Status::BadJson.as_ref(),    "Malformed json");
///
/// assert_eq!(format!("{}", Status::Ok),         "OK");
/// assert_eq!(format!("{}", Status::Retry),      "Retry");
/// assert_eq!(format!("{}", Status::Failed),     "Failed");
/// assert_eq!(format!("{}", Status::BadRequest), "Invalid request type");
/// assert_eq!(format!("{}", Status::BadJson),    "Malformed json");
///
/// assert_eq!(format!("{:?}", Status::Ok),         "Ok");
/// assert_eq!(format!("{:?}", Status::Retry),      "Retry");
/// assert_eq!(format!("{:?}", Status::Failed),     "Failed");
/// assert_eq!(format!("{:?}", Status::BadRequest), "BadRequest");
/// assert_eq!(format!("{:?}", Status::BadJson),    "BadJson");
///
/// assert_eq!(format!("{:#?}", Status::Ok),         "Ok");
/// assert_eq!(format!("{:#?}", Status::Retry),      "Retry");
/// assert_eq!(format!("{:#?}", Status::Failed),     "Failed");
/// assert_eq!(format!("{:#?}", Status::BadRequest), "BadRequest");
/// assert_eq!(format!("{:#?}", Status::BadJson),    "BadJson");
/// ```
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    AsRefStr,
    Display,
    EnumCount,
    EnumIs,
    EnumIter,
    EnumMessage,
    EnumProperty,
    EnumString,
    EnumTryAs,
    FromRepr,
    IntoStaticStr,
    VariantArray,
    VariantNames,
    Serialize,
    Deserialize,
)]
pub enum Status {
    /// Successful RPC response, everything is OK.
    #[strum(serialize = "OK")]
    #[serde(rename = "OK", alias = "Ok", alias = "ok")]
    #[default]
    Ok,

    #[serde(alias = "Retry", alias = "RETRY", alias = "retry")]
    /// The RPC call failed and should be retried.
    ///
    /// TODO: confirm this.
    Retry,

    #[serde(alias = "failed", alias = "FAILED")]
    /// The RPC call failed.
    Failed,

    /// The RPC call contained bad input, unknown method, unknown params, etc.
    #[strum(serialize = "Invalid request type")]
    #[serde(
        rename = "Invalid request type",
        alias = "invalid request type",
        alias = "INVALID REQUEST TYPE"
    )]
    BadRequest,

    /// The RPC call contained malformed JSON.
    #[strum(serialize = "Malformed json")]
    #[serde(
        rename = "Malformed json",
        alias = "malformed json",
        alias = "MALFORMED JSON",
        alias = "Malformed JSON",
        alias = "malformed JSON"
    )]
    BadJson,
    // TODO:
    // This may not be all the string `monerod` uses.
    // We could use an `Other(String)` here just in case,
    // otherwise deserialization would fail.
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
