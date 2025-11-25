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
    collections::{HashMap, HashSet},
    ops::Range,
    sync::Arc,
};

use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use rayon::{
    iter::{Either, IntoParallelIterator, ParallelIterator},
    prelude::*,
    ThreadPool,
};
use thread_local::ThreadLocal;

use cuprate_database_service::{
    init_thread_pool, DatabaseReadService, ReaderThreads, RuntimeError,
};
use cuprate_helper::map::combine_low_high_bits_to_u128;
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    output_cache::OutputCache,
    rpc::OutputHistogramInput,
    Chain, ChainId, ExtendedBlockHeader, OutputDistributionInput, TxsInBlock,
};

use crate::database::{
    ALT_BLOCK_HEIGHTS, BLOCK_HEIGHTS, BLOCK_INFOS, KEY_IMAGES, RCT_OUTPUTS, TX_IDS, TX_INFOS,
    TX_OUTPUTS,
};
use crate::error::{BlockchainError, DbResult};
use crate::ops::output::get_num_outputs_with_amount;
use crate::ops::tx::get_tx_blob_from_id;
use crate::types::{BlockInfo, RctOutput, TxInfo, ZeroKey};
use crate::{
    ops::{
        alt_block::{
            get_alt_block, get_alt_block_extended_header_from_height, get_alt_block_hash,
            get_alt_chain_history_ranges,
        },
        block::{
            block_exists, get_block, get_block_by_hash, get_block_complete_entry,
            get_block_complete_entry_from_height, get_block_extended_header_from_height,
            get_block_height,
        },
        blockchain::find_split_point,
        output::id_to_output_on_chain,
    },
    service::{
        free::{compact_history_genesis_not_included, compact_history_index_to_height_offset},
        types::{BlockchainReadHandle, ResponseResult},
    },
    types::{
        AltBlockHeight, Amount, AmountIndex, BlockHash, BlockHeight, KeyImage, PreRctOutputId,
    },
    Blockchain,
};

//---------------------------------------------------------------------------------------------------- init_read_service
/// Initialize the [`BlockchainReadHandle`] thread-pool backed by [`rayon`].
///
/// This spawns `threads` amount of reader threads
/// attached to `env` and returns a handle to the pool.
///
/// Should be called _once_ per actual database. Calling this function more than once will create
/// multiple unnecessary rayon thread-pools.
#[cold]
#[inline(never)] // Only called once.
pub fn init_read_service(env: Arc<Blockchain>, threads: ReaderThreads) -> BlockchainReadHandle {
    init_read_service_with_pool(env, init_thread_pool(threads))
}

/// Initialize the blockchain database read service, with a specific rayon thread-pool instead of
/// creating a new one.
///
/// Should be called _once_ per actual database, although nothing bad will happen, cloning the [`BlockchainReadHandle`]
/// is the correct way to get multiple handles to the database.
#[cold]
#[inline(never)] // Only called once.
pub fn init_read_service_with_pool(
    env: Arc<Blockchain>,
    pool: Arc<ThreadPool>,
) -> BlockchainReadHandle {
    DatabaseReadService::new(env, pool, map_request)
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
    env: &Blockchain,               // Access to the database
    request: BlockchainReadRequest, // The request we must fulfill
) -> Result<BlockchainResponse, RuntimeError> {
    use BlockchainReadRequest as R;

    /* SOMEDAY: pre-request handling, run some code for each request? */

    Ok(match request {
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
    .map_err(|e| RuntimeError::Io(std::io::Error::other(e)))?)

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
fn block_complete_entries(db: &Blockchain, block_hashes: Vec<BlockHash>) -> ResponseResult {
    let tx_ro = &db.dynamic_tables.read_txn()?;

    let tapes = db
        .linear_tapes
        .reader()
        .expect("Cuprate should be the only writer to the tapes");

    let mut missing_hashes = Vec::with_capacity(block_hashes.len());
    let mut blocks = Vec::with_capacity(block_hashes.len());
    for res in block_hashes.into_iter().map(|block_hash| {
        match get_block_complete_entry(&block_hash, false, &tx_ro, &tapes) {
            Err(BlockchainError::NotFound) => Ok(Either::Left(block_hash)),
            res => res.map(Either::Right),
        }
    }) {
        match res? {
            Either::Left(l) => missing_hashes.push(l),
            Either::Right(r) => blocks.push(r),
        }
    }

    let blockchain_height = crate::ops::blockchain::chain_height(tx_ro)?;

    Ok(BlockchainResponse::BlockCompleteEntries {
        blocks,
        missing_hashes,
        blockchain_height,
    })
}

/// [`BlockchainReadRequest::BlockCompleteEntriesByHeight`].
fn block_complete_entries_by_height(
    db: &Blockchain,
    block_heights: Vec<BlockHeight>,
) -> ResponseResult {
    let tapes = db
        .linear_tapes
        .reader()
        .expect("Cuprate should be the only writer to the tapes");

    let blocks = block_heights
        .into_par_iter()
        .map(|height| get_block_complete_entry_from_height(height, false, &tapes))
        .collect::<DbResult<_>>()?;

    Ok(BlockchainResponse::BlockCompleteEntriesByHeight(blocks))
}

/// [`BlockchainReadRequest::BlockExtendedHeader`].
#[inline]
fn block_extended_header(db: &Blockchain, block_height: BlockHeight) -> ResponseResult {
    let tapes = db
        .linear_tapes
        .reader()
        .expect("Cuprate should be the only writer to the tapes");

    Ok(BlockchainResponse::BlockExtendedHeader(
        get_block_extended_header_from_height(block_height, &tapes)?,
    ))
}

/// [`BlockchainReadRequest::BlockHash`].
#[inline]
fn block_hash(db: &Blockchain, block_height: BlockHeight, chain: Chain) -> ResponseResult {
    let tx_ro = db.dynamic_tables.read_txn()?;

    let tapes = db
        .linear_tapes
        .reader()
        .expect("Cuprate should be the only writer to the tapes");

    let block_hash = match chain {
        Chain::Main => {
            tapes
                .fixed_sized_tape_slice::<BlockInfo>(BLOCK_INFOS)
                .get(block_height)
                .ok_or(BlockchainError::NotFound)?
                .block_hash
        }
        Chain::Alt(chain) => get_alt_block_hash(&block_height, chain, &tx_ro, &tapes)?,
    };

    Ok(BlockchainResponse::BlockHash(block_hash))
}

/// [`BlockchainReadRequest::BlockHashInRange`].
#[inline]
fn block_hash_in_range(db: &Blockchain, range: Range<usize>, chain: Chain) -> ResponseResult {
    let tx_ro = db.dynamic_tables.read_txn()?;

    let tapes = db
        .linear_tapes
        .reader()
        .expect("Cuprate should be the only writer to the tapes");

    let block_hash = range
        .into_iter()
        .map(|block_height| {
            let block_hash = match chain {
                Chain::Main => {
                    tapes
                        .fixed_sized_tape_slice::<BlockInfo>(BLOCK_INFOS)
                        .get(block_height)
                        .ok_or(BlockchainError::NotFound)?
                        .block_hash
                }
                Chain::Alt(chain) => get_alt_block_hash(&block_height, chain, &tx_ro, &tapes)?,
            };

            Ok(block_hash)
        })
        .collect::<Result<_, BlockchainError>>()?;

    Ok(BlockchainResponse::BlockHashInRange(block_hash))
}

/// [`BlockchainReadRequest::FindBlock`]
fn find_block(db: &Blockchain, block_hash: BlockHash) -> ResponseResult {
    let tx_ro = db.dynamic_tables.read_txn()?;

    // Check the main chain first.
    match BLOCK_HEIGHTS.get().unwrap().get(&tx_ro, &block_hash)? {
        Some(height) => return Ok(BlockchainResponse::FindBlock(Some((Chain::Main, height)))),
        None => (),
    }

    match ALT_BLOCK_HEIGHTS.get().unwrap().get(&tx_ro, &block_hash)? {
        Some(height) => Ok(BlockchainResponse::FindBlock(Some((
            Chain::Alt(height.chain_id.into()),
            height.height,
        )))),
        None => Ok(BlockchainResponse::FindBlock(None)),
    }
}

/// [`BlockchainReadRequest::FilterUnknownHashes`].
#[inline]
fn filter_unknown_hashes(db: &Blockchain, mut hashes: HashSet<BlockHash>) -> ResponseResult {
    let tx_ro = db.dynamic_tables.read_txn()?;

    let mut err = None;

    hashes.retain(|block_hash| match block_exists(block_hash, &tx_ro) {
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
    db: &Blockchain,
    range: Range<BlockHeight>,
    chain: Chain,
) -> ResponseResult {
    let tx_ro = db.dynamic_tables.read_txn()?;

    let tapes = db
        .linear_tapes
        .reader()
        .expect("Cuprate should be the only writer to the tapes");

    // Collect results using `rayon`.
    let vec = match chain {
        Chain::Main => range
            .into_iter()
            .map(|block_height| get_block_extended_header_from_height(block_height, &tapes))
            .collect::<DbResult<Vec<ExtendedBlockHeader>>>()?,
        Chain::Alt(chain_id) => {
            let ranges = { get_alt_chain_history_ranges(range, chain_id, &tx_ro)? };

            ranges
                .iter()
                .rev()
                .flat_map(|(chain, range)| {
                    range.clone().into_iter().map(|height| match *chain {
                        Chain::Main => get_block_extended_header_from_height(height, &tapes),
                        Chain::Alt(chain_id) => get_alt_block_extended_header_from_height(
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
fn chain_height(db: &Blockchain) -> ResponseResult {
    let tapes = db
        .linear_tapes
        .reader()
        .expect("Cuprate should be the only writer to the tapes");
    let block_infos = tapes.fixed_sized_tape_slice::<BlockInfo>(BLOCK_INFOS);

    let chain_height = block_infos.len();

    if chain_height == 0 {
        return Err(BlockchainError::NotFound);
    }

    let block_hash = block_infos
        .last()
        .unwrap()
        .block_hash;

    Ok(BlockchainResponse::ChainHeight(chain_height, block_hash))
}

/// [`BlockchainReadRequest::GeneratedCoins`].
#[inline]
fn generated_coins(db: &Blockchain, height: usize) -> ResponseResult {
    let tapes = db
        .linear_tapes
        .reader()
        .expect("Cuprate should be the only writer to the tapes");
    let block_infos = tapes.fixed_sized_tape_slice::<BlockInfo>(BLOCK_INFOS);

    Ok(BlockchainResponse::GeneratedCoins(
        block_infos
            .get(height)
            .map(|b| b.cumulative_generated_coins)
            .unwrap_or(0)
    ))
}

/// [`BlockchainReadRequest::Outputs`].
#[inline]
fn outputs(
    db: &Blockchain,
    outputs: IndexMap<Amount, IndexSet<AmountIndex>>,
    get_txid: bool,
) -> ResponseResult {
    // Prepare tx/tables in `ThreadLocal`.

    // TODO: we need to ensure the tables & tapes are in sync here.
    let tx_ro = db.dynamic_tables.read_txn()?;
    let tapes = db
        .linear_tapes
        .reader()
        .expect("Cuprate should be the only writer to the tapes");
    
    let rct_tape = tapes.fixed_sized_tape_slice::<RctOutput>(RCT_OUTPUTS);

    let amount_of_outs = outputs
        .iter()
        .map(|(&amount, _)| {
            if amount == 0 {
                Ok((
                    amount,
                    tapes
                        .fixed_sized_tape_slice::<RctOutput>(RCT_OUTPUTS)
                        .len() as u64,
                ))
            } else {
                // v1 transactions.
                match get_num_outputs_with_amount(&tx_ro, amount) {
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

        let output_on_chain = match id_to_output_on_chain(&id, get_txid, &tx_ro, &tapes, &rct_tape) {
            Ok(output) => output,
            Err(BlockchainError::NotFound) => return Ok(Either::Right(amount_index)),
            Err(e) => return Err(e),
        };

        Ok(Either::Left((amount_index, output_on_chain)))
    };

    let mut map = IndexMap::<_, IndexMap<_, _>>::with_capacity(outputs.len());
    let mut wanted_outputs = IndexMap::<_, IndexSet<_>>::with_capacity(outputs.len());

    for res in outputs.into_iter().flat_map(|(amount, amount_index_set)| {
        amount_index_set
            .into_iter()
            .map(move |amount_index| inner_map(amount, amount_index))
            .map_ok(move |e| e.map_either(|l| (amount, l), |r| (amount, r)))
    }) {
        match res? {
            Either::Left((amount, v)) => {
                map.entry(amount).or_default().insert(v.0, v.1);
            }
            Either::Right((amount, index)) => {
                wanted_outputs.entry(amount).or_default().insert(index);
            }
        }
    }

    let cache = OutputCache::new(map, amount_of_outs, wanted_outputs);

    Ok(BlockchainResponse::Outputs(cache))
}

/// [`BlockchainReadRequest::OutputsVec`].
#[inline]
fn outputs_vec(
    db: &Blockchain,
    outputs: Vec<(Amount, AmountIndex)>,
    get_txid: bool,
) -> ResponseResult {
    Ok(BlockchainResponse::OutputsVec(todo!()))
}

/// [`BlockchainReadRequest::NumberOutputsWithAmount`].
#[inline]
fn number_outputs_with_amount(db: &Blockchain, amounts: Vec<Amount>) -> ResponseResult {
    let tx_ro = db.dynamic_tables.read_txn()?;
    let tapes = db
        .linear_tapes
        .reader()
        .expect("Cuprate should be the only writer to the tapes");

    // Cache the amount of RCT outputs once.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "INVARIANT: #[cfg] @ lib.rs asserts `usize == u64`"
    )]
    let num_rct_outputs = tapes
        .fixed_sized_tape_slice::<RctOutput>(RCT_OUTPUTS)
        .len();

    // Collect results using `rayon`.
    let map = amounts
        .into_iter()
        .map(|amount| {
            if amount == 0 {
                // v2 transactions.
                Ok((amount, num_rct_outputs))
            } else {
                // v1 transactions.
                match get_num_outputs_with_amount(&tx_ro, amount) {
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
fn key_images_spent(db: &Blockchain, key_images: HashSet<KeyImage>) -> ResponseResult {
    let tx_ro = db.dynamic_tables.read_txn()?;

    // FIXME:
    // Create/use `enum cuprate_types::Exist { Does, DoesNot }`
    // or similar instead of `bool` for clarity.
    // <https://github.com/Cuprate/cuprate/pull/113#discussion_r1581536526>
    //
    // Collect results using `rayon`.
    match key_images
        .into_iter()
        .map(|ki| {
            KEY_IMAGES
                .get()
                .unwrap()
                .get_duplicate(&tx_ro, &ZeroKey, &ki)
        })
        // If the result is either:
        // `Ok(true)` => a key image was found, return early
        // `Err` => an error was found, return early
        //
        // Else, `Ok(false)` will continue the iterator.
        .find(|result| !matches!(result, Ok(None)))
    {
        None | Some(Ok(None)) => Ok(BlockchainResponse::KeyImagesSpent(false)), // Key image was NOT found.
        Some(Ok(Some(_))) => Ok(BlockchainResponse::KeyImagesSpent(true)), // Key image was found.
        Some(Err(e)) => Err(e)?, // A database error occurred.
    }
}

/// [`BlockchainReadRequest::KeyImagesSpentVec`].
fn key_images_spent_vec(db: &Blockchain, key_images: Vec<KeyImage>) -> ResponseResult {
    let tx_ro = db.dynamic_tables.read_txn()?;

    // Collect results using `rayon`.
    Ok(BlockchainResponse::KeyImagesSpentVec(
        key_images
            .into_iter()
            .map(|ki| {
                Ok(KEY_IMAGES
                    .get()
                    .unwrap()
                    .get_duplicate(&tx_ro, &ZeroKey, &ki)?
                    .is_some())
            })
            .collect::<DbResult<_>>()?,
    ))
}

/// [`BlockchainReadRequest::CompactChainHistory`]
fn compact_chain_history(db: &Blockchain) -> ResponseResult {
    let tapes = db
        .linear_tapes
        .reader()
        .expect("Cuprate should be the only writer to the tapes");

    let block_infos = tapes.fixed_sized_tape_slice::<BlockInfo>(BLOCK_INFOS);

    let get_block_info = |height| block_infos.get(height).ok_or(BlockchainError::NotFound);

    let top_block_height = block_infos.len() - 1;

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
        .map_while(|i| top_block_height.checked_sub(i))
        .map(|height| Ok(get_block_info(height)?.block_hash))
        .collect::<DbResult<Vec<_>>>()?;

    if compact_history_genesis_not_included::<INITIAL_BLOCKS>(top_block_height) {
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
    db: &Blockchain,
    block_ids: &[BlockHash],
    next_entry_size: usize,
) -> ResponseResult {
    let tx_ro = db.dynamic_tables.read_txn()?;

    let tapes = db
        .linear_tapes
        .reader()
        .expect("Cuprate should be the only writer to the tapes");

    let block_infos = tapes.fixed_sized_tape_slice::<BlockInfo>(BLOCK_INFOS);

    let idx = find_split_point(block_ids, false, false, &tx_ro)?;

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
    let first_known_height = BLOCK_HEIGHTS
        .get()
        .unwrap()
        .get(&tx_ro, &first_known_block_hash)?
        .ok_or(BlockchainError::NotFound)?;

    let chain_height = crate::ops::blockchain::chain_height(&tx_ro)?;
    let last_height_in_chain_entry = min(first_known_height + next_entry_size, chain_height);

    let (block_ids, block_weights) = (first_known_height..last_height_in_chain_entry)
        .map(|height| {
            let block_info = block_infos
                .get(height)
                .ok_or(BlockchainError::NotFound)?;

            Ok((block_info.block_hash, block_info.weight))
        })
        .collect::<DbResult<(Vec<_>, Vec<_>)>>()?;

    let top_block_info = block_infos
        .get(chain_height - 1)
        .ok_or(BlockchainError::NotFound)?;

    let first_block_blob = if block_ids.len() >= 2 {
        Some(get_block(&(first_known_height + 1), &tapes)?.serialize())
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
fn find_first_unknown(db: &Blockchain, block_ids: &[BlockHash]) -> ResponseResult {
    let tx_ro = db.dynamic_tables.read_txn()?;

    let idx = find_split_point(block_ids, true, true, &tx_ro)?;

    Ok(if idx == block_ids.len() {
        BlockchainResponse::FindFirstUnknown(None)
    } else if idx == 0 {
        BlockchainResponse::FindFirstUnknown(Some((0, 0)))
    } else {
        let last_known_height = get_block_height(&block_ids[idx - 1], &tx_ro)?;

        BlockchainResponse::FindFirstUnknown(Some((idx, last_known_height + 1)))
    })
}

/// [`BlockchainReadRequest::TxsInBlock`]
fn txs_in_block(db: &Blockchain, block_hash: [u8; 32], missing_txs: Vec<u64>) -> ResponseResult {
    let tx_ro = db.dynamic_tables.read_txn()?;

    let tapes = db.linear_tapes.reader().expect("TODO");

    let block_height = BLOCK_HEIGHTS
        .get()
        .unwrap()
        .get(&tx_ro, &block_hash)?
        .ok_or(BlockchainError::NotFound)?;
    let block_info_tape_reader = tapes.fixed_sized_tape_slice::<BlockInfo>(BLOCK_INFOS);

    let block_info = block_info_tape_reader
        .get(block_height)
        .ok_or(BlockchainError::NotFound)?;

    let block = get_block(&block_height, &tapes)?;

    if block.transactions.len() < missing_txs.len() {
        return Ok(BlockchainResponse::TxsInBlock(None));
    }

    let txs = missing_txs
        .into_iter()
        .map(|index_offset| {
            Ok(get_tx_blob_from_id(
                &(block_info.mining_tx_index + index_offset as usize),
                &tapes,
            )?)
        })
        .collect::<DbResult<_>>()?;

    Ok(BlockchainResponse::TxsInBlock(Some(TxsInBlock {
        block: block.serialize(),
        txs,
    })))
}

/// [`BlockchainReadRequest::AltBlocksInChain`]
fn alt_blocks_in_chain(db: &Blockchain, chain_id: ChainId) -> ResponseResult {
    let tx_ro = db.dynamic_tables.read_txn()?;

    let tapes = db.linear_tapes.reader().expect("TODO");

    // Get the history of this alt-chain.
    let history = { get_alt_chain_history_ranges(0..usize::MAX, chain_id, &tx_ro)? };

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
}

/// [`BlockchainReadRequest::Block`]
fn block(db: &Blockchain, block_height: BlockHeight) -> ResponseResult {
    let tapes = db.linear_tapes.reader().expect("TODO");

    Ok(BlockchainResponse::Block(get_block(&block_height, &tapes)?))
}

/// [`BlockchainReadRequest::BlockByHash`]
fn block_by_hash(db: &Blockchain, block_hash: BlockHash) -> ResponseResult {
    let tx_ro = db.dynamic_tables.read_txn()?;

    let tapes = db.linear_tapes.reader().expect("TODO");

    Ok(BlockchainResponse::Block(get_block_by_hash(
        &block_hash,
        &tx_ro,
        &tapes,
    )?))
}

/// [`BlockchainReadRequest::TotalTxCount`]
fn total_tx_count(db: &Blockchain) -> ResponseResult {
    Ok(BlockchainResponse::TotalTxCount(todo!()))
}

/// [`BlockchainReadRequest::DatabaseSize`]
fn database_size(db: &Blockchain) -> ResponseResult {
    Ok(BlockchainResponse::DatabaseSize {
        database_size: todo!(),
        free_space: todo!(),
    })
}

/// [`BlockchainReadRequest::OutputHistogram`]
fn output_histogram(db: &Blockchain, input: OutputHistogramInput) -> ResponseResult {
    Ok(BlockchainResponse::OutputHistogram(todo!()))
}

/// [`BlockchainReadRequest::CoinbaseTxSum`]
fn coinbase_tx_sum(db: &Blockchain, height: usize, count: u64) -> ResponseResult {
    Ok(BlockchainResponse::CoinbaseTxSum(todo!()))
}

/// [`BlockchainReadRequest::AltChains`]
fn alt_chains(db: &Blockchain) -> ResponseResult {
    Ok(BlockchainResponse::AltChains(todo!()))
}

/// [`BlockchainReadRequest::AltChainCount`]
fn alt_chain_count(db: &Blockchain) -> ResponseResult {
    Ok(BlockchainResponse::AltChainCount(todo!()))
}

/// [`BlockchainReadRequest::Transactions`]
fn transactions(db: &Blockchain, tx_hashes: HashSet<[u8; 32]>) -> ResponseResult {
    Ok(BlockchainResponse::Transactions {
        txs: todo!(),
        missed_txs: todo!(),
    })
}

/// [`BlockchainReadRequest::TotalRctOutputs`]
fn total_rct_outputs(db: &Blockchain) -> ResponseResult {
    let tapes = db.linear_tapes.reader().expect("TODO");

    let len = tapes
        .fixed_sized_tape_slice::<RctOutput>(RCT_OUTPUTS)
        .len() as u64;

    Ok(BlockchainResponse::TotalRctOutputs(len))
}

/// [`BlockchainReadRequest::TxOutputIndexes`]
fn tx_output_indexes(db: &Blockchain, tx_hash: &[u8; 32]) -> ResponseResult {
    let tx_ro = db.dynamic_tables.read_txn()?;

    let tx_id = TX_IDS
        .get()
        .unwrap()
        .get(&tx_ro, tx_hash)?
        .ok_or(BlockchainError::NotFound)?;

    let tapes = db.linear_tapes.reader().expect("TODO");
    let tx_infos = tapes
        .fixed_sized_tape_slice::<TxInfo>(TX_INFOS);


    let tx_info = tx_infos
        .get(tx_id)
        .ok_or(BlockchainError::NotFound)?;

    let o_indexes = if tx_info.rct_output_start_idx == u64::MAX {
        TX_OUTPUTS
            .get()
            .unwrap()
            .get(&tx_ro, &tx_id)?
            .ok_or(BlockchainError::NotFound)?
    } else {
        (0..tx_info.numb_rct_outputs)
            .map(|i| i as u64 + tx_info.rct_output_start_idx)
            .collect()
    };

    Ok(BlockchainResponse::TxOutputIndexes(o_indexes))
}

/// [`BlockchainReadRequest::OutputDistribution`]
fn output_distribution(db: &Blockchain, input: OutputDistributionInput) -> ResponseResult {
    Ok(BlockchainResponse::OutputDistribution(todo!()))
}
