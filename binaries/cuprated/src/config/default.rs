use serde::{de::Error, Deserialize, Deserializer, Serialize};

/// An enum that can be either a default value or a custom value.
///
/// Useful when config value's defaults depend on other values, i.e. the default ports to listen on
/// depend on the network chosen.
///
/// The [`DefaultOrCustom::Default`] variant will be serialised as a string: "Default",
/// [`DefaultOrCustom::Custom`] will just use the serialisation of the inner value.
///
/// Deserialisation of the [`DefaultOrCustom::Default`] variant ignores ASCII case,
/// so "default" and "DEFAULT" are also accepted. This means `T` must not be a
/// string-like type, otherwise the string "default" would never reach it.
#[derive(Copy, Clone, Debug, Serialize, PartialEq, Eq)]
pub enum DefaultOrCustom<T> {
    Default,
    #[serde(untagged)]
    Custom(T),
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for DefaultOrCustom<T> {
    /// This is implemented manually (instead of derived with `#[serde(untagged)]`)
    /// so that the "Default" keyword is matched case-insensitively, see:
    /// <https://github.com/Cuprate/cuprate/issues/598>.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = toml::Value::deserialize(deserializer)?;

        let is_default =
            matches!(&value, toml::Value::String(s) if s.eq_ignore_ascii_case("default"));

        if is_default {
            Ok(Self::Default)
        } else {
            T::deserialize(value)
                .map(Self::Custom)
                .map_err(D::Error::custom)
        }
    }
}

impl<T> DefaultOrCustom<T> {
    /// Returns the given default value if this is [`DefaultOrCustom::Default`], otherwise returns
    /// the custom value.
    pub const fn value<'a>(&'a self, default: &'a T) -> &'a T {
        match self {
            Self::Default => default,
            Self::Custom(value) => value,
        }
    }
}
