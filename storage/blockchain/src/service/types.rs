//! Database service type aliases.

//---------------------------------------------------------------------------------------------------- Use
use cuprate_database::DbResult;
use cuprate_types::blockchain::{
    BlockchainReadRequest, BlockchainResponse, BlockchainWriteRequest,
};

//---------------------------------------------------------------------------------------------------- Types
/// The actual type of the response.
///
/// Either our [`BlockchainResponse`], or a database error occurred.
pub(super) type ResponseResult = DbResult<BlockchainResponse>;

