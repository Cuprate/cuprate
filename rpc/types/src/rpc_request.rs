//! TODO

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Struct definitions
/// TODO
pub trait RpcRequest {
    /// Returns `true` if this method should
    /// only be allowed on local servers.
    ///
    /// If this returns `false`, it should be
    /// okay to execute the method even on restricted
    /// RPC servers.
    ///
    /// ```rust
    /// use cuprate_rpc_types::JsonRpcRequest;
    ///
    /// // Allowed method, even on restricted RPC servers (18089).
    /// assert_eq!(JsonRpcRequest::GetBlockCount(()).is_restricted(), false);
    ///
    /// // Restricted methods, only allowed
    /// // for unrestricted RPC servers (18081).
    /// assert_eq!(JsonRpcRequest::GetConnections(()).is_restricted(), true);
    /// ```
    fn is_restricted(&self) -> bool;
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
