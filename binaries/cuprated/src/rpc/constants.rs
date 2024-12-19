//! Constants used within RPC.

/// The string message used in RPC response fields for when
/// `cuprated` does not support a field that `monerod` has.
pub(super) const FIELD_NOT_SUPPORTED: &str = "`cuprated` does not support this field.";

/// The error message returned when an unsupported RPC call is requested.
pub(super) const UNSUPPORTED_RPC_CALL: &str = "This RPC call is not supported by Cuprate.";
