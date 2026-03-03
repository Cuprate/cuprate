//! [`tower::Service`] integration + thread-pool.
//!
//! ## `service`
//! The `service` module implements the [`tower`] integration,
//! along with the reader/writer thread-pool system.
//!
//! The thread-pool allows outside crates to communicate with it by
//! sending database requests and receiving responses `async`hronously -
//! without having to actually worry and handle the database themselves.
//!
//! The system is managed by this crate, and only requires initialisation by the user.
//!
// needed for docs
use tower as _;

use cuprate_types::blockchain::BlockchainResponse;

use crate::{error::DbResult, BlockchainDatabase};

mod free;
mod read;
mod write;

pub use free::init_with_pool;
pub use read::BlockchainReadHandle;
pub use write::{init_write_service, BlockchainWriteHandle};

//---------------------------------------------------------------------------------------------------- Types
/// The actual type of the response.
///
/// Either our [`BlockchainResponse`], or a database error occurred.
pub(super) type ResponseResult = DbResult<BlockchainResponse>;
