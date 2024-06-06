//! TODO

#![cfg(test)]

//---------------------------------------------------------------------------------------------------- Use
use std::borrow::Cow;

use pretty_assertions::assert_eq;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{from_str, to_string, to_string_pretty, to_value, Value};

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
    println!("assert_serde: to_string()");
    let string = to_string(t).unwrap();
    let t2: T = from_str(&string).unwrap();
    assert_eq!(*t, t2);

    println!("assert_serde: to_string_pretty()");
    let string = to_string_pretty(t).unwrap();
    let t2: T = from_str(&string).unwrap();
    assert_eq!(*t, t2);

    println!("assert_serde: to_value()");
    let value = to_value(t).unwrap();
    assert_eq!(value, *expected_value);
}
