//! These functions define the default values
//! of optional fields in request/response types.
//!
//! For example, [`crate::json::GetBlockRequest`]
//! has a [`crate::json::GetBlockRequest::height`]
//! field and a [`crate::json::GetBlockRequest::hash`]
//! field, when the RPC interface reads JSON without
//! `height`, it will use [`default`] to fill that in.

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- TODO
/// Default [`bool`] type used in _some_ request/response types, `true`.
#[inline]
pub(crate) const fn default_true() -> bool {
    true
}

/// Default `1` value used in request/response types.
#[inline]
pub(crate) fn default_one<T: From<u8>>() -> T {
    T::from(1)
}

/// Generate a default `T` to be used in request/response types.
#[inline]
pub(crate) fn default<T: Default>() -> T {
    T::default()
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}
