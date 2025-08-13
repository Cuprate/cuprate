use std::sync::Arc;

use cuprate_database::{
    ConcreteEnv, DatabaseRo, DatabaseRw, DbResult, Env, EnvInner, InitError, RuntimeError, TxRw,
};
use cuprate_database_service::DatabaseWriteHandle;
use cuprate_types::TransactionVerificationData;

use crate::{
    ops::{self, TxPoolWriteError},
    service::{
        interface::{TxpoolWriteRequest, TxpoolWriteResponse},
        types::TxpoolWriteHandle,
    },
    tables::{OpenTables, Tables, TransactionInfos},
    types::{KeyImage, TransactionHash, TxStateFlags},
};

//---------------------------------------------------------------------------------------------------- init_write_service
/// Initialize the txpool write service from a [`ConcreteEnv`].
pub(super) fn init_write_service(env: Arc<ConcreteEnv>) -> Result<TxpoolWriteHandle, InitError> {
    DatabaseWriteHandle::init(env, handle_txpool_request)
}

//---------------------------------------------------------------------------------------------------- handle_txpool_request
/// Handle an incoming [`TxpoolWriteRequest`], returning a [`TxpoolWriteResponse`].
fn handle_txpool_request(
    env: &ConcreteEnv,
    req: &TxpoolWriteRequest,
) -> DbResult<TxpoolWriteResponse> {
    match req {
        TxpoolWriteRequest::AddTransaction { tx, state_stem } => {
            add_transaction(env, tx, *state_stem)
        }
        TxpoolWriteRequest::RemoveTransaction(tx_hash) => remove_transaction(env, tx_hash),
        TxpoolWriteRequest::Promote(tx_hash) => promote(env, tx_hash),
        TxpoolWriteRequest::NewBlock { spent_key_images } => new_block(env, spent_key_images),
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
) -> DbResult<TxpoolWriteResponse> {
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
    }

    drop(tables_mut);
    // The tx was added to the pool successfully.
    TxRw::commit(tx_rw)?;
    Ok(TxpoolWriteResponse::AddTransaction(None))
}

/// [`TxpoolWriteRequest::RemoveTransaction`]
fn remove_transaction(
    env: &ConcreteEnv,
    tx_hash: &TransactionHash,
) -> DbResult<TxpoolWriteResponse> {
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

/// [`TxpoolWriteRequest::Promote`]
fn promote(env: &ConcreteEnv, tx_hash: &TransactionHash) -> DbResult<TxpoolWriteResponse> {
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw()?;

    let res = || {
        let mut tx_infos = env_inner.open_db_rw::<TransactionInfos>(&tx_rw)?;

        tx_infos.update(tx_hash, |mut info| {
            info.flags.remove(TxStateFlags::STATE_STEM);
            Some(info)
        })
    };

    if let Err(e) = res() {
        // error promoting the tx, abort the DB transaction.
        TxRw::abort(tx_rw)
            .expect("could not maintain database atomicity by aborting write transaction");

        return Err(e);
    }

    TxRw::commit(tx_rw)?;
    Ok(TxpoolWriteResponse::Ok)
}

/// [`TxpoolWriteRequest::NewBlock`]
fn new_block(env: &ConcreteEnv, spent_key_images: &[KeyImage]) -> DbResult<TxpoolWriteResponse> {
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw()?;

    // FIXME: use try blocks once stable.
    let result = || {
        let mut tables_mut = env_inner.open_tables_mut(&tx_rw)?;

        // Remove all txs which spend key images that were spent in the new block.
        for key_image in spent_key_images {
            match tables_mut
                .spent_key_images()
                .get(key_image)
                .and_then(|tx_hash| ops::remove_transaction(&tx_hash, &mut tables_mut))
            {
                Ok(()) | Err(RuntimeError::KeyNotFound) => (),
                Err(e) => return Err(e),
            }
        }

        Ok(())
    };

    if let Err(e) = result() {
        TxRw::abort(tx_rw)?;
        return Err(e);
    }

    TxRw::commit(tx_rw)?;
    Ok(TxpoolWriteResponse::Ok)
}
