use serde::{Deserialize, Serialize};

/// An enum that can be either a default value or a custom value.
///
/// Useful when config value's defaults depend on other values, i.e. the default ports to listen on
/// depend on the network chosen.
///
/// The [`DefaultOrCustom::Default`] variant will be serialised as a string: "Default",
/// [`DefaultOrCustom::Custom`] will just use the serialisation of the inner value.
#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum DefaultOrCustom<T> {
    Default,
    #[serde(untagged)]
    Custom(T),
}

impl<T> DefaultOrCustom<T> {
    pub fn get_value<'a>(&'a self, default: &'a T) -> &'a T {
        match self {
            DefaultOrCustom::Default => default,
            DefaultOrCustom::Custom(value) => value,
        }
    }
}
