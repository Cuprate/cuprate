//! Database service type aliases.
//!
//! Only used internally for our [`tower::Service`] impls.

use cuprate_database::RuntimeError;
use cuprate_database_service::{DatabaseReadService, DatabaseWriteHandle};

use crate::service::interface::{
    TxpoolReadRequest, TxpoolReadResponse, TxpoolWriteRequest, TxpoolWriteResponse,
};

/// The actual type of the response.
///
/// Either our [`TxpoolReadResponse`], or a database error occurred.
pub(super) type ReadResponseResult = Result<TxpoolReadResponse, RuntimeError>;

/// The transaction pool database write service.
pub type TxpoolWriteHandle = DatabaseWriteHandle<TxpoolWriteRequest, TxpoolWriteResponse>;

/// The transaction pool database read service.
pub type TxpoolReadHandle = DatabaseReadService<TxpoolReadRequest, TxpoolReadResponse>;
