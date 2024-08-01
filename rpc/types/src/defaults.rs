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
pub(crate) const fn default_false() -> bool {
    false
}

/// Default [`bool`] type used in _some_ request/response types, `true`.
#[inline]
pub(crate) const fn default_true() -> bool {
    true
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

/// Default `0` value used in request/response types.
#[inline]
pub(crate) fn default_zero<T: From<u8>>() -> T {
    T::from(0)
}

/// Default `1` value used in request/response types.
#[inline]
pub(crate) fn default_one<T: From<u8>>() -> T {
    T::from(1)
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    use super::*;

    /// Tests that [`default_zero`] returns `0` on all unsigned numbers.
    #[test]
    fn zero() {
        assert_eq!(default_zero::<usize>(), 0);
        assert_eq!(default_zero::<u64>(), 0);
        assert_eq!(default_zero::<u32>(), 0);
        assert_eq!(default_zero::<u16>(), 0);
        assert_eq!(default_zero::<u8>(), 0);
    }
}
