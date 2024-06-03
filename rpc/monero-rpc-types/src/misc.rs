//! TODO.

//---------------------------------------------------------------------------------------------------- Import
use strum::{
    AsRefStr, Display, EnumCount, EnumIs, EnumIter, EnumMessage, EnumProperty, EnumString,
    EnumTryAs, FromRepr, IntoStaticStr, VariantArray, VariantNames,
};

//---------------------------------------------------------------------------------------------------- TODO
/// TODO
///
/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/message.cpp#L40-L44>.
///
/// ## String formatting
/// ```rust
/// # use monero_rpc_types::misc::*;
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
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Status {
    /// TODO
    #[strum(serialize = "OK")]
    #[cfg_attr(feature = "serde", serde(rename = "OK"))]
    #[default]
    Ok,

    /// TODO
    Retry,

    /// TODO
    Failed,

    /// TODO
    #[strum(serialize = "Invalid request type")]
    #[cfg_attr(feature = "serde", serde(rename = "Invalid request type"))]
    BadRequest,

    /// TODO
    #[strum(serialize = "Malformed json")]
    #[cfg_attr(feature = "serde", serde(rename = "Malformed json"))]
    BadJson,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
