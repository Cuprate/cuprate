use crate::{
    service::{
        interface::{TxpoolWriteRequest, TxpoolWriteResponse},
        types::TxpoolWriteHandle,
    },
    TxPoolWriteError,
};
use cuprate_database::{ConcreteEnv, RuntimeError};
use cuprate_database_service::DatabaseWriteHandle;
use cuprate_types::blockchain::BCWriteRequest;
use std::sync::Arc;

//---------------------------------------------------------------------------------------------------- init_write_service
/// Initialize the txpool write service from a [`ConcreteEnv`].
pub fn init_write_service(env: Arc<ConcreteEnv>) -> TxpoolWriteHandle {
    DatabaseWriteHandle::init(env, handle_txpool_request)
}

//---------------------------------------------------------------------------------------------------- handle_txpool_request
/// Handle an incoming [`TxpoolWriteRequest`], returning a [`TxpoolWriteResponse`].
fn handle_txpool_request(
    env: &ConcreteEnv,
    req: &TxpoolWriteRequest,
) -> Result<TxpoolWriteResponse, TxPoolWriteError> {
    match req {
        TxpoolWriteRequest::AddTransaction { .. } => todo!(),
        _ => todo!(),
    }
}

//---------------------------------------------------------------------------------------------------- Handler functions
// These are the actual functions that do stuff according to the incoming [`TxpoolWriteRequest`].
//
// Each function name is a 1-1 mapping (from CamelCase -> snake_case) to
// the enum variant name, e.g: `BlockExtendedHeader` -> `block_extended_header`.
//
// Each function will return the [`Response`] that we
// should send back to the caller in [`map_request()`].
