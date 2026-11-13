#![expect(
    unreachable_code,
    unused_variables,
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    reason = "TODO: finish implementing the signatures from <https://github.com/Cuprate/cuprate/pull/297>"
)]
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    num::NonZero,
    sync::Arc,
    task::{Context, Poll},
};

use fjall::Readable;
use futures::channel::oneshot;
use monero_oxide::transaction::{NotPruned, Transaction};
use rayon::ThreadPool;
use tower::Service;

use cuprate_helper::{
    asynch::InfallibleOneshotReceiver, cast::usize_to_u64, num::median,
    time::current_unix_timestamp,
};
use cuprate_types::{
    rpc::{SpentKeyImageInfo, TxInfo, TxpoolHisto, TxpoolStats},
    TxInPool,
};

use crate::{
    error::TxPoolError,
    ops::{get_transaction_verification_data, in_stem_pool},
    service::interface::{TxpoolReadRequest, TxpoolReadResponse},
    txpool::TxpoolDatabase,
    types::{TransactionBlobHash, TransactionHash, TransactionInfo, TxStateFlags},
    TxEntry,
};

/// The txpool [`Service`] read handle.
#[derive(Clone)]
pub struct TxpoolReadHandle {
    pub(crate) pool: Arc<ThreadPool>,

    pub(crate) txpool: Arc<TxpoolDatabase>,
}

impl Service<TxpoolReadRequest> for TxpoolReadHandle {
    type Response = TxpoolReadResponse;
    type Error = TxPoolError;
    type Future = InfallibleOneshotReceiver<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: TxpoolReadRequest) -> Self::Future {
        let (tx, rx) = oneshot::channel();

        let db = Arc::clone(&self.txpool);
        self.pool.spawn(move || {
            let res = map_request(&db, req);

            let _ = tx.send(res);
        });

        InfallibleOneshotReceiver::from(rx)
    }
}

//---------------------------------------------------------------------------------------------------- Request Mapping
// This function maps [`Request`]s to function calls
// executed by the rayon DB reader threadpool.

/// Map [`TxpoolReadRequest`]'s to specific database handler functions.
///
/// This is the main entrance into all `Request` handler functions.
/// The basic structure is:
/// 1. `Request` is mapped to a handler function
/// 2. Handler function is called
/// 3. [`TxpoolReadResponse`] is returned
fn map_request(
    db: &TxpoolDatabase,        // Access to the database
    request: TxpoolReadRequest, // The request we must fulfill
) -> Result<TxpoolReadResponse, TxPoolError> {
    match request {
        TxpoolReadRequest::TxBlob(tx_hash) => tx_blob(db, &tx_hash),
        TxpoolReadRequest::TxVerificationData(tx_hash) => tx_verification_data(db, &tx_hash),
        TxpoolReadRequest::FilterKnownTxBlobHashes(blob_hashes) => {
            filter_known_tx_blob_hashes(db, blob_hashes)
        }
        TxpoolReadRequest::TxsForBlock(txs_needed) => txs_for_block(db, txs_needed),
        TxpoolReadRequest::Backlog => backlog(db),
        TxpoolReadRequest::Size {
            include_sensitive_txs,
        } => size(db, include_sensitive_txs),
        TxpoolReadRequest::PoolInfo {
            include_sensitive_txs,
            max_tx_count,
            start_time,
        } => pool_info(db, include_sensitive_txs, max_tx_count, start_time),
        TxpoolReadRequest::TxsByHash {
            tx_hashes,
            include_sensitive_txs,
        } => txs_by_hash(db, tx_hashes, include_sensitive_txs),
        TxpoolReadRequest::KeyImagesSpent {
            key_images,
            include_sensitive_txs,
        } => key_images_spent(db, key_images, include_sensitive_txs),
        TxpoolReadRequest::KeyImagesSpentVec {
            key_images,
            include_sensitive_txs,
        } => key_images_spent_vec(db, key_images, include_sensitive_txs),
        TxpoolReadRequest::Pool {
            include_sensitive_txs,
        } => pool(db, include_sensitive_txs),
        TxpoolReadRequest::PoolStats {
            include_sensitive_txs,
        } => pool_stats(db, include_sensitive_txs),
        TxpoolReadRequest::AllHashes {
            include_sensitive_txs,
        } => all_hashes(db, include_sensitive_txs),
    }
}

//---------------------------------------------------------------------------------------------------- Handler functions
// These are the actual functions that do stuff according to the incoming [`TxpoolReadRequest`].
//
// Each function name is a 1-1 mapping (from CamelCase -> snake_case) to
// the enum variant name, e.g: `TxBlob` -> `tx_blob`.
//
// Each function will return the [`TxpoolReadResponse`] that we
// should send back to the caller in [`map_request()`].
//
// INVARIANT:
// These functions are called above in `tower::Service::call()`
// using a custom threadpool which means any call to `par_*()` functions
// will be using the custom rayon DB reader thread-pool, not the global one.
//
// All functions below assume that this is the case, such that
// `par_*()` functions will not block the _global_ rayon thread-pool.

/// [`TxpoolReadRequest::TxBlob`].
#[inline]
fn tx_blob(
    db: &TxpoolDatabase,
    tx_hash: &TransactionHash,
) -> Result<TxpoolReadResponse, TxPoolError> {
    let snapshot = db.fjall_database.snapshot();

    let tx_blob = snapshot
        .get(&db.tx_blobs, tx_hash)?
        .ok_or(TxPoolError::NotFound)?
        .to_vec();

    Ok(TxpoolReadResponse::TxBlob {
        tx_blob,
        state_stem: in_stem_pool(tx_hash, &snapshot, db)?,
    })
}

/// [`TxpoolReadRequest::TxVerificationData`].
#[inline]
fn tx_verification_data(
    db: &TxpoolDatabase,
    tx_hash: &TransactionHash,
) -> Result<TxpoolReadResponse, TxPoolError> {
    let snapshot = db.fjall_database.snapshot();

    get_transaction_verification_data(tx_hash, &snapshot, db)
        .map(TxpoolReadResponse::TxVerificationData)
}

/// [`TxpoolReadRequest::FilterKnownTxBlobHashes`].
fn filter_known_tx_blob_hashes(
    db: &TxpoolDatabase,
    mut blob_hashes: HashSet<TransactionBlobHash>,
) -> Result<TxpoolReadResponse, TxPoolError> {
    let snapshot = db.fjall_database.snapshot();

    let mut stem_pool_hashes = Vec::new();

    // A closure that returns `true` if a tx with a certain blob hash is unknown.
    // This also fills in `stem_tx_hashes`.
    let mut tx_unknown = |blob_hash| -> Result<bool, TxPoolError> {
        match snapshot.get(&db.known_blob_hashes, blob_hash)? {
            Some(tx_hash) => {
                let tx_hash = tx_hash.as_ref().try_into().unwrap();

                if in_stem_pool(&tx_hash, &snapshot, db)? {
                    stem_pool_hashes.push(tx_hash);
                }
                Ok(false)
            }
            None => Ok(true),
        }
    };

    let mut err = None;
    blob_hashes.retain(|blob_hash| match tx_unknown(*blob_hash) {
        Ok(res) => res,
        Err(e) => {
            err = Some(e);
            false
        }
    });

    if let Some(e) = err {
        return Err(e);
    }

    Ok(TxpoolReadResponse::FilterKnownTxBlobHashes {
        unknown_blob_hashes: blob_hashes,
        stem_pool_hashes,
    })
}

/// [`TxpoolReadRequest::TxsForBlock`].
fn txs_for_block(
    db: &TxpoolDatabase,
    txs: Vec<TransactionHash>,
) -> Result<TxpoolReadResponse, TxPoolError> {
    let snapshot = db.fjall_database.snapshot();

    let mut missing_tx_indexes = Vec::with_capacity(txs.len());
    let mut txs_verification_data = HashMap::with_capacity(txs.len());

    for (i, tx_hash) in txs.into_iter().enumerate() {
        match get_transaction_verification_data(&tx_hash, &snapshot, db) {
            Ok(tx) => {
                txs_verification_data.insert(tx_hash, tx);
            }
            Err(TxPoolError::NotFound) => missing_tx_indexes.push(i),
            Err(e) => return Err(e),
        }
    }

    Ok(TxpoolReadResponse::TxsForBlock {
        txs: txs_verification_data,
        missing: missing_tx_indexes,
    })
}

/// [`TxpoolReadRequest::Backlog`].
#[inline]
fn backlog(db: &TxpoolDatabase) -> Result<TxpoolReadResponse, TxPoolError> {
    let snapshot = db.fjall_database.snapshot();

    let backlog = snapshot
        .iter(&db.tx_infos)
        .map(|info| {
            let (id, tx_info) = info.into_inner()?;

            let tx_info: TransactionInfo = bytemuck::pod_read_unaligned(tx_info.as_ref());

            Ok(TxEntry {
                id: id.as_ref().try_into().unwrap(),
                weight: tx_info.weight,
                fee: tx_info.fee,
                private: tx_info.flags.private(),
                received_at: tx_info.received_at,
            })
        })
        .collect::<Result<_, TxPoolError>>()?;

    Ok(TxpoolReadResponse::Backlog(backlog))
}

/// [`TxpoolReadRequest::Size`].
#[inline]
fn size(
    db: &TxpoolDatabase,
    include_sensitive_txs: bool,
) -> Result<TxpoolReadResponse, TxPoolError> {
    let count = if include_sensitive_txs {
        db.tx_infos.len()?
    } else {
        let mut n = 0_usize;
        for guard in db.tx_infos.iter() {
            let info: TransactionInfo = bytemuck::pod_read_unaligned(guard.value()?.as_ref());
            if !info.flags.private() {
                n += 1;
            }
        }
        n
    };
    Ok(TxpoolReadResponse::Size(count))
}

/// [`TxpoolReadRequest::PoolInfo`].
fn pool_info(
    db: &TxpoolDatabase,
    include_sensitive_txs: bool,
    max_tx_count: usize,
    start_time: Option<NonZero<usize>>,
) -> Result<TxpoolReadResponse, TxPoolError> {
    Ok(TxpoolReadResponse::PoolInfo(todo!()))
}

/// [`TxpoolReadRequest::TxsByHash`].
fn txs_by_hash(
    db: &TxpoolDatabase,
    tx_hashes: Vec<[u8; 32]>,
    include_sensitive_txs: bool,
) -> Result<TxpoolReadResponse, TxPoolError> {
    let snapshot = db.fjall_database.snapshot();
    let mut txs = Vec::with_capacity(tx_hashes.len());

    for tx_hash in tx_hashes {
        let Some(info_bytes) = snapshot.get(&db.tx_infos, tx_hash)? else {
            continue;
        };
        let tx_info: TransactionInfo = bytemuck::pod_read_unaligned(info_bytes.as_ref());

        if !include_sensitive_txs && tx_info.flags.private() {
            continue;
        }

        let Some(blob) = snapshot.get(&db.tx_blobs, tx_hash)? else {
            continue;
        };

        txs.push(TxInPool {
            tx_hash,
            tx_blob: blob.to_vec(),
            double_spend_seen: tx_info.flags.contains(TxStateFlags::DOUBLE_SPENT),
            received_timestamp: tx_info.received_at,
            relayed: !tx_info.flags.private(),
        });
    }

    Ok(TxpoolReadResponse::TxsByHash(txs))
}

/// Returns whether a key image is spent by a transaction in the pool.
fn key_image_spent_in_pool(
    db: &TxpoolDatabase,
    snapshot: &fjall::Snapshot,
    key_image: &[u8; 32],
    include_sensitive_txs: bool,
) -> Result<bool, TxPoolError> {
    let Some(tx_hash) = snapshot.get(&db.spent_key_images, key_image)? else {
        return Ok(false);
    };

    if include_sensitive_txs {
        return Ok(true);
    }

    let tx_hash: TransactionHash = tx_hash.as_ref().try_into().unwrap();
    Ok(!in_stem_pool(&tx_hash, snapshot, db)?)
}

/// [`TxpoolReadRequest::KeyImagesSpent`].
fn key_images_spent(
    db: &TxpoolDatabase,
    key_images: HashSet<[u8; 32]>,
    include_sensitive_txs: bool,
) -> Result<TxpoolReadResponse, TxPoolError> {
    let snapshot = db.fjall_database.snapshot();

    #[expect(
        clippy::iter_over_hash_type,
        reason = "ordering does not matter, this returns whether any key image is spent"
    )]
    for key_image in &key_images {
        if key_image_spent_in_pool(db, &snapshot, key_image, include_sensitive_txs)? {
            return Ok(TxpoolReadResponse::KeyImagesSpent(true));
        }
    }

    Ok(TxpoolReadResponse::KeyImagesSpent(false))
}

/// [`TxpoolReadRequest::KeyImagesSpentVec`].
fn key_images_spent_vec(
    db: &TxpoolDatabase,
    key_images: Vec<[u8; 32]>,
    include_sensitive_txs: bool,
) -> Result<TxpoolReadResponse, TxPoolError> {
    let snapshot = db.fjall_database.snapshot();

    Ok(TxpoolReadResponse::KeyImagesSpentVec(
        key_images
            .iter()
            .map(|ki| key_image_spent_in_pool(db, &snapshot, ki, include_sensitive_txs))
            .collect::<Result<_, _>>()?,
    ))
}

/// [`TxpoolReadRequest::Pool`].
fn pool(
    db: &TxpoolDatabase,
    include_sensitive_txs: bool,
) -> Result<TxpoolReadResponse, TxPoolError> {
    let snapshot = db.fjall_database.snapshot();

    let mut txs = Vec::new();

    for guard in snapshot.iter(&db.tx_infos) {
        let (tx_hash, tx_info) = guard.into_inner()?;
        let tx_info: TransactionInfo = bytemuck::pod_read_unaligned(tx_info.as_ref());

        if !include_sensitive_txs && tx_info.flags.private() {
            continue;
        }

        let tx_blob = snapshot
            .get(&db.tx_blobs, &tx_hash)?
            .ok_or(TxPoolError::NotFound)?
            .to_vec();
        let tx = Transaction::<NotPruned>::read(&mut tx_blob.as_slice())
            .expect("Tx in the tx-pool must be parseable");

        txs.push(TxInfo {
            blob_size: usize_to_u64(tx_blob.len()),
            do_not_relay: false,
            double_spend_seen: tx_info.flags.contains(TxStateFlags::DOUBLE_SPENT),
            fee: tx_info.fee,
            id_hash: tx_hash.as_ref().try_into().unwrap(),
            // The tx-pool does not track these.
            kept_by_block: false,
            last_failed_height: 0,
            last_failed_id_hash: [0; 32],
            last_relayed_time: 0,
            max_used_block_height: 0,
            max_used_block_id_hash: [0; 32],
            receive_time: if include_sensitive_txs {
                tx_info.received_at
            } else {
                0
            },
            relayed: !tx_info.flags.private(),
            tx_blob,
            tx_json: tx.into(),
            weight: usize_to_u64(tx_info.weight),
        });
    }

    let mut spent_key_images = Vec::new();

    for guard in snapshot.iter(&db.spent_key_images) {
        let (key_image, tx_hash) = guard.into_inner()?;
        let tx_hash: TransactionHash = tx_hash.as_ref().try_into().unwrap();

        if !include_sensitive_txs && in_stem_pool(&tx_hash, &snapshot, db)? {
            continue;
        }

        spent_key_images.push(SpentKeyImageInfo {
            id_hash: key_image.as_ref().try_into().unwrap(),
            txs_hashes: vec![tx_hash],
        });
    }

    Ok(TxpoolReadResponse::Pool {
        txs,
        spent_key_images,
    })
}

/// [`TxpoolReadRequest::PoolStats`].
fn pool_stats(
    db: &TxpoolDatabase,
    include_sensitive_txs: bool,
) -> Result<TxpoolReadResponse, TxPoolError> {
    let snapshot = db.fjall_database.snapshot();

    let mut tx_infos = Vec::new();

    for guard in snapshot.iter(&db.tx_infos) {
        let tx_info: TransactionInfo = bytemuck::pod_read_unaligned(guard.value()?.as_ref());

        if include_sensitive_txs || !tx_info.flags.private() {
            tx_infos.push(tx_info);
        }
    }

    Ok(TxpoolReadResponse::PoolStats(txpool_stats(
        &tx_infos,
        current_unix_timestamp(),
    )))
}

/// Calculates [`TxpoolStats`] the same way as monerod.
///
/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_core/tx_pool.cpp#L1137-L1221>
fn txpool_stats(tx_infos: &[TransactionInfo], now: u64) -> TxpoolStats {
    let mut stats = TxpoolStats {
        txs_total: u32::try_from(tx_infos.len()).unwrap_or(u32::MAX),
        ..Default::default()
    };

    let mut weights = Vec::with_capacity(tx_infos.len());
    let mut age_histo = BTreeMap::<u64, TxpoolHisto>::new();

    for tx_info in tx_infos {
        let weight = u32::try_from(tx_info.weight).unwrap_or(u32::MAX);
        weights.push(weight);

        stats.bytes_total += u64::from(weight);
        if stats.bytes_min == 0 || weight < stats.bytes_min {
            stats.bytes_min = weight;
        }
        stats.bytes_max = stats.bytes_max.max(weight);
        stats.fee_total += tx_info.fee;

        if tx_info.flags.private() {
            stats.num_not_relayed += 1;
        }
        if tx_info.flags.contains(TxStateFlags::DOUBLE_SPENT) {
            stats.num_double_spends += 1;
        }

        if stats.oldest == 0 || tx_info.received_at < stats.oldest {
            stats.oldest = tx_info.received_at;
        }
        if tx_info.received_at < now.saturating_sub(600) {
            stats.num_10m += 1;
        }

        let age = now.saturating_sub(tx_info.received_at).max(1);
        let histo = age_histo.entry(age).or_default();
        histo.txs += 1;
        histo.bytes += u64::from(weight);
    }

    weights.sort_unstable();
    stats.bytes_med = if weights.is_empty() {
        0
    } else {
        median(&weights)
    };

    if tx_infos.len() < 2 {
        return stats;
    }

    // With 50+ txs the oldest 2% get a final bin of their own.
    let end = tx_infos.len() / 50;

    let (factor, delta) = if end == 0 {
        let factor = tx_infos.len().min(10);
        stats.histo = vec![TxpoolHisto::default(); factor];
        (factor, now.saturating_sub(stats.oldest))
    } else {
        let mut cumulative = 0;
        for (&age, histo) in age_histo.iter().rev() {
            stats.histo_98pc = age;
            cumulative += histo.txs;
            if u64::from(cumulative) >= usize_to_u64(end) {
                break;
            }
        }

        stats.histo = vec![TxpoolHisto::default(); 10];
        (9, stats.histo_98pc)
    };

    let factor_u64 = usize_to_u64(factor);
    let delta = delta.max(1);

    for (age, histo) in age_histo {
        let i = if end != 0 && age >= stats.histo_98pc {
            factor
        } else {
            // `age * factor - 1 < delta * factor`, so this fits in `usize`.
            usize::try_from((age * factor_u64 - 1) / delta).unwrap()
        };

        stats.histo[i].txs += histo.txs;
        stats.histo[i].bytes += histo.bytes;
    }

    stats
}

/// [`TxpoolReadRequest::AllHashes`].
fn all_hashes(
    db: &TxpoolDatabase,
    include_sensitive_txs: bool,
) -> Result<TxpoolReadResponse, TxPoolError> {
    let mut hashes = Vec::new();

    for guard in db.tx_infos.iter() {
        let (tx_hash, info) = guard.into_inner()?;

        if !include_sensitive_txs {
            let info: TransactionInfo = bytemuck::pod_read_unaligned(info.as_ref());
            if info.flags.private() {
                continue;
            }
        }

        hashes.push(tx_hash.as_ref().try_into().unwrap());
    }

    Ok(TxpoolReadResponse::AllHashes(hashes))
}
