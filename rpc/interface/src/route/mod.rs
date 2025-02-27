//! Routing functions.
//!
//! These are the function signatures passed to
//! [`crate::RouterBuilder`] when registering routes.

pub(crate) mod bin;
pub(crate) mod fallback;
pub(crate) mod json_rpc;
pub(crate) mod other_json;
