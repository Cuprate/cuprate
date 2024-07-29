//! TODO

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Struct definitions
/// TODO
pub trait RpcCall {
    /// Is `true` if this RPC method should
    /// only be allowed on local servers.
    ///
    /// If this is `false`, it should be
    /// okay to execute the method even on restricted
    /// RPC servers.
    ///
    /// ```rust
    /// use cuprate_rpc_types::{
    ///     RpcCall,
    ///     json::{GetBlockCountRequest, GetConnectionsRequest},
    /// };
    ///
    /// // Allowed method, even on restricted RPC servers (18089).
    /// assert_eq!(GetBlockCountRequest::IS_RESTRICTED.is_some_and(|x| !x));
    ///
    /// // Restricted methods, only allowed
    /// // for unrestricted RPC servers (18081).
    /// assert_eq!(GetConnectionsRequest::IS_RESTRICTED.is_some_and(|x| x));
    /// ```
    const IS_RESTRICTED: bool;

    /// TODO
    const IS_EMPTY: bool;
}

/// TODO
pub trait RpcCallValue {
    /// Returns `true` if this RPC method should
    /// only be allowed on local servers.
    ///
    /// If this returns `false`, it should be
    /// okay to execute the method even on restricted
    /// RPC servers.
    ///
    /// ```rust
    /// use cuprate_rpc_types::{
    ///     RpcCall,
    ///     json::{GetBlockCountRequest, GetConnectionsRequest},
    /// };
    ///
    /// // Allowed method, even on restricted RPC servers (18089).
    /// assert_eq!(GetBlockCountRequest::IS_RESTRICTED.is_some_and(|x| !x));
    ///
    /// // Restricted methods, only allowed
    /// // for unrestricted RPC servers (18081).
    /// assert_eq!(GetConnectionsRequest::IS_RESTRICTED.is_some_and(|x| x));
    /// ```
    fn is_restricted(&self) -> bool;

    /// TODO
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
