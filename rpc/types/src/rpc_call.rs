//! RPC call metadata.

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- RpcCall
/// Metadata about an RPC call.
///
/// This trait describes some metadata about RPC requests.
///
/// It is implemented on all request types within:
/// - [`crate::json`]
/// - [`crate::other`]
/// - [`crate::bin`]
///
/// See also [`RpcCallValue`] for a dynamic by-value version of this trait.
pub trait RpcCall {
    /// Is `true` if this RPC method should
    /// only be allowed on local servers.
    ///
    /// If this is `false`, it should be
    /// okay to execute the method even on restricted
    /// RPC servers.
    ///
    /// ```rust
    /// use cuprate_rpc_types::{RpcCall, json::*};
    ///
    /// // Allowed method, even on restricted RPC servers (18089).
    /// assert!(!GetBlockCountRequest::IS_RESTRICTED);
    ///
    /// // Restricted methods, only allowed
    /// // for unrestricted RPC servers (18081).
    /// assert!(GetConnectionsRequest::IS_RESTRICTED);
    /// ```
    const IS_RESTRICTED: bool;

    /// Is `true` if this RPC method has no inputs, i.e. it is a `struct` with no fields.
    ///
    /// ```rust
    /// use cuprate_rpc_types::{RpcCall, json::*};
    ///
    /// assert!(GetBlockCountRequest::IS_EMPTY);
    /// assert!(!OnGetBlockHashRequest::IS_EMPTY);
    /// ```
    const IS_EMPTY: bool;
}

//---------------------------------------------------------------------------------------------------- RpcCallValue
/// By-value version of [`RpcCall`].
///
/// This trait is a mirror of [`RpcCall`],
/// except it takes `self` by value instead
/// of being a `const` property.
///
/// This exists for `enum`s where requests must be dynamically
/// `match`ed like [`JsonRpcRequest`](crate::json::JsonRpcRequest).
///
/// All types that implement [`RpcCall`] automatically implement [`RpcCallValue`].
pub trait RpcCallValue {
    /// Same as [`RpcCall::IS_RESTRICTED`].
    ///
    /// ```rust
    /// use cuprate_rpc_types::{RpcCallValue, json::*};
    ///
    /// assert!(!GetBlockCountRequest::default().is_restricted());
    /// assert!(GetConnectionsRequest::default().is_restricted());
    /// ```
    fn is_restricted(&self) -> bool;

    /// Same as [`RpcCall::IS_EMPTY`].
    ///
    /// ```rust
    /// use cuprate_rpc_types::{RpcCallValue, json::*};
    ///
    /// assert!(GetBlockCountRequest::default().is_empty());
    /// assert!(!OnGetBlockHashRequest::default().is_empty());
    /// ```
    fn is_empty(&self) -> bool;
}

impl<T: RpcCall> RpcCallValue for T {
    #[inline]
    fn is_restricted(&self) -> bool {
        Self::IS_RESTRICTED
    }

    #[inline]
    fn is_empty(&self) -> bool {
        Self::IS_EMPTY
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
