//! [Error codes and objects](https://www.jsonrpc.org/specification#error_object).
//!
//! This module contains JSON-RPC 2.0's error object and codes,
//! as well as some associated constants.

mod code;
mod constants;
mod object;

pub use code::ErrorCode;
pub use constants::{
    INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR, SERVER_ERROR,
};
pub use object::ErrorObject;
