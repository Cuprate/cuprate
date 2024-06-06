//! TODO

#![cfg(test)]

//---------------------------------------------------------------------------------------------------- Use
use std::borrow::Cow;

use pretty_assertions::assert_eq;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{to_value, Value};

//---------------------------------------------------------------------------------------------------- Body
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct Body<P> {
    pub(crate) method: Cow<'static, str>,
    pub(crate) params: P,
}

//---------------------------------------------------------------------------------------------------- Free functions
/// TODO
pub(crate) fn assert_serde<T>(t: &T, expected_value: &Value)
where
    T: Serialize + DeserializeOwned + std::fmt::Debug + Clone + PartialEq,
{
    let value = to_value(t).unwrap();
    assert_eq!(value, *expected_value);
}
