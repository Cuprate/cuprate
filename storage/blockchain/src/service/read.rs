//! Database reader thread-pool definitions and logic.

#![expect(
    unreachable_code,
    unused_variables,
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    reason = "TODO: finish implementing the signatures from <https://github.com/Cuprate/cuprate/pull/297>"
)]

//---------------------------------------------------------------------------------------------------- Import
use std::{
    cmp::min,
    collections::{BTreeMap, HashMap, HashSet},
    ops::Range,
    sync::Arc,
    task::{Context, Poll},
};

use bytes::Bytes;
use fjall::Readable;
use futures::channel::oneshot;
use indexmap::{IndexMap, IndexSet};
use rayon::{
    iter::{Either, IntoParallelIterator, ParallelIterator},
    prelude::*,
    ThreadPool,
};
use tapes::TapesRead;
use tower::Service;

use cuprate_helper::{
    asynch::InfallibleOneshotReceiver,
    cast::{u64_to_usize, usize_to_u64},
    map::{combine_low_high_bits_to_u128, split_u128_into_low_high_bits},
};
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    output_cache::OutputCache,
    rpc::{
        ChainInfo, CoinbaseTxSum, OutputDistributionData, OutputHistogramEntry,
        OutputHistogramInput,
    },
    Chain, ChainId, ExtendedBlockHeader, OutputDistributionInput, TransactionBlobs, TxsInBlock,
};

use crate::{
    error::{BlockchainError, DbResult},
    ops::{
        alt_block::{
            get_alt_block, get_alt_block_extended_header_from_height, get_alt_block_hash,
            get_alt_chain_history_ranges,
        },
        block::{
            block_exists, block_height, get_block, get_block_by_hash, get_block_complete_entry,
            get_block_complete_entry_from_height, get_block_extended_header_from_height,
        },
        blockchain::find_split_point,
        output::{
            get_num_outputs_with_amount, id_to_output_on_chain, unlocked_and_recent_instances,
        },
        tx::get_tx_blob_from_id,
    },
    service::{
        free::{compact_history_genesis_not_included, compact_history_index_to_height_offset},
        ResponseResult,
    },
    types::{
        AltBlockHeight, AltChainInfo, Amount, AmountIndex, BlockHash, BlockHeight,
        CompactAltBlockInfo, KeyImage, Output, PreRctOutputId, RawChainId,
    },
    BlockchainDatabase,
};

/// The [`tower::Service`] handle to read from the database.
#[derive(Clone)]
pub struct BlockchainReadHandle {
    /// Handle to the custom `rayon` DB reader thread-pool.
    ///
    /// Requests are [`rayon::ThreadPool::spawn`]ed in this thread-pool,
    /// and responses are returned via a channel we (the caller) provide.
    pub pool: Arc<ThreadPool>,

    pub blockchain: Arc<BlockchainDatabase>,
}

impl Service<BlockchainReadRequest> for BlockchainReadHandle {
    type Response = BlockchainResponse;
    type Error = BlockchainError;
    type Future = InfallibleOneshotReceiver<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: BlockchainReadRequest) -> Self::Future {
        let (tx, rx) = oneshot::channel();

        let db = Arc::clone(&self.blockchain);
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

/// Map [`Request`]'s to specific database handler functions.
///
/// This is the main entrance into all `Request` handler functions.
/// The basic structure is:
/// 1. `Request` is mapped to a handler function
/// 2. Handler function is called
/// 3. [`BlockchainResponse`] is returned
fn map_request(
    env: &BlockchainDatabase,       // Access to the database
    request: BlockchainReadRequest, // The request we must fulfill
) -> Result<BlockchainResponse, BlockchainError> {
    use BlockchainReadRequest as R;

    /* SOMEDAY: pre-request handling, run some code for each request? */

    match request {
        R::BlockCompleteEntries(block_hashes) => block_complete_entries(env, block_hashes),
        R::BlockCompleteEntriesByHeight(heights) => block_complete_entries_by_height(env, heights),
        R::BlockCompleteEntriesAboveSplitPoint {
            chain,
            start_height,
            no_miner_tx,
            len,
            pruned,
        } => block_complete_entries_above_split_point(
            env,
            chain,
            start_height,
            no_miner_tx,
            len,
            pruned,
        ),
        R::BlockExtendedHeader(block) => block_extended_header(env, block),
        R::BlockHash(block, chain) => block_hash(env, block, chain),
        R::BlockHashInRange(blocks, chain) => block_hash_in_range(env, blocks, chain),
        R::FindBlock(block_hash) => find_block(env, block_hash),
        R::FilterUnknownHashes(hashes) => filter_unknown_hashes(env, hashes),
        R::BlockExtendedHeaderInRange(range, chain) => {
            block_extended_header_in_range(env, range, chain)
        }
        R::ChainHeight => chain_height(env),
        R::GeneratedCoins(height) => generated_coins(env, height),
        R::Outputs {
            outputs: map,
            get_txid,
        } => outputs(env, map, get_txid),
        R::OutputsVec { outputs, get_txid } => outputs_vec(env, outputs, get_txid),
        R::NumberOutputsWithAmount(vec) => number_outputs_with_amount(env, vec),
        R::KeyImagesSpent(set) => key_images_spent(env, set),
        R::KeyImagesSpentVec(set) => key_images_spent_vec(env, set),
        R::CompactChainHistory => compact_chain_history(env),
        R::NextChainEntry(block_hashes, amount) => next_chain_entry(env, &block_hashes, amount),
        R::FindFirstUnknown(block_ids) => find_first_unknown(env, &block_ids),
        R::TxsInBlock {
            block_hash,
            tx_indexes,
        } => txs_in_block(env, block_hash, tx_indexes),
        R::AltBlocksInChain(chain_id) => alt_blocks_in_chain(env, chain_id),
        R::Block { height } => block(env, height),
        R::BlockByHash(hash) => block_by_hash(env, hash),
        R::TotalTxCount => total_tx_count(env),
        R::DatabaseSize => database_size(env),
        R::OutputHistogram(input) => output_histogram(env, input),
        R::CoinbaseTxSum { height, count } => coinbase_tx_sum(env, height, count),
        R::AltChains => alt_chains(env),
        R::AltChainCount => alt_chain_count(env),
        R::Transactions { tx_hashes } => transactions(env, tx_hashes),
        R::TotalRctOutputs => total_rct_outputs(env),
        R::TxOutputIndexes { tx_hash } => tx_output_indexes(env, &tx_hash),
        R::OutputDistribution(input) => output_distribution(env, input),
    }

    /* SOMEDAY: post-request handling, run some code for each request? */
}

//---------------------------------------------------------------------------------------------------- Handler functions
// These are the actual functions that do stuff according to the incoming [`Request`].
//
// Each function name is a 1-1 mapping (from CamelCase -> snake_case) to
// the enum variant name, e.g: `BlockExtendedHeader` -> `block_extended_header`.
//
// Each function will return the [`Response`] that we
// should send back to the caller in [`map_request()`].
//
// INVARIANT:
// These functions are called above in `tower::Service::call()`
// using a custom threadpool which means any call to `par_*()` functions
// will be using the custom rayon DB reader thread-pool, not the global one.
//
// All functions below assume that this is the case, such that
// `par_*()` functions will not block the _global_ rayon thread-pool.

// FIXME: implement multi-transaction read atomicity.
// <https://github.com/Cuprate/cuprate/pull/113#discussion_r1576874589>.

// TODO: The overhead of parallelism may be too much for every request, perfomace test to find optimal
// amount of parallelism.

/// [`BlockchainReadRequest::BlockCompleteEntries`].
fn block_complete_entries(db: &BlockchainDatabase, block_hashes: Vec<BlockHash>) -> ResponseResult {
    let tx_ro = db.fjall.snapshot();
    let tapes = db.linear_tapes.reader();

    let (missing_hashes, blocks) = block_hashes
        .into_par_iter()
        .map(
            |block_hash| match get_block_complete_entry(db, &block_hash, false, &tx_ro, &tapes) {
                Err(BlockchainError::NotFound) => Ok(Either::Left(block_hash)),
                res => res.map(Either::Right),
            },
        )
        .collect::<DbResult<_>>()?;

    let blockchain_height = crate::ops::blockchain::chain_height(db, &tapes)?;

    Ok(BlockchainResponse::BlockCompleteEntries {
        blocks,
        missing_hashes,
        blockchain_height,
    })
}

/// [`BlockchainReadRequest::BlockCompleteEntriesAboveSplitPoint`].
fn block_complete_entries_above_split_point(
    db: &BlockchainDatabase,
    chain: Vec<[u8; 32]>,
    start_height: Option<usize>,
    no_miner_tx: bool,
    len: usize,
    pruned: bool,
) -> ResponseResult {
    /// Total size of all block/tx blobs to return before stopping early.
    ///
    /// This is lower than monerod, as monerod packs too close to the epee size limit.
    const MAX_TOTAL_SIZE: usize = 50 * 1024 * 1024;
    /// Total tx count to return before stopping early.
    ///
    /// This is lower than monerod, as monerod packs too close to the epee size limit.
    const MAX_TOTAL_TXS: usize = 10_000;

    let tx_ro = db.fjall.snapshot();
    let tapes = db.linear_tapes.reader();

    let blockchain_height = crate::ops::blockchain::chain_height(db, &tapes)?;
    let top_hash = tapes
        .read_entry(&db.block_infos, usize_to_u64(blockchain_height - 1))?
        .ok_or(BlockchainError::NotFound)?
        .block_hash;

    // If a specific start height was requested, use it directly. Otherwise, scan to find the split point.
    let height = if let Some(h) = start_height {
        if h >= blockchain_height {
            return Ok(BlockchainResponse::BlockCompleteEntriesAboveSplitPoint {
                blocks: vec![],
                output_indices: vec![],
                blockchain_height,
                start_height: h,
                top_hash,
            });
        }
        h
    } else {
        let split = find_split_point(db, &chain, false, false, &tx_ro)?;

        if split == chain.len() {
            return Err(BlockchainError::NotFound);
        }

        block_height(db, &tx_ro, &chain[split])?.ok_or(BlockchainError::NotFound)?
    };

    if height == blockchain_height {
        return Ok(BlockchainResponse::BlockCompleteEntriesAboveSplitPoint {
            blocks: vec![],
            output_indices: vec![],
            blockchain_height,
            start_height: height,
            top_hash,
        });
    }

    let mut tx_count = 0;
    let mut total_size = 0;

    let blocks: Vec<_> = (height..min(height + len, blockchain_height))
        .map_while(|height| {
            if total_size >= MAX_TOTAL_SIZE || tx_count >= MAX_TOTAL_TXS {
                return None;
            }

            let block = match get_block_complete_entry_from_height(height, pruned, &tapes, db) {
                Ok(v) => v,
                Err(e) => return Some(Err(e)),
            };

            tx_count += block.txs.len() + 1;

            let tx_blobs_size = match &block.txs {
                TransactionBlobs::None => 0,
                TransactionBlobs::Normal(b) => b.iter().map(Bytes::len).sum(),
                TransactionBlobs::Pruned(p) => p.iter().map(|p| p.blob.len() + 32).sum(),
            };

            total_size += block.block.len() + tx_blobs_size;

            Some(Ok(block))
        })
        .collect::<DbResult<_>>()?;

    let first_tx_idx = tapes
        .read_entry(&db.block_infos, usize_to_u64(height))?
        .ok_or(BlockchainError::NotFound)?
        .mining_tx_index;

    let mut output_indices = Vec::with_capacity(blocks.len());
    output_indices.push(Vec::with_capacity(8));

    let mut last_height = height;
    let mut miner_tx = true;

    for (i, tx_info) in tapes.iter_from(&db.tx_infos, first_tx_idx)?.enumerate() {
        let tx_info = tx_info?;

        if tx_info.height != last_height {
            if tx_info.height == height + blocks.len() {
                // We have gone past all txs in the blocks we need
                break;
            }
            last_height = tx_info.height;
            miner_tx = true;
            output_indices.push(Vec::with_capacity(8));
        }

        // monerod replaces the miner tx's indices with an empty
        // placeholder when `no_miner_tx` is set.
        if no_miner_tx && miner_tx {
            miner_tx = false;
            output_indices.last_mut().unwrap().push(vec![]);
            continue;
        }
        miner_tx = false;

        let o_indexes = if tx_info.is_v1_tx() {
            // For v1 txs we need to look up indexes.
            let res = tx_ro
                .get(
                    &db.v1_tx_outputs,
                    (first_tx_idx + usize_to_u64(i)).to_le_bytes(),
                )?
                .ok_or(BlockchainError::NotFound)?;

            res.chunks(8)
                .map(|chunk| u64::from_le_bytes(chunk.try_into().unwrap()))
                .collect::<Vec<_>>()
        } else {
            // For v2 we can use the data in the tx_info.
            (0..tx_info.numb_rct_outputs)
                .map(|i| usize_to_u64(i) + tx_info.rct_output_start_idx)
                .collect()
        };

        output_indices.last_mut().unwrap().push(o_indexes);
    }

    Ok(BlockchainResponse::BlockCompleteEntriesAboveSplitPoint {
        blocks,
        output_indices,
        blockchain_height,
        start_height: height,
        top_hash,
    })
}

/// [`BlockchainReadRequest::BlockCompleteEntriesByHeight`].
fn block_complete_entries_by_height(
    db: &BlockchainDatabase,
    block_heights: Vec<BlockHeight>,
) -> ResponseResult {
    let tapes = db.linear_tapes.reader();

    let blocks = block_heights
        .into_par_iter()
        .map(|height| get_block_complete_entry_from_height(height, false, &tapes, db))
        .collect::<DbResult<_>>()?;

    Ok(BlockchainResponse::BlockCompleteEntriesByHeight(blocks))
}

/// [`BlockchainReadRequest::BlockExtendedHeader`].
#[inline]
fn block_extended_header(db: &BlockchainDatabase, block_height: BlockHeight) -> ResponseResult {
    let tapes = db.linear_tapes.reader();

    Ok(BlockchainResponse::BlockExtendedHeader(
        get_block_extended_header_from_height(block_height, &tapes, db)?,
    ))
}

/// [`BlockchainReadRequest::BlockHash`].
#[inline]
fn block_hash(db: &BlockchainDatabase, block_height: BlockHeight, chain: Chain) -> ResponseResult {
    let tx_ro = db.fjall.snapshot();

    let tapes = db.linear_tapes.reader();

    let block_hash = match chain {
        Chain::Main => {
            tapes
                .read_entry(&db.block_infos, usize_to_u64(block_height))?
                .ok_or(BlockchainError::NotFound)?
                .block_hash
        }
        Chain::Alt(chain) => get_alt_block_hash(db, &block_height, chain, &tx_ro, &tapes)?,
    };

    Ok(BlockchainResponse::BlockHash(block_hash))
}

/// [`BlockchainReadRequest::BlockHashInRange`].
#[inline]
fn block_hash_in_range(
    db: &BlockchainDatabase,
    range: Range<usize>,
    chain: Chain,
) -> ResponseResult {
    let tx_ro = db.fjall.snapshot();
    let tapes = db.linear_tapes.reader();

    if range.is_empty() {
        return Ok(BlockchainResponse::BlockHashInRange(vec![]));
    }

    let block_hash = match chain {
        Chain::Main => tapes
            .iter_from(&db.block_infos, usize_to_u64(range.start))?
            .map(|info| Ok(info?.block_hash))
            .take(range.len())
            .collect::<Result<_, BlockchainError>>()?,
        Chain::Alt(chain) => range
            .into_par_iter()
            .map(|block_height| get_alt_block_hash(db, &block_height, chain, &tx_ro, &tapes))
            .collect::<DbResult<Vec<_>>>()?,
    };

    Ok(BlockchainResponse::BlockHashInRange(block_hash))
}

/// [`BlockchainReadRequest::FindBlock`]
fn find_block(db: &BlockchainDatabase, block_hash: BlockHash) -> ResponseResult {
    let tx_ro = db.fjall.snapshot();

    // Check the main chain first.
    if let Some(height) = block_height(db, &tx_ro, &block_hash)? {
        return Ok(BlockchainResponse::FindBlock(Some((Chain::Main, height))));
    }

    match tx_ro.get(&db.alt_block_heights, block_hash)? {
        Some(height) => {
            let height: AltBlockHeight = bytemuck::pod_read_unaligned(height.as_ref());

            Ok(BlockchainResponse::FindBlock(Some((
                Chain::Alt(height.chain_id.into()),
                height.height,
            ))))
        }
        None => Ok(BlockchainResponse::FindBlock(None)),
    }
}

/// [`BlockchainReadRequest::FilterUnknownHashes`].
#[inline]
fn filter_unknown_hashes(
    db: &BlockchainDatabase,
    mut hashes: HashSet<BlockHash>,
) -> ResponseResult {
    let tx_ro = db.fjall.snapshot();

    let mut err = None;

    hashes.retain(|block_hash| match block_exists(db, block_hash, &tx_ro) {
        Ok(exists) => exists,
        Err(e) => {
            err.get_or_insert(e);
            false
        }
    });

    if let Some(e) = err {
        Err(e)
    } else {
        Ok(BlockchainResponse::FilterUnknownHashes(hashes))
    }
}

/// [`BlockchainReadRequest::BlockExtendedHeaderInRange`].
#[inline]
fn block_extended_header_in_range(
    db: &BlockchainDatabase,
    range: Range<BlockHeight>,
    chain: Chain,
) -> ResponseResult {
    let tx_ro = db.fjall.snapshot();
    let tapes = db.linear_tapes.reader();

    // Collect results using `rayon`.
    let vec = match chain {
        Chain::Main => range
            .into_iter()
            .map(|block_height| get_block_extended_header_from_height(block_height, &tapes, db))
            .collect::<DbResult<Vec<ExtendedBlockHeader>>>()?,
        Chain::Alt(chain_id) => {
            let ranges = { get_alt_chain_history_ranges(db, range, chain_id, &tx_ro)? };

            ranges
                .iter()
                .rev()
                .flat_map(|(chain, range)| {
                    range.clone().map(|height| match *chain {
                        Chain::Main => get_block_extended_header_from_height(height, &tapes, db),
                        Chain::Alt(chain_id) => get_alt_block_extended_header_from_height(
                            db,
                            &AltBlockHeight {
                                chain_id: chain_id.into(),
                                height,
                            },
                            &tx_ro,
                        ),
                    })
                })
                .collect::<DbResult<Vec<_>>>()?
        }
    };

    Ok(BlockchainResponse::BlockExtendedHeaderInRange(vec))
}

/// [`BlockchainReadRequest::ChainHeight`].
#[inline]
fn chain_height(db: &BlockchainDatabase) -> ResponseResult {
    let tapes = db.linear_tapes.reader();

    let chain_height = tapes
        .fixed_sized_tape_len(&db.block_infos)
        .expect("Required tape not found");

    if chain_height == 0 {
        return Err(BlockchainError::NotFound);
    }

    let block_hash = tapes
        .read_entry(&db.block_infos, chain_height - 1)?
        .ok_or(BlockchainError::NotFound)?
        .block_hash;

    Ok(BlockchainResponse::ChainHeight(
        chain_height.try_into().unwrap(),
        block_hash,
    ))
}

/// [`BlockchainReadRequest::GeneratedCoins`].
#[inline]
fn generated_coins(db: &BlockchainDatabase, height: usize) -> ResponseResult {
    let tapes = db.linear_tapes.reader();

    Ok(BlockchainResponse::GeneratedCoins(
        tapes
            .read_entry(&db.block_infos, usize_to_u64(height))?
            .map_or(0, |info| info.cumulative_generated_coins),
    ))
}

/// [`BlockchainReadRequest::Outputs`].
#[inline]
fn outputs(
    db: &BlockchainDatabase,
    outputs: IndexMap<Amount, IndexSet<AmountIndex>>,
    get_txid: bool,
) -> ResponseResult {
    // Prepare tx/tables in `ThreadLocal`.

    // TODO: we need to ensure the tables & tapes are in sync here.
    let tx_ro = db.fjall.snapshot();
    let tapes = db.linear_tapes.reader();

    let amount_of_outs = outputs
        .par_iter()
        .map(|(&amount, _)| {
            if amount == 0 {
                Ok((
                    amount,
                    tapes
                        .fixed_sized_tape_len(&db.rct_outputs)
                        .expect("Required tape not found"),
                ))
            } else {
                // v1 transactions.
                match get_num_outputs_with_amount(db, &tx_ro, amount) {
                    Ok(count) => Ok((amount, count)),
                    Err(e) => Err(e),
                }
            }
        })
        .collect::<Result<_, _>>()?;

    // The 2nd mapping function.
    // This is pulled out from the below `map()` for readability.
    let inner_map = |amount, amount_index| {
        let id = PreRctOutputId {
            amount,
            amount_index,
        };

        let output_on_chain = match id_to_output_on_chain(db, &id, get_txid, &tx_ro, &tapes) {
            Ok(output) => output,
            Err(BlockchainError::NotFound) => return Ok(Either::Right(amount_index)),
            Err(e) => return Err(e),
        };

        Ok(Either::Left((amount_index, output_on_chain)))
    };

    let (map, wanted_outputs) = outputs
        .into_par_iter()
        .map(|(amount, amount_index_set)| {
            let (left, right) = amount_index_set
                .into_par_iter()
                .map(|amount_index| inner_map(amount, amount_index))
                .collect::<Result<_, _>>()?;

            Ok(((amount, left), (amount, right)))
        })
        .collect::<DbResult<(IndexMap<_, IndexMap<_, _>>, IndexMap<_, IndexSet<_>>)>>()?;

    let cache = OutputCache::new(map, amount_of_outs, wanted_outputs);

    Ok(BlockchainResponse::Outputs(cache))
}

/// [`BlockchainReadRequest::OutputsVec`].
#[inline]
fn outputs_vec(
    db: &BlockchainDatabase,
    outputs: Vec<(Amount, AmountIndex)>,
    get_txid: bool,
) -> ResponseResult {
    let tx_ro = db.fjall.snapshot();
    let tapes = db.linear_tapes.reader();

    let result = outputs
        .into_iter()
        .map(|(amount, amount_index)| {
            let id = PreRctOutputId {
                amount,
                amount_index,
            };
            let output = id_to_output_on_chain(db, &id, get_txid, &tx_ro, &tapes)?;
            Ok((amount, vec![(amount_index, output)]))
        })
        .collect::<DbResult<Vec<_>>>()?;

    Ok(BlockchainResponse::OutputsVec(result))
}

/// [`BlockchainReadRequest::NumberOutputsWithAmount`].
#[inline]
fn number_outputs_with_amount(db: &BlockchainDatabase, amounts: Vec<Amount>) -> ResponseResult {
    let tx_ro = db.fjall.snapshot();
    let tapes = db.linear_tapes.reader();

    // Cache the amount of RCT outputs once.
    let num_rct_outputs = u64_to_usize(
        tapes
            .fixed_sized_tape_len(&db.rct_outputs)
            .expect("Required tape not found"),
    );

    // Collect results using `rayon`.
    let map = amounts
        .into_par_iter()
        .map(|amount| {
            if amount == 0 {
                // v2 transactions.
                Ok((amount, num_rct_outputs))
            } else {
                // v1 transactions.
                match get_num_outputs_with_amount(db, &tx_ro, amount) {
                    Ok(count) => Ok((amount, u64_to_usize(count))),
                    Err(e) => Err(e),
                }
            }
        })
        .collect::<DbResult<HashMap<Amount, usize>>>()?;

    Ok(BlockchainResponse::NumberOutputsWithAmount(map))
}

/// [`BlockchainReadRequest::KeyImagesSpent`].
#[inline]
fn key_images_spent(db: &BlockchainDatabase, key_images: HashSet<KeyImage>) -> ResponseResult {
    let tx_ro = db.fjall.snapshot();

    // FIXME:
    // Create/use `enum cuprate_types::Exist { Does, DoesNot }`
    // or similar instead of `bool` for clarity.
    // <https://github.com/Cuprate/cuprate/pull/113#discussion_r1581536526>
    //
    // Collect results using `rayon`.
    match key_images
        .into_par_iter()
        .map(|ki| tx_ro.contains_key(&db.key_images, ki))
        // If the result is either:
        // `Ok(true)` => a key image was found, return early
        // `Err` => an error was found, return early
        //
        // Else, `Ok(false)` will continue the iterator.
        .find_any(|result| !matches!(result, Ok(false)))
    {
        None => Ok(BlockchainResponse::KeyImagesSpent(false)), // Key image was NOT found.
        Some(Ok(true)) => Ok(BlockchainResponse::KeyImagesSpent(true)), // Key image was found.
        Some(Err(e)) => Err(e.into()),                         // A database error occurred.
        Some(Ok(false)) => unreachable!(),
    }
}

/// [`BlockchainReadRequest::KeyImagesSpentVec`].
fn key_images_spent_vec(db: &BlockchainDatabase, key_images: Vec<KeyImage>) -> ResponseResult {
    let tx_ro = db.fjall.snapshot();

    // Collect results using `rayon`.
    Ok(BlockchainResponse::KeyImagesSpentVec(
        key_images
            .into_par_iter()
            .map(|ki| tx_ro.contains_key(&db.key_images, ki))
            .collect::<Result<_, _>>()?,
    ))
}

/// [`BlockchainReadRequest::CompactChainHistory`]
fn compact_chain_history(db: &BlockchainDatabase) -> ResponseResult {
    let tapes = db.linear_tapes.reader();

    let get_block_info = |height| -> Result<_, BlockchainError> {
        tapes
            .read_entry(&db.block_infos, height)?
            .ok_or(BlockchainError::NotFound)
    };

    let top_block_height = tapes
        .fixed_sized_tape_len(&db.block_infos)
        .expect("Required tape not open")
        - 1;

    let top_block_info = get_block_info(top_block_height)?;
    let cumulative_difficulty = combine_low_high_bits_to_u128(
        top_block_info.cumulative_difficulty_low,
        top_block_info.cumulative_difficulty_high,
    );

    /// The amount of top block IDs in the compact chain.
    const INITIAL_BLOCKS: usize = 11;

    // rayon is not used here because the amount of block IDs is expected to be small.
    let mut block_ids = (0..)
        .map(compact_history_index_to_height_offset::<INITIAL_BLOCKS>)
        .map_while(|i| top_block_height.checked_sub(usize_to_u64(i)))
        .map(|height| Ok(get_block_info(height)?.block_hash))
        .collect::<DbResult<Vec<_>>>()?;

    if compact_history_genesis_not_included::<INITIAL_BLOCKS>(u64_to_usize(top_block_height)) {
        block_ids.push(get_block_info(0)?.block_hash);
    }

    Ok(BlockchainResponse::CompactChainHistory {
        cumulative_difficulty,
        block_ids,
    })
}

/// [`BlockchainReadRequest::NextChainEntry`]
///
/// # Invariant
/// `block_ids` must be sorted in reverse chronological block order, or else
/// the returned result is unspecified and meaningless, as this function
/// performs a binary search.
fn next_chain_entry(
    db: &BlockchainDatabase,
    block_ids: &[BlockHash],
    next_entry_size: usize,
) -> ResponseResult {
    let tx_ro = db.fjall.snapshot();

    let tapes = db.linear_tapes.reader();

    let idx = find_split_point(db, block_ids, false, false, &tx_ro)?;

    // This will happen if we have a different genesis block.
    if idx == block_ids.len() {
        return Ok(BlockchainResponse::NextChainEntry {
            start_height: None,
            chain_height: 0,
            block_ids: vec![],
            block_weights: vec![],
            cumulative_difficulty: 0,
            first_block_blob: None,
        });
    }

    // The returned chain entry must overlap with one of the blocks  we were told about.
    let first_known_block_hash = block_ids[idx];
    let first_known_height =
        block_height(db, &tx_ro, &first_known_block_hash)?.ok_or(BlockchainError::NotFound)?;

    let chain_height = crate::ops::blockchain::chain_height(db, &tapes)?;
    let last_height_in_chain_entry = min(first_known_height + next_entry_size, chain_height);

    let entry_count = last_height_in_chain_entry - first_known_height;
    let mut block_infos = vec![crate::types::BlockInfo::default(); entry_count];
    tapes.read_entries(
        &db.block_infos,
        usize_to_u64(first_known_height),
        &mut block_infos,
    )?;

    let (block_ids, block_weights) = block_infos
        .iter()
        .map(|info| (info.block_hash, info.weight))
        .unzip::<_, _, Vec<_>, Vec<_>>();

    let top_block_info = tapes
        .read_entry(&db.block_infos, usize_to_u64(chain_height) - 1)?
        .ok_or(BlockchainError::NotFound)?;

    let first_block_blob = if block_ids.len() >= 2 {
        Some(get_block(&(first_known_height + 1), None, &tapes, db)?.serialize())
    } else {
        None
    };

    Ok(BlockchainResponse::NextChainEntry {
        start_height: Some(first_known_height),
        chain_height,
        block_ids,
        block_weights,
        cumulative_difficulty: combine_low_high_bits_to_u128(
            top_block_info.cumulative_difficulty_low,
            top_block_info.cumulative_difficulty_high,
        ),
        first_block_blob,
    })
}

/// [`BlockchainReadRequest::FindFirstUnknown`]
///
/// # Invariant
/// `block_ids` must be sorted in chronological block order, or else
/// the returned result is unspecified and meaningless, as this function
/// performs a binary search.
fn find_first_unknown(db: &BlockchainDatabase, block_ids: &[BlockHash]) -> ResponseResult {
    let tx_ro = db.fjall.snapshot();
    let idx = find_split_point(db, block_ids, true, true, &tx_ro)?;

    Ok(if idx == block_ids.len() {
        BlockchainResponse::FindFirstUnknown(None)
    } else if idx == 0 {
        BlockchainResponse::FindFirstUnknown(Some((0, 0)))
    } else {
        let last_known_height = usize::from_le_bytes(
            tx_ro
                .get(&db.block_heights, block_ids[idx - 1])?
                .unwrap()
                .as_ref()
                .try_into()
                .unwrap(),
        );

        BlockchainResponse::FindFirstUnknown(Some((idx, last_known_height + 1)))
    })
}

/// [`BlockchainReadRequest::TxsInBlock`]
fn txs_in_block(
    db: &BlockchainDatabase,
    block_hash: [u8; 32],
    missing_txs: Vec<u64>,
) -> ResponseResult {
    let tx_ro = db.fjall.snapshot();
    let tapes = db.linear_tapes.reader();

    let block_height = usize::from_le_bytes(
        tx_ro
            .get(&db.block_heights, block_hash)?
            .ok_or(BlockchainError::NotFound)?
            .as_ref()
            .try_into()
            .unwrap(),
    );

    let block_info = tapes
        .read_entry(&db.block_infos, usize_to_u64(block_height))?
        .ok_or(BlockchainError::NotFound)?;

    let block = get_block(&block_height, None, &tapes, db)?;
    let first_tx_index = block_info.mining_tx_index + 1;

    if block.transactions.len() < missing_txs.len() {
        return Ok(BlockchainResponse::TxsInBlock(None));
    }

    let txs = missing_txs
        .into_iter()
        .map(|index_offset| get_tx_blob_from_id(&(first_tx_index + index_offset), &tapes, db))
        .collect::<DbResult<_>>()?;

    Ok(BlockchainResponse::TxsInBlock(Some(TxsInBlock {
        block: block.serialize(),
        txs,
    })))
}

/// [`BlockchainReadRequest::AltBlocksInChain`]
fn alt_blocks_in_chain(db: &BlockchainDatabase, chain_id: ChainId) -> ResponseResult {
    let tx_ro = db.fjall.snapshot();

    // Get the history of this alt-chain.
    let history = { get_alt_chain_history_ranges(db, 0..usize::MAX, chain_id, &tx_ro)? };

    // Get all the blocks until we join the main-chain.
    let blocks = history
        .iter()
        .rev()
        .skip(1)
        .flat_map(|(chain_id, range)| {
            let Chain::Alt(chain_id) = chain_id else {
                panic!("Should not have main chain blocks here we skipped last range");
            };

            range.clone().map(|height| {
                get_alt_block(
                    db,
                    &AltBlockHeight {
                        chain_id: (*chain_id).into(),
                        height,
                    },
                    &tx_ro,
                )
            })
        })
        .collect::<DbResult<_>>()?;

    Ok(BlockchainResponse::AltBlocksInChain(blocks))
}

/// [`BlockchainReadRequest::Block`]
fn block(db: &BlockchainDatabase, block_height: BlockHeight) -> ResponseResult {
    let tapes = db.linear_tapes.reader();

    Ok(BlockchainResponse::Block(get_block(
        &block_height,
        None,
        &tapes,
        db,
    )?))
}

/// [`BlockchainReadRequest::BlockByHash`]
fn block_by_hash(db: &BlockchainDatabase, block_hash: BlockHash) -> ResponseResult {
    let tx_ro = db.fjall.snapshot();
    let tapes = db.linear_tapes.reader();

    Ok(BlockchainResponse::Block(get_block_by_hash(
        db,
        &block_hash,
        &tx_ro,
        &tapes,
    )?))
}

/// [`BlockchainReadRequest::TotalTxCount`]
fn total_tx_count(db: &BlockchainDatabase) -> ResponseResult {
    let tapes = db.linear_tapes.reader();
    let count = u64_to_usize(
        tapes
            .fixed_sized_tape_len(&db.tx_infos)
            .expect("tx_infos tape exists"),
    );
    Ok(BlockchainResponse::TotalTxCount(count))
}

/// Walk a directory recursively and sum the sizes of all files.
fn dir_size(path: &std::path::Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|e| match e.file_type() {
            Ok(ft) if ft.is_dir() => dir_size(&e.path()),
            Ok(ft) if ft.is_file() => e.metadata().map_or(0, |m| m.len()),
            _ => 0,
        })
        .sum()
}

/// [`BlockchainReadRequest::DatabaseSize`]
fn database_size(db: &BlockchainDatabase) -> ResponseResult {
    // Sum file sizes in both data directories (blob and index).
    let blob_size = dir_size(&db.config.blob_dir);
    let index_size = if db.config.index_dir == db.config.blob_dir {
        0
    } else {
        dir_size(&db.config.index_dir)
    };
    let database_size = blob_size + index_size;

    // TODO:
    let free_space = u64::MAX;

    Ok(BlockchainResponse::DatabaseSize {
        database_size,
        free_space,
    })
}

/// [`BlockchainReadRequest::OutputHistogram`]
fn output_histogram(db: &BlockchainDatabase, input: OutputHistogramInput) -> ResponseResult {
    let tapes = db.linear_tapes.reader();
    let tx_ro = db.fjall.snapshot();

    let num_rct_outputs = tapes
        .fixed_sized_tape_len(&db.rct_outputs)
        .expect("rct_outputs tape exists");

    let amounts_and_counts: BTreeMap<Amount, u64> = if input.amounts.is_empty() {
        let mut result = BTreeMap::new();

        // RCT outputs are represented as amount = 0 and live in a separate tape.
        if num_rct_outputs > 0 {
            result.insert(0_u64, num_rct_outputs);
        }

        // We need to get the amount of outputs for each amount. We do this by first finding the next
        // `amount` value in the sorted table, then calling `get_num_outputs_with_amount`, then using
        // fjalls `range` to find the next amount value.
        let mut next = tx_ro.first_key_value(&db.pre_rct_outputs);
        while let Some(guard) = next {
            let amount = Amount::from_be_bytes(guard.key()?[..8].try_into().unwrap());

            result.insert(amount, get_num_outputs_with_amount(db, &tx_ro, amount)?);

            // Seek the first key of the next amount group, stopping on overflow/end.
            next = match amount.checked_add(1) {
                Some(next_amount) => tx_ro
                    .range(&db.pre_rct_outputs, next_amount.to_be_bytes()..)
                    .next(),
                None => None,
            };
        }

        result
    } else {
        // Use the caller-specified amounts.
        input
            .amounts
            .iter()
            .map(|&amount| {
                let count = if amount == 0 {
                    num_rct_outputs
                } else {
                    get_num_outputs_with_amount(db, &tx_ro, amount)?
                };
                Ok((amount, count))
            })
            .collect::<DbResult<_>>()?
    };

    let current_height = tapes
        .fixed_sized_tape_len(&db.block_infos)
        .expect("block_infos tape exists");

    let histogram = amounts_and_counts
        .into_iter()
        .filter(|&(_, total_instances)| {
            // filter the amounts that have to many or too little outputs
            (input.min_count == 0 || total_instances >= input.min_count)
                && (input.max_count == 0 || total_instances <= input.max_count)
        })
        .map(|(amount, total_instances)| {
            let (unlocked_instances, recent_instances) =
                if input.unlocked || input.recent_cutoff > 0 {
                    unlocked_and_recent_instances(
                        db,
                        &tx_ro,
                        &tapes,
                        amount,
                        total_instances,
                        current_height,
                        input.recent_cutoff,
                    )?
                } else {
                    (0, 0)
                };

            Ok(OutputHistogramEntry {
                amount,
                total_instances,
                unlocked_instances,
                recent_instances,
            })
        })
        .collect::<DbResult<_>>()?;

    Ok(BlockchainResponse::OutputHistogram(histogram))
}

/// [`BlockchainReadRequest::CoinbaseTxSum`]
fn coinbase_tx_sum(db: &BlockchainDatabase, height: usize, count: u64) -> ResponseResult {
    let tapes = db.linear_tapes.reader();

    let start_cumulative = if height == 0 {
        0_u64
    } else {
        tapes
            .read_entry(&db.block_infos, usize_to_u64(height - 1))?
            .ok_or(BlockchainError::NotFound)?
            .cumulative_generated_coins
    };

    let (emission_amount, fee_amount, _) = (height..)
        .zip(tapes.iter_from(&db.block_infos, usize_to_u64(height))?)
        .take(u64_to_usize(count))
        .try_fold(
            (0_u128, 0_u128, start_cumulative),
            |(emission_amount, fee_amount, prev_cumulative), (h, block_info)| {
                let block_info = block_info?;

                // base_reward = newly minted coins for this block (does not include fees).
                let base_reward = block_info
                    .cumulative_generated_coins
                    .saturating_sub(prev_cumulative);

                // coinbase_output = sum of miner_tx output amounts = base_reward + block_fees.
                let block = get_block(&h, Some(&block_info), &tapes, db)?;
                let coinbase_output: u64 = block
                    .miner_transaction()
                    .prefix()
                    .outputs
                    .iter()
                    .map(|o| o.amount.unwrap_or(0))
                    .sum();

                DbResult::Ok((
                    emission_amount + u128::from(base_reward),
                    fee_amount + u128::from(coinbase_output.saturating_sub(base_reward)),
                    block_info.cumulative_generated_coins,
                ))
            },
        )?;

    let (emission_amount, emission_amount_top64) = split_u128_into_low_high_bits(emission_amount);
    let (fee_amount, fee_amount_top64) = split_u128_into_low_high_bits(fee_amount);

    Ok(BlockchainResponse::CoinbaseTxSum(CoinbaseTxSum {
        emission_amount,
        emission_amount_top64,
        fee_amount,
        fee_amount_top64,
    }))
}

/// [`BlockchainReadRequest::AltChains`]
fn alt_chains(db: &BlockchainDatabase) -> ResponseResult {
    let tx_ro = db.fjall.snapshot();
    let tapes = db.linear_tapes.reader();

    let mut chains = Vec::new();

    for guard in db.alt_chain_infos.iter() {
        let (chain_id_bytes, chain_info_bytes) = guard.into_inner()?;

        let chain_id = RawChainId(u64::from_le_bytes(
            chain_id_bytes.as_ref().try_into().unwrap(),
        ));
        let chain_info: AltChainInfo = bytemuck::pod_read_unaligned(chain_info_bytes.as_ref());

        let tip = chain_info.chain_height - 1;
        let history =
            get_alt_chain_history_ranges(db, 0..chain_info.chain_height, chain_id.into(), &tx_ro)?;

        let tip_height = AltBlockHeight {
            chain_id,
            height: tip,
        };
        let tip_info_bytes = tx_ro
            .get(&db.alt_block_infos, bytemuck::bytes_of(&tip_height))?
            .ok_or(BlockchainError::NotFound)?;
        let tip_info: CompactAltBlockInfo = bytemuck::pod_read_unaligned(tip_info_bytes.as_ref());

        // The last segment is the main-chain portion. Its range starts at 0 and ends at the
        // fork height, so the last main chain block = range.end - 1.
        let main_fork_height = history
            .last()
            .map(|(_, r)| r.end.saturating_sub(1))
            .ok_or(BlockchainError::NotFound)?;

        // Build the hashes of the alt chain all the way to the main chain split point.
        let mut block_hashes = Vec::new();
        for (segment_chain, height_range) in &history {
            let Chain::Alt(segment_chain_id) = segment_chain else {
                break;
            };
            let raw_id = RawChainId::from(*segment_chain_id);
            for height in height_range.clone().rev() {
                let alt_h = AltBlockHeight {
                    chain_id: raw_id,
                    height,
                };
                let info_bytes = tx_ro
                    .get(&db.alt_block_infos, bytemuck::bytes_of(&alt_h))?
                    .ok_or(BlockchainError::NotFound)?;
                let info: CompactAltBlockInfo = bytemuck::pod_read_unaligned(info_bytes.as_ref());
                block_hashes.push(info.block_hash);
            }
        }

        // Get the main chain block hash at the fork point.
        let main_chain_parent_block = tapes
            .read_entry(&db.block_infos, usize_to_u64(main_fork_height))?
            .map(|info| info.block_hash)
            .ok_or(BlockchainError::NotFound)?;

        let length = usize_to_u64(block_hashes.len());

        chains.push(ChainInfo {
            block_hash: tip_info.block_hash,
            block_hashes,
            difficulty: tip_info.cumulative_difficulty_low,
            difficulty_top64: tip_info.cumulative_difficulty_high,
            height: usize_to_u64(tip),
            length,
            main_chain_parent_block,
        });
    }

    Ok(BlockchainResponse::AltChains(chains))
}

/// [`BlockchainReadRequest::AltChainCount`]
fn alt_chain_count(db: &BlockchainDatabase) -> ResponseResult {
    let count = db.alt_chain_infos.len()?;
    Ok(BlockchainResponse::AltChainCount(count))
}

/// [`BlockchainReadRequest::Transactions`]
fn transactions(db: &BlockchainDatabase, tx_hashes: HashSet<[u8; 32]>) -> ResponseResult {
    Ok(BlockchainResponse::Transactions {
        txs: todo!(),
        missed_txs: todo!(),
    })
}

/// [`BlockchainReadRequest::TotalRctOutputs`]
fn total_rct_outputs(db: &BlockchainDatabase) -> ResponseResult {
    let tapes = db.linear_tapes.reader();

    let len = tapes
        .fixed_sized_tape_len(&db.rct_outputs)
        .expect("Required tape not found");

    Ok(BlockchainResponse::TotalRctOutputs(len))
}

/// [`BlockchainReadRequest::TxOutputIndexes`]
fn tx_output_indexes(db: &BlockchainDatabase, tx_hash: &[u8; 32]) -> ResponseResult {
    let tx_ro = db.fjall.snapshot();
    let tapes = db.linear_tapes.reader();

    let tx_id = tx_ro
        .get(&db.tx_ids, tx_hash)?
        .ok_or(BlockchainError::NotFound)?;

    let tx_id = u64::from_le_bytes(tx_id.as_ref().try_into().unwrap());

    let tx_info = tapes
        .read_entry(&db.tx_infos, tx_id)?
        .ok_or(BlockchainError::NotFound)?;

    let o_indexes = if tx_info.is_v1_tx() {
        let bytes = tx_ro
            .get(&db.v1_tx_outputs, tx_id.to_le_bytes())?
            .ok_or(BlockchainError::NotFound)?;

        bytemuck::pod_collect_to_vec(bytes.as_ref())
    } else {
        (0..tx_info.numb_rct_outputs)
            .map(|i| usize_to_u64(i) + tx_info.rct_output_start_idx)
            .collect()
    };

    Ok(BlockchainResponse::TxOutputIndexes(o_indexes))
}

/// [`BlockchainReadRequest::OutputDistribution`]
fn output_distribution(db: &BlockchainDatabase, input: OutputDistributionInput) -> ResponseResult {
    let tapes = db.linear_tapes.reader();
    let chain_height = u64_to_usize(
        tapes
            .fixed_sized_tape_len(&db.block_infos)
            .expect("block_infos tape exists"),
    );

    if input.to_height.is_some_and(|h| h.get() < input.from_height) {
        return Err(BlockchainError::NotFound);
    }

    let to_height = input.to_height.map_or(chain_height - 1, |h| {
        let h = h.get();
        u64_to_usize(h)
    });

    if to_height >= chain_height {
        return Err(BlockchainError::NotFound);
    }

    let mut result = Vec::with_capacity(input.amounts.len());

    for &amount in &input.amounts {
        if amount == 0 {
            // clamp the start to the start of RCT, like monerod.
            let start_height = u64_to_usize(input.from_height.max(input.rct_start_height));

            if start_height > to_height {
                result.push(OutputDistributionData {
                    amount: 0,
                    distribution: vec![],
                    start_height: usize_to_u64(start_height),
                    base: 0,
                });
                continue;
            }

            // Read one block below the range to get the `base` value to calculate real values from
            // cumulative ones.
            let real_start = start_height.saturating_sub(1);
            let count = to_height - real_start + 1;

            let mut iter = tapes
                .iter_from(&db.block_infos, usize_to_u64(real_start))?
                .take(count)
                .map(|info| -> DbResult<u64> { Ok(info?.cumulative_rct_outs) });

            let base = if start_height > 0 {
                // Take the first value (the base for the cumulative calculation)
                iter.next().ok_or(BlockchainError::NotFound)??
            } else {
                0
            };

            let distribution: Vec<u64> = if input.cumulative {
                iter.collect::<DbResult<_>>()?
            } else {
                let mut prev = base;
                iter.map(|cumulative| {
                    let cumulative = cumulative?;
                    let delta = cumulative - prev;
                    prev = cumulative;
                    Ok(delta)
                })
                .collect::<DbResult<_>>()?
            };

            result.push(OutputDistributionData {
                amount: 0,
                distribution,
                start_height: usize_to_u64(start_height),
                base,
            });
        } else {
            let start_height = u64_to_usize(input.from_height);

            if start_height > to_height {
                return Err(BlockchainError::NotFound);
            }

            let mut per_block: Vec<u64> = vec![0; to_height - start_height + 1];
            let mut below_start: u64 = 0;

            // loop over all outputs with this amount.
            for guard in db.pre_rct_outputs.prefix(amount.to_be_bytes()) {
                let output: Output = bytemuck::pod_read_unaligned(guard.value()?.as_ref());
                let h = output.height;
                // Add the output to the block it says it is in.
                if h < start_height {
                    below_start += 1;
                } else if h <= to_height {
                    per_block[h - start_height] += 1;
                }
            }

            // monerod folds the below `start_height` count into the first bucket and
            // reports `base = 0` for pre-RCT amounts.
            per_block[0] += below_start;

            let distribution = if input.cumulative {
                let mut cumulative = per_block;
                for i in 1..cumulative.len() {
                    cumulative[i] += cumulative[i - 1];
                }
                cumulative
            } else {
                per_block
            };

            result.push(OutputDistributionData {
                amount,
                distribution,
                start_height: usize_to_u64(start_height),
                base: 0,
            });
        }
    }

    Ok(BlockchainResponse::OutputDistribution(result))
}
