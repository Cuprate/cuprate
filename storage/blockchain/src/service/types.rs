//! Database service type aliases.
//!
//! Only used internally for our `tower::Service` impls.

//---------------------------------------------------------------------------------------------------- Use
use cuprate_database::RuntimeError;
use cuprate_database_service::{DatabaseReadService, DatabaseWriteHandle};
use cuprate_types::blockchain::{BCReadRequest, BCResponse, BCWriteRequest};

//---------------------------------------------------------------------------------------------------- Types
/// The actual type of the response.
///
/// Either our [`BCResponse`], or a database error occurred.
pub(super) type ResponseResult = Result<BCResponse, RuntimeError>;

pub type BCWriteHandle = DatabaseWriteHandle<BCWriteRequest, BCResponse>;

pub type BCReadHandle = DatabaseReadService<BCReadRequest, BCResponse, RuntimeError>;
