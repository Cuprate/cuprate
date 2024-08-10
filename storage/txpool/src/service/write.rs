use std::sync::Arc;

use cuprate_database::{ConcreteEnv, Env, EnvInner, RuntimeError, TxRw};
use cuprate_database_service::DatabaseWriteHandle;
use cuprate_types::TransactionVerificationData;

use crate::{
    ops,
    service::{
        interface::{TxpoolWriteRequest, TxpoolWriteResponse},
        types::TxpoolWriteHandle,
    },
    tables::OpenTables,
    types::TransactionHash,
    TxPoolWriteError,
};

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
) -> Result<TxpoolWriteResponse, RuntimeError> {
    match req {
        TxpoolWriteRequest::AddTransaction { tx, state_stem } => {
            add_transaction(env, tx, *state_stem)
        }
        TxpoolWriteRequest::RemoveTransaction(tx_hash) => remove_transaction(env, tx_hash),
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

/// [`TxpoolWriteRequest::AddTransaction`]
fn add_transaction(
    env: &ConcreteEnv,
    tx: &TransactionVerificationData,
    state_stem: bool,
) -> Result<TxpoolWriteResponse, RuntimeError> {
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw()?;

    let mut tables_mut = env_inner.open_tables_mut(&tx_rw)?;

    if let Err(e) = ops::add_transaction(tx, state_stem, &mut tables_mut) {
        drop(tables_mut);
        // error adding the tx, abort the DB transaction.
        TxRw::abort(tx_rw)
            .expect("could not maintain database atomicity by aborting write transaction");

        return match e {
            TxPoolWriteError::DoubleSpend(tx_hash) => {
                // If we couldn't add the tx due to a double spend still return ok, but include the tx
                // this double spent.
                // TODO: mark the double spent tx?
                Ok(TxpoolWriteResponse::AddTransaction(Some(tx_hash)))
            }
            TxPoolWriteError::Database(e) => Err(e),
        };
    };

    drop(tables_mut);
    // The tx was added to the pool successfully.
    TxRw::commit(tx_rw)?;
    Ok(TxpoolWriteResponse::AddTransaction(None))
}

/// [`TxpoolWriteRequest::RemoveTransaction`]
fn remove_transaction(
    env: &ConcreteEnv,
    tx_hash: &TransactionHash,
) -> Result<TxpoolWriteResponse, RuntimeError> {
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw()?;

    let mut tables_mut = env_inner.open_tables_mut(&tx_rw)?;

    if let Err(e) = ops::remove_transaction(tx_hash, &mut tables_mut) {
        drop(tables_mut);
        // error removing the tx, abort the DB transaction.
        TxRw::abort(tx_rw)
            .expect("could not maintain database atomicity by aborting write transaction");
        
        return Err(e);
    }

    drop(tables_mut);

    TxRw::commit(tx_rw)?;
    Ok(TxpoolWriteResponse::Ok)
}
