//! TODO

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Struct definitions
/// TODO
pub trait RpcCall {
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
    /// assert_eq!(GetBlockCountRequest::default().is_restricted(), false);
    ///
    /// // Restricted methods, only allowed
    /// // for unrestricted RPC servers (18081).
    /// assert_eq!(GetConnectionsRequest::default().is_restricted(), true);
    /// ```
    fn is_restricted(&self) -> bool;
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
