//! Database service type aliases.

//---------------------------------------------------------------------------------------------------- Use
use cuprate_database::RuntimeError;
use cuprate_database_service::{DatabaseReadService, DatabaseWriteHandle};
use cuprate_types::blockchain::{BCReadRequest, BCResponse, BCWriteRequest};

//---------------------------------------------------------------------------------------------------- Types
/// The actual type of the response.
///
/// Either our [`BCResponse`], or a database error occurred.
pub(super) type ResponseResult = Result<BCResponse, RuntimeError>;

/// The blockchain database write service.
pub type BCWriteHandle = DatabaseWriteHandle<BCWriteRequest, BCResponse>;

/// The blockchain database read service.
pub type BCReadHandle = DatabaseReadService<BCReadRequest, BCResponse>;
