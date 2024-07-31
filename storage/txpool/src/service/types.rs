//! Database service type aliases.
//!
//! Only used internally for our [`tower::Service`] impls.

use cuprate_database_service::{DatabaseReadService, DatabaseWriteHandle};

use crate::service::interface::{
    TxpoolReadRequest, TxpoolReadResponse, TxpoolWriteRequest, TxpoolWriteResponse,
};

/// The transaction pool database write service.
pub type TxPoolWriteHandle = DatabaseWriteHandle<TxpoolWriteRequest, TxpoolWriteResponse>;

/// The transaction pool database read service.
pub type TxPoolReadHandle = DatabaseReadService<TxpoolReadRequest, TxpoolReadResponse>;
