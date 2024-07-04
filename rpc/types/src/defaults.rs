//! TODO

//---------------------------------------------------------------------------------------------------- Import
use std::borrow::Cow;

//---------------------------------------------------------------------------------------------------- TODO
/// TODO
#[inline]
pub(crate) const fn default_bool() -> bool {
    false
}

/// TODO
#[inline]
pub(crate) const fn default_cow_str() -> Cow<'static, str> {
    Cow::Borrowed("")
}

/// TODO
#[inline]
pub(crate) const fn default_string() -> String {
    String::new()
}

/// TODO
#[inline]
pub(crate) const fn default_height() -> u64 {
    0
}

/// TODO
#[inline]
pub(crate) const fn default_vec<T>() -> Vec<T> {
    Vec::new()
}

/// TODO
#[inline]
pub(crate) const fn default_u64() -> u64 {
    0
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
