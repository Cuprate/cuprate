//---------------------------------------------------------------------------------------------------- Use
use serde::{Deserialize, Deserializer};
use serde::de::{Error, Visitor};
use core::fmt;

//---------------------------------------------------------------------------------------------------- Key
pub(crate) enum Key {
    JsonRpc,
    Result,
    Error,
    Id,
}

struct KeyVisitor;

impl Visitor<'_> for KeyVisitor {
	type Value = Key;

	fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		formatter.write_str("Key field must be a string and one of the following values: ['jsonrpc', 'result', 'error', 'id']")
	}

	fn visit_str<E: Error>(self, text: &str) -> Result<Self::Value, E> {
		if text.eq_ignore_ascii_case("jsonrpc") {
			Ok(Key::JsonRpc)
		} else if text.eq_ignore_ascii_case("result") {
			Ok(Key::Result)
		} else if text.eq_ignore_ascii_case("error") {
			Ok(Key::Error)
		} else if text.eq_ignore_ascii_case("id") {
			Ok(Key::Id)
		} else {
			Err(Error::invalid_value(serde::de::Unexpected::Str(text), &self))
		}
	}
}


impl<'a> Deserialize<'a> for Key {
    fn deserialize<D: Deserializer<'a>>(des: D) -> Result<Self, D::Error> {
        des.deserialize_str(KeyVisitor)
    }
}
