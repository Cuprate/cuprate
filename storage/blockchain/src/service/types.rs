//! Database service type aliases.

//---------------------------------------------------------------------------------------------------- Use
use cuprate_database::RuntimeError;
use cuprate_database_service::{DatabaseReadService, DatabaseWriteHandle};
use cuprate_types::blockchain::{BlockchainReadRequest, BlockchainResponse, BlockchainWriteRequest};

//---------------------------------------------------------------------------------------------------- Types
/// The actual type of the response.
///
/// Either our [`BlockchainResponse`], or a database error occurred.
pub(super) type ResponseResult = Result<BlockchainResponse, RuntimeError>;

/// The blockchain database write service.
pub type BlockchainWriteHandle = DatabaseWriteHandle<BlockchainWriteRequest, BlockchainResponse>;

/// The blockchain database read service.
pub type BlockchainReadHandle = DatabaseReadService<BlockchainReadRequest, BlockchainResponse>;
