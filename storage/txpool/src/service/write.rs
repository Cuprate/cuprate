use crate::error::TxPoolError;
use crate::txpool::TxpoolDatabase;
use crate::types::TransactionInfo;
use crate::{
    ops::{self, TxPoolWriteError},
    service::interface::{TxpoolWriteRequest, TxpoolWriteResponse},
    types::{KeyImage, TransactionHash, TxStateFlags},
};
use cuprate_helper::asynch::InfallibleOneshotReceiver;
use cuprate_types::TransactionVerificationData;
use fjall::PersistMode;
use futures::channel::oneshot;
use monero_oxide::transaction::Input;
use rayon::ThreadPool;
use std::collections::hash_map::Entry;
use std::collections::HashSet;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::Service;

#[derive(Clone)]
pub struct TxpoolWriteHandle {
    /// Handle to the custom `rayon` DB reader thread-pool.
    ///
    /// Requests are [`rayon::ThreadPool::spawn`]ed in this thread-pool,
    /// and responses are returned via a channel we (the caller) provide.
    pub pool: Arc<ThreadPool>,

    pub txpool: Arc<TxpoolDatabase>,
}

impl Service<TxpoolWriteRequest> for TxpoolWriteHandle {
    type Response = TxpoolWriteResponse;
    type Error = TxPoolError;
    type Future = InfallibleOneshotReceiver<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: TxpoolWriteRequest) -> Self::Future {
        let (tx, rx) = oneshot::channel();

        let db = Arc::clone(&self.txpool);
        self.pool.spawn(move || {
            let res = handle_txpool_request(&db, req);

            drop(tx.send(res));
        });

        InfallibleOneshotReceiver::from(rx)
    }
}

//---------------------------------------------------------------------------------------------------- handle_txpool_request
/// Handle an incoming [`TxpoolWriteRequest`], returning a [`TxpoolWriteResponse`].
fn handle_txpool_request(
    env: &TxpoolDatabase,
    req: TxpoolWriteRequest,
) -> Result<TxpoolWriteResponse, TxPoolError> {
    match req {
        TxpoolWriteRequest::AddTransaction { tx, state_stem } => {
            add_transaction(env, &tx, state_stem)
        }
        TxpoolWriteRequest::RemoveTransaction(tx_hash) => remove_transaction(env, &tx_hash),
        TxpoolWriteRequest::Promote(tx_hash) => promote(env, &tx_hash),
        TxpoolWriteRequest::NewBlock { spent_key_images } => new_block(env, &spent_key_images),
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
    db: &TxpoolDatabase,
    tx: &TransactionVerificationData,
    state_stem: bool,
) -> Result<TxpoolWriteResponse, TxPoolError> {
    struct KiDropGuard<'a>(Vec<[u8; 32]>, &'a TxpoolDatabase);

    impl Drop for KiDropGuard<'_> {
        fn drop(&mut self) {
            for ki in &self.0 {
                self.1.in_progress_key_images.lock().unwrap().remove(ki);
            }
        }
    }

    let mut guard = KiDropGuard(Vec::with_capacity(tx.tx.prefix().inputs.len()), db);

    let mut in_progress_key_images = db.in_progress_key_images.lock().unwrap();
    for ki in tx.tx.prefix().inputs.iter().map(|i| match i {
        Input::ToKey { key_image, .. } => key_image,
        Input::Gen(_) => unreachable!(),
    }) {
        let e = in_progress_key_images.entry(*ki.as_bytes());

        match e {
            Entry::Occupied(o) => return Ok(TxpoolWriteResponse::AddTransaction(Some(*o.get()))),
            Entry::Vacant(v) => {
                v.insert(*ki.as_bytes());
            }
        }

        guard.0.push(*ki.as_bytes());
    }
    drop(in_progress_key_images);

    let mut writer = db.fjall_database.batch();

    if let Err(e) = ops::add_transaction(tx, state_stem, &mut writer, db) {
        // error adding the tx, abort the DB transaction.
        drop(writer);

        return match e {
            TxPoolWriteError::DoubleSpend(tx_hash) => {
                // If we couldn't add the tx due to a double spend still return ok, but include the tx
                // this double spent.
                // TODO: mark the double spent tx?
                Ok(TxpoolWriteResponse::AddTransaction(Some(tx_hash)))
            }
            TxPoolWriteError::TxPool(e) => Err(e),
        };
    }

    // The tx was added to the pool successfully.
    writer.commit()?;

    Ok(TxpoolWriteResponse::AddTransaction(None))
}

/// [`TxpoolWriteRequest::RemoveTransaction`]
fn remove_transaction(
    db: &TxpoolDatabase,
    tx_hash: &TransactionHash,
) -> Result<TxpoolWriteResponse, TxPoolError> {
    let mut writer = db.fjall_database.batch();

    ops::remove_transaction(tx_hash, &mut writer, db)?;

    writer.commit()?;

    Ok(TxpoolWriteResponse::Ok)
}

/// [`TxpoolWriteRequest::Promote`]
fn promote(
    db: &TxpoolDatabase,
    tx_hash: &TransactionHash,
) -> Result<TxpoolWriteResponse, TxPoolError> {
    let tx_info = db.tx_infos.get(tx_hash)?.ok_or(TxPoolError::NotFound)?;
    let mut tx_info: TransactionInfo = bytemuck::pod_read_unaligned(tx_info.as_ref());

    if !tx_info.flags.contains(TxStateFlags::STATE_STEM) {
        return Ok(TxpoolWriteResponse::Ok);
    }

    tx_info.flags.remove(TxStateFlags::STATE_STEM);

    db.tx_infos.insert(tx_hash, bytemuck::bytes_of(&tx_info))?;

    if !db.tx_blobs.contains_key(tx_hash)? {
        db.tx_infos.remove(tx_hash)?;
    }

    Ok(TxpoolWriteResponse::Ok)
}

/// [`TxpoolWriteRequest::NewBlock`]
fn new_block(
    db: &TxpoolDatabase,
    spent_key_images: &[KeyImage],
) -> Result<TxpoolWriteResponse, TxPoolError> {
    let mut txs_removed = HashSet::new();

    let mut writer = db.fjall_database.batch();

    // Remove all txs which spend key images that were spent in the new block.
    for key_image in spent_key_images {
        if let Some(tx_hash) = db.spent_key_images.get(key_image)? {
            let tx_hash = tx_hash.as_ref().try_into().unwrap();

            if txs_removed.insert(tx_hash) {
                ops::remove_transaction(&tx_hash, &mut writer, db)?;
            }
        }
    }

    writer.commit()?;
    Ok(TxpoolWriteResponse::NewBlock(
        txs_removed.into_iter().collect(),
    ))
}
