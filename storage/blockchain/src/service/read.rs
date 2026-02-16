//! Database reader thread-pool definitions and logic.

#![expect(
    unreachable_code,
    unused_variables,
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    reason = "TODO: finish implementing the signatures from <https://github.com/Cuprate/cuprate/pull/297>"
)]

//---------------------------------------------------------------------------------------------------- Import
use bytes::Bytes;
use cuprate_helper::asynch::InfallibleOneshotReceiver;
use cuprate_helper::map::combine_low_high_bits_to_u128;
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    output_cache::OutputCache,
    rpc::OutputHistogramInput,
    Chain, ChainId, ExtendedBlockHeader, OutputDistributionInput, TransactionBlobs, TxsInBlock,
};
use fjall::Readable;
use futures::channel::oneshot;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use rayon::{
    iter::{Either, IntoParallelIterator, ParallelIterator},
    prelude::*,
    ThreadPool,
};
use std::cmp::max;
use std::task::{Context, Poll};
use std::{
    cmp::min,
    collections::{HashMap, HashSet},
    ops::Range,
    sync::Arc,
};
use tapes::{TapesAppend, TapesRead, TapesTruncate};
use thread_local::ThreadLocal;
use tower::Service;

use crate::error::{BlockchainError, DbResult};
use crate::ops::output::get_num_outputs_with_amount;
use crate::ops::tx::get_tx_blob_from_id;
use crate::types::{BlockInfo, RctOutput, TxInfo};
use crate::{
    ops::{
        /*
        alt_block::{
            get_alt_block, get_alt_block_extended_header_from_height, get_alt_block_hash,
            get_alt_chain_history_ranges,
        },

         */
        block::{
            block_exists, block_height, get_block, get_block_by_hash, get_block_complete_entry,
            get_block_complete_entry_from_height, get_block_extended_header_from_height,
            get_block_height,
        },
        blockchain::find_split_point,
        output::id_to_output_on_chain,
    },
    service::{
        free::{compact_history_genesis_not_included, compact_history_index_to_height_offset},
        ResponseResult,
    },
    types::{
        AltBlockHeight, Amount, AmountIndex, BlockHash, BlockHeight, KeyImage, PreRctOutputId,
    },
    BlockchainDatabase,
};

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
        let (mut tx, rx) = oneshot::channel();

        let db = self.blockchain.clone();
        self.pool.spawn(move || {
            let res = map_request(&db, req);

            tx.send(res);
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
    let tx_ro = db.fjall_keyspace.snapshot();

    let tapes = db.linear_tapes.reader();

    let mut missing_hashes = Vec::with_capacity(block_hashes.len());
    let mut blocks = Vec::with_capacity(block_hashes.len());
    for res in block_hashes.into_iter().map(|block_hash| {
        match get_block_complete_entry(db, &block_hash, false, &tx_ro, &tapes) {
            Err(BlockchainError::NotFound) => Ok(Either::Left(block_hash)),
            res => res.map(Either::Right),
        }
    }) {
        match res? {
            Either::Left(l) => missing_hashes.push(l),
            Either::Right(r) => blocks.push(r),
        }
    }

    let blockchain_height = crate::ops::blockchain::chain_height(db, &tapes)?;

    Ok(BlockchainResponse::BlockCompleteEntries {
        blocks,
        // output_indices: vec![],
        missing_hashes,
        blockchain_height,
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

/*
/// [`BlockchainReadRequest::BlockCompleteEntriesAboveSplitPoint`].
fn block_complete_entries_above_split_point(
    db: &BlockchainDatabase,
    chain: Vec<[u8; 32]>,
    get_indices: bool,
    len: usize,
    pruned: bool,
) -> ResponseResult {
    const MAX_TOTAL_SIZE: usize = 50 * 1024 * 1024;
    const MAX_TOTAL_TXS: usize = 10_000;

    let tx_ro = db.fjall_keyspace.snapshot();

    let tapes = db.linear_tapes.reader();

    let split = find_split_point(db, &chain, false, false, &tx_ro)?;

    if split == chain.len() {
        todo!()
    }

    let height = block_height(db, &tx_ro, &chain[split])?.ok_or(BlockchainError::NotFound)?;
    let blockchain_height = crate::ops::blockchain::chain_height(db, &tapes)?;

    if height == blockchain_height {
        todo!()
    }

    let mut tx_count = 0;
    let mut total_size = 0;

    let blocks: Vec<_> = (height..min(height + len, blockchain_height))
        .map_while(|height| {
            let block = match get_block_complete_entry_from_height(height, pruned, &tapes, db) {
                Ok(v) => v,
                Err(e) => return Some(Err(e)),
            };

            let first = tx_count == 0;
            tx_count += block.txs.len() + 1;

            let tx_blobs_size = match &block.txs {
                TransactionBlobs::None => 0,
                TransactionBlobs::Normal(b) => b.iter().map(Bytes::len).sum(),
                TransactionBlobs::Pruned(p) => p.iter().map(|p| p.blob.len() + 32).sum(),
            };

            total_size += block.block.len() + tx_blobs_size;

            if first || (total_size < MAX_TOTAL_SIZE && tx_count < MAX_TOTAL_TXS) {
                Some(Ok(block))
            } else {
                None
            }
        })
        .collect::<DbResult<_>>()?;

    let output_indices = if get_indices {
        let first_tx_idx = tapes
            .read_entry(&db.block_infos, height as u64)?
            .ok_or(BlockchainError::NotFound)?
            .mining_tx_index;

        let mut output_indices = Vec::with_capacity(blocks.len());
        output_indices.push(Vec::with_capacity(8));

        let mut last_height = height;

        for (i, tx_info) in tapes.iter_from(&db.tx_infos, first_tx_idx)?.enumerate() {
            let tx_info = tx_info?;

            if tx_info.height != last_height {
                if tx_info.height == height + blocks.len() {
                    break;
                }
                last_height = tx_info.height;
                output_indices.push(Vec::with_capacity(8));
            }

            let o_indexes = if tx_info.rct_output_start_idx == u64::MAX {
                let res = tx_ro
                    .get(&db.v1_tx_outputs, &(first_tx_idx + i as u64).to_le_bytes())
                    .expect("TDOD")
                    .ok_or(BlockchainError::NotFound)?;

                res.chunks(8)
                    .map(|chunk| u64::from_le_bytes(chunk.try_into().unwrap()))
                    .collect::<Vec<_>>()
            } else {
                (0..tx_info.numb_rct_outputs)
                    .map(|i| i as u64 + tx_info.rct_output_start_idx)
                    .collect()
            };

            output_indices.last_mut().unwrap().push(o_indexes);
        }

        output_indices
    } else {
        vec![]
    };

    Ok(BlockchainResponse::BlockCompleteEntriesAboveSplitPoint {
        blocks,
        output_indices,
        blockchain_height,
        start_height: height,
    })
}

 */

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
    let tx_ro = db.fjall_keyspace.snapshot();

    let tapes = db.linear_tapes.reader();

    let block_hash = match chain {
        Chain::Main => {
            tapes
                .read_entry(&db.block_infos, block_height as u64)?
                .ok_or(BlockchainError::NotFound)?
                .block_hash
        }
        Chain::Alt(chain) => todo!(), // get_alt_block_hash(db, &block_height, chain, &tx_ro, &tapes)?,
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
    let tx_ro = db.fjall_keyspace.snapshot();

    let tapes = db.linear_tapes.reader();

    let block_hash = range
        .into_iter()
        .map(|block_height| {
            let block_hash = match chain {
                Chain::Main => {
                    tapes
                        .read_entry(&db.block_infos, block_height as u64)?
                        .ok_or(BlockchainError::NotFound)?
                        .block_hash
                }
                Chain::Alt(chain) => todo!(), // get_alt_block_hash(db, &block_height, chain, &tx_ro, &tapes)?,
            };

            Ok(block_hash)
        })
        .collect::<Result<_, BlockchainError>>()?;

    Ok(BlockchainResponse::BlockHashInRange(block_hash))
}

/// [`BlockchainReadRequest::FindBlock`]
fn find_block(db: &BlockchainDatabase, block_hash: BlockHash) -> ResponseResult {
    let tx_ro = db.fjall_keyspace.snapshot();

    // Check the main chain first.
    match block_height(db, &tx_ro, &block_hash)? {
        Some(height) => return Ok(BlockchainResponse::FindBlock(Some((Chain::Main, height)))),
        None => (),
    }

    Ok(BlockchainResponse::FindBlock(None))
    /*
    match db.alt_block_heights.get(&tx_ro, &block_hash)? {
        Some(height) => Ok(BlockchainResponse::FindBlock(Some((
            Chain::Alt(height.chain_id.into()),
            height.height,
        )))),
        None => Ok(BlockchainResponse::FindBlock(None)),
    }

     */
}

/// [`BlockchainReadRequest::FilterUnknownHashes`].
#[inline]
fn filter_unknown_hashes(
    db: &BlockchainDatabase,
    mut hashes: HashSet<BlockHash>,
) -> ResponseResult {
    let tx_ro = db.fjall_keyspace.snapshot();

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
    let tx_ro = db.fjall_keyspace.snapshot();

    let tapes = db.linear_tapes.reader();

    // Collect results using `rayon`.
    let vec = match chain {
        Chain::Main => range
            .into_iter()
            .map(|block_height| get_block_extended_header_from_height(block_height, &tapes, db))
            .collect::<DbResult<Vec<ExtendedBlockHeader>>>()?,
        Chain::Alt(chain_id) => {
            todo!()
            /*
            let ranges = { get_alt_chain_history_ranges(db, range, chain_id, &tx_ro)? };

            ranges
                .iter()
                .rev()
                .flat_map(|(chain, range)| {
                    range.clone().into_iter().map(|height| match *chain {
                        Chain::Main => get_block_extended_header_from_height(height, &tapes),
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

             */
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
        chain_height as usize,
        block_hash,
    ))
}

/// [`BlockchainReadRequest::GeneratedCoins`].
#[inline]
fn generated_coins(db: &BlockchainDatabase, height: usize) -> ResponseResult {
    let tapes = db.linear_tapes.reader();

    Ok(BlockchainResponse::GeneratedCoins(
        tapes
            .read_entry(&db.block_infos, height as u64)?
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
    let tx_ro = db.fjall_keyspace.snapshot();
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
    Ok(BlockchainResponse::OutputsVec(todo!()))
}

/// [`BlockchainReadRequest::NumberOutputsWithAmount`].
#[inline]
fn number_outputs_with_amount(db: &BlockchainDatabase, amounts: Vec<Amount>) -> ResponseResult {
    let tx_ro = db.fjall_keyspace.snapshot();
    let tapes = db.linear_tapes.reader();

    // Cache the amount of RCT outputs once.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "INVARIANT: #[cfg] @ lib.rs asserts `usize == u64`"
    )]
    let num_rct_outputs = tapes
        .fixed_sized_tape_len(&db.rct_outputs)
        .expect("Required tape not found") as usize;

    // Collect results using `rayon`.
    let map = amounts
        .into_iter()
        .map(|amount| {
            if amount == 0 {
                // v2 transactions.
                Ok((amount, num_rct_outputs))
            } else {
                // v1 transactions.
                match get_num_outputs_with_amount(db, &tx_ro, amount) {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "INVARIANT: #[cfg] @ lib.rs asserts `usize == u64`"
                    )]
                    Ok(count) => Ok((amount, count as usize)),
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
    let tx_ro = db.fjall_keyspace.snapshot();

    // FIXME:
    // Create/use `enum cuprate_types::Exist { Does, DoesNot }`
    // or similar instead of `bool` for clarity.
    // <https://github.com/Cuprate/cuprate/pull/113#discussion_r1581536526>
    //
    // Collect results using `rayon`.
    match key_images
        .into_iter()
        .map(|ki| tx_ro.contains_key(&db.key_images, &ki))
        // If the result is either:
        // `Ok(true)` => a key image was found, return early
        // `Err` => an error was found, return early
        //
        // Else, `Ok(false)` will continue the iterator.
        .find(|result| !matches!(result, Ok(true)))
    {
        None | Some(Ok(false)) => Ok(BlockchainResponse::KeyImagesSpent(false)), // Key image was NOT found.
        Some(Ok(true)) => Ok(BlockchainResponse::KeyImagesSpent(true)), // Key image was found.
        Some(Err(e)) => Err(e).expect("TODO"), // A database error occurred.
    }
}

/// [`BlockchainReadRequest::KeyImagesSpentVec`].
fn key_images_spent_vec(db: &BlockchainDatabase, key_images: Vec<KeyImage>) -> ResponseResult {
    let tx_ro = db.fjall_keyspace.snapshot();

    // Collect results using `rayon`.
    Ok(BlockchainResponse::KeyImagesSpentVec(
        key_images
            .into_iter()
            .map(|ki| tx_ro.contains_key(&db.key_images, &ki))
            .collect::<Result<_, _>>()
            .expect("TODO"),
    ))
}

/// [`BlockchainReadRequest::CompactChainHistory`]
fn compact_chain_history(db: &BlockchainDatabase) -> ResponseResult {
    let tapes = db.linear_tapes.reader();

    let get_block_info = |height| -> Result<_, BlockchainError> {
        Ok(tapes
            .read_entry(&db.block_infos, height)?
            .ok_or(BlockchainError::NotFound)?)
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
        .map_while(|i| top_block_height.checked_sub(i as u64))
        .map(|height| Ok(get_block_info(height)?.block_hash))
        .collect::<DbResult<Vec<_>>>()?;

    if compact_history_genesis_not_included::<INITIAL_BLOCKS>(top_block_height as usize) {
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
    let tx_ro = db.fjall_keyspace.snapshot();

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
    let first_known_height = block_height(db, &tx_ro, &first_known_block_hash)?.unwrap();

    let chain_height = crate::ops::blockchain::chain_height(db, &tapes)?;
    let last_height_in_chain_entry = min(first_known_height + next_entry_size, chain_height);

    let (block_ids, block_weights) = (first_known_height..last_height_in_chain_entry)
        .map(|height| {
            let block_info = tapes
                .read_entry(&db.block_infos, height as u64)?
                .ok_or(BlockchainError::NotFound)?;

            Ok((block_info.block_hash, block_info.weight))
        })
        .collect::<DbResult<(Vec<_>, Vec<_>)>>()?;

    let top_block_info = tapes
        .read_entry(&db.block_infos, chain_height as u64 - 1)?
        .ok_or(BlockchainError::NotFound)?;

    let first_block_blob = if block_ids.len() >= 2 {
        Some(get_block(&(first_known_height + 1), &tapes, db)?.serialize())
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
    let tx_ro = db.fjall_keyspace.snapshot();

    let idx = find_split_point(db, block_ids, true, true, &tx_ro)?;

    Ok(if idx == block_ids.len() {
        BlockchainResponse::FindFirstUnknown(None)
    } else if idx == 0 {
        BlockchainResponse::FindFirstUnknown(Some((0, 0)))
    } else {
        let last_known_height = usize::from_le_bytes(
            tx_ro
                .get(&db.block_heights, &block_ids[idx - 1])?
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
    let tx_ro = db.fjall_keyspace.snapshot();

    let tapes = db.linear_tapes.reader();

    let block_height = usize::from_le_bytes(
        tx_ro
            .get(&db.block_heights, &block_hash)?
            .ok_or(BlockchainError::NotFound)?
            .as_ref()
            .try_into()
            .unwrap(),
    );

    let block_info = tapes
        .read_entry(&db.block_infos, block_height as u64)?
        .ok_or(BlockchainError::NotFound)?;

    let block = get_block(&block_height, &tapes, db)?;

    if block.transactions.len() < missing_txs.len() {
        return Ok(BlockchainResponse::TxsInBlock(None));
    }

    let txs = missing_txs
        .into_iter()
        .map(|index_offset| {
            Ok(get_tx_blob_from_id(
                &(block_info.mining_tx_index + index_offset),
                &tapes,
                db,
            )?)
        })
        .collect::<DbResult<_>>()?;

    Ok(BlockchainResponse::TxsInBlock(Some(TxsInBlock {
        block: block.serialize(),
        txs,
    })))
}

/// [`BlockchainReadRequest::AltBlocksInChain`]
fn alt_blocks_in_chain(db: &BlockchainDatabase, chain_id: ChainId) -> ResponseResult {
    todo!()
    /*
    let tx_ro = db.dynamic_tables.read_txn()?;

    let tapes = db.linear_tapes.reader().expect("TODO");

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

            range.clone().into_iter().map(|height| {
                get_alt_block(
                    db,
                    &AltBlockHeight {
                        chain_id: (*chain_id).into(),
                        height,
                    },
                    &tx_ro,
                    &tapes,
                )
            })
        })
        .collect::<DbResult<_>>()?;

    Ok(BlockchainResponse::AltBlocksInChain(blocks))

     */
}

/// [`BlockchainReadRequest::Block`]
fn block(db: &BlockchainDatabase, block_height: BlockHeight) -> ResponseResult {
    let tapes = db.linear_tapes.reader();

    Ok(BlockchainResponse::Block(get_block(
        &block_height,
        &tapes,
        db,
    )?))
}

/// [`BlockchainReadRequest::BlockByHash`]
fn block_by_hash(db: &BlockchainDatabase, block_hash: BlockHash) -> ResponseResult {
    let tx_ro = db.fjall_keyspace.snapshot();

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
    Ok(BlockchainResponse::TotalTxCount(todo!()))
}

/// [`BlockchainReadRequest::DatabaseSize`]
fn database_size(db: &BlockchainDatabase) -> ResponseResult {
    Ok(BlockchainResponse::DatabaseSize {
        database_size: todo!(),
        free_space: todo!(),
    })
}

/// [`BlockchainReadRequest::OutputHistogram`]
fn output_histogram(db: &BlockchainDatabase, input: OutputHistogramInput) -> ResponseResult {
    Ok(BlockchainResponse::OutputHistogram(todo!()))
}

/// [`BlockchainReadRequest::CoinbaseTxSum`]
fn coinbase_tx_sum(db: &BlockchainDatabase, height: usize, count: u64) -> ResponseResult {
    Ok(BlockchainResponse::CoinbaseTxSum(todo!()))
}

/// [`BlockchainReadRequest::AltChains`]
fn alt_chains(db: &BlockchainDatabase) -> ResponseResult {
    Ok(BlockchainResponse::AltChains(todo!()))
}

/// [`BlockchainReadRequest::AltChainCount`]
fn alt_chain_count(db: &BlockchainDatabase) -> ResponseResult {
    Ok(BlockchainResponse::AltChainCount(todo!()))
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
    todo!()
    /*
    let tx_ro = db.dynamic_tables.read_txn()?;

    let tx_id = db.tx_ids
        .get(&tx_ro, tx_hash)?
        .ok_or(BlockchainError::NotFound)?;

    let tapes = db.linear_tapes.reader().expect("TODO");
    let tx_infos = tapes.fixed_sized_tape_slice::<TxInfo>(TX_INFOS);

    let tx_info = tx_infos.get(tx_id).ok_or(BlockchainError::NotFound)?;

    let o_indexes = if tx_info.rct_output_start_idx == u64::MAX {
        db.tx_outputs
            .get(&tx_ro, &tx_id)?
            .ok_or(BlockchainError::NotFound)?
    } else {
        (0..tx_info.numb_rct_outputs)
            .map(|i| i as u64 + tx_info.rct_output_start_idx)
            .collect()
    };

    Ok(BlockchainResponse::TxOutputIndexes(o_indexes))

     */
}

/// [`BlockchainReadRequest::OutputDistribution`]
fn output_distribution(db: &BlockchainDatabase, input: OutputDistributionInput) -> ResponseResult {
    Ok(BlockchainResponse::OutputDistribution(todo!()))
}
