//! The base data that appear in many RPC request/responses.
//!
//! TODO

mod access_request_base;
mod access_response_base;
mod empty_request_base;
mod empty_response_base;
mod response_base;

pub use access_request_base::AccessRequestBase;
pub use access_response_base::AccessResponseBase;
pub use empty_request_base::EmptyRequestBase;
pub use empty_response_base::EmptyResponseBase;
pub use response_base::ResponseBase;
