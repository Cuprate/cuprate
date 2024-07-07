//! These functions define the default values
//! of optional fields in request/response types.
//!
//! For example, [`crate::json::GetBlockRequest`]
//! has a [`crate::json::GetBlockRequest::height`]
//! field and a [`crate::json::GetBlockRequest::hash`]
//! field, when the RPC interface reads JSON without
//! `height`, it will use [`default_height`] to fill that in.

//---------------------------------------------------------------------------------------------------- Import
use std::borrow::Cow;

//---------------------------------------------------------------------------------------------------- TODO
/// Default [`bool`] type used in request/response types, `false`.
#[inline]
pub(crate) const fn default_bool() -> bool {
    false
}

/// Default `Cow<'static, str` type used in request/response types.
#[inline]
pub(crate) const fn default_cow_str() -> Cow<'static, str> {
    Cow::Borrowed("")
}

/// Default [`String`] type used in request/response types.
#[inline]
pub(crate) const fn default_string() -> String {
    String::new()
}

/// Default block height used in request/response types.
#[inline]
pub(crate) const fn default_height() -> u64 {
    0
}

/// Default [`Vec`] used in request/response types.
#[inline]
pub(crate) const fn default_vec<T>() -> Vec<T> {
    Vec::new()
}

/// Default [`u64`] used in request/response types.
#[inline]
pub(crate) const fn default_u64() -> u64 {
    0
}

/// Default [`u8`] used in request/response types.
#[inline]
pub(crate) const fn default_u8() -> u8 {
    0
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
