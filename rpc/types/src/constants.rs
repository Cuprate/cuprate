//! TODO

// From: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L83-L89>
//
// ```
// When making *any* change here, bump minor
// If the change is incompatible, then bump major and set minor to 0
// This ensures CORE_RPC_VERSION always increases, that every change
// has its own version, and that clients can just test major to see
// whether they can talk to a given daemon without having to know in
// advance which version they will stop working with
// Don't go over 32767 for any of these
// ```
//
// What this means for Cuprate: just follow `monerod`.

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Status
// Common RPC status strings:
// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L78-L81>.
//
// Note that these are _distinct_ from the ones in ZMQ:
// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/message.cpp#L40-L44>.

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L78>
pub const CORE_RPC_STATUS_OK: &str = "OK";

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L79>
pub const CORE_RPC_STATUS_BUSY: &str = "BUSY";

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L80>
pub const CORE_RPC_STATUS_NOT_MINING: &str = "NOT MINING";

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L81>
pub const CORE_RPC_STATUS_PAYMENT_REQUIRED: &str = "PAYMENT REQUIRED";

/// Custom `CORE_RPC_STATUS` for usage in Cuprate.
pub const CORE_RPC_STATUS_UNKNOWN: &str = "UNKNOWN";

//---------------------------------------------------------------------------------------------------- Versions
/// RPC major version.
///
/// See: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L90>.
pub const CORE_RPC_VERSION_MAJOR: u32 = 3;

/// RPC miror version.
///
/// See: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L91>.
pub const CORE_RPC_VERSION_MINOR: u32 = 14;

/// RPC version.
///
/// See: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L92-L93>.
///
/// ```rust
/// assert_eq!(cuprate_rpc_types::CORE_RPC_VERSION, 196_622);
/// ```
pub const CORE_RPC_VERSION: u32 = (CORE_RPC_VERSION_MAJOR << 16) | CORE_RPC_VERSION_MINOR;

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
