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
use rayon::{
    iter::{Either, IntoParallelIterator, ParallelIterator},
    prelude::*,
    ThreadPool,
};
use thread_local::ThreadLocal;

use cuprate_database::{ConcreteEnv, DatabaseRo, DbResult, Env, EnvInner, RuntimeError};
use cuprate_database_service::{init_thread_pool, DatabaseReadService, ReaderThreads};
use cuprate_helper::map::combine_low_high_bits_to_u128;
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    output_cache::OutputCache,
    rpc::OutputHistogramInput,
    Chain, ChainId, ExtendedBlockHeader, OutputDistributionInput, TxsInBlock,
};

use crate::{
    ops::{
        alt_block::{
            get_alt_block, get_alt_block_extended_header_from_height, get_alt_block_hash,
            get_alt_chain_history_ranges,
        },
        block::{
            block_exists, get_block, get_block_blob_with_tx_indexes, get_block_by_hash,
            get_block_complete_entry, get_block_complete_entry_from_height,
            get_block_extended_header_from_height, get_block_height, get_block_info,
        },
        blockchain::{cumulative_generated_coins, find_split_point, top_block_height},
        key_image::key_image_exists,
        output::id_to_output_on_chain,
    },
    service::{
        free::{compact_history_genesis_not_included, compact_history_index_to_height_offset},
        types::{BlockchainReadHandle, ResponseResult},
    },
    tables::{
        AltBlockHeights, BlockHeights, BlockInfos, OpenTables, RctOutputs, Tables, TablesIter,
        TxIds, TxOutputs,
    },
    types::{
        AltBlockHeight, Amount, AmountIndex, BlockHash, BlockHeight, KeyImage, PreRctOutputId,
    },
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
pub fn init_read_service(env: Arc<ConcreteEnv>, threads: ReaderThreads) -> BlockchainReadHandle {
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
    env: Arc<ConcreteEnv>,
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
fn map_request<E>(
    env: &E,                        // Access to the database
    request: BlockchainReadRequest, // The request we must fulfill
) -> ResponseResult
where
    E: Env + 'static,
    for <'a> <E as Env>::EnvInner<'a>: Sync,
    for <'a, 'b> <<E as Env>::EnvInner<'a> as OpenTables<'a>>::Ro<'b>: Send,
{
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

//---------------------------------------------------------------------------------------------------- Thread Local
/// Q: Why does this exist?
///
/// A1: `heed`'s transactions and tables are not `Sync`, so we cannot use
/// them with rayon, however, we set a feature such that they are `Send`.
///
/// A2: When sending to rayon, we want to ensure each read transaction
/// is only being used by 1 thread only to scale reads
///
/// <https://github.com/Cuprate/cuprate/pull/113#discussion_r1576762346>
#[inline]
fn thread_local<T: Send>(env: &impl Env) -> ThreadLocal<T> {
    ThreadLocal::with_capacity(env.config().reader_threads.get())
}

/// Take in a `ThreadLocal<impl Tables>` and return an `&impl Tables + Send`.
///
/// # Safety
/// See [`DatabaseRo`] docs.
///
/// We are safely using `UnsafeSendable` in `service`'s reader thread-pool
/// as we are pairing our usage with `ThreadLocal` - only 1 thread
/// will ever access a transaction at a time. This is an INVARIANT.
///
/// A `Mutex` was considered but:
/// - It is less performant
/// - It isn't technically needed for safety in our use-case
/// - It causes `DatabaseIter` function return issues as there is a `MutexGuard` object
///
/// <https://github.com/Cuprate/cuprate/pull/113#discussion_r1581684698>
///
/// # Notes
/// This is used for other backends as well instead of branching with `cfg_if`.
/// The other backends (as of current) are `Send + Sync` so this is fine.
/// <https://github.com/Cuprate/cuprate/pull/113#discussion_r1585618374>
macro_rules! get_tables {
    ($env_inner:ident, $tx_ro:ident, $tables:ident) => {{
        $tables.get_or_try(|| {
            match $env_inner.open_tables($tx_ro) {
                // SAFETY: see above macro doc comment.
                Ok(tables) => Ok(unsafe { crate::unsafe_sendable::UnsafeSendable::new(tables) }),
                Err(e) => Err(e),
            }
        })
    }};
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
fn block_complete_entries<E>(env: &E, block_hashes: Vec<BlockHash>) -> ResponseResult
where
    E: Env + 'static,
    for <'a> <E as Env>::EnvInner<'a>: Sync,
    for <'a, 'b> <<E as Env>::EnvInner<'a> as OpenTables<'a>>::Ro<'b>: Send,
{
    // Prepare tx/tables in `ThreadLocal`.
    let env_inner = env.env_inner();
    let tx_ro = thread_local(env);
    let tables = thread_local(env);

    let (missing_hashes, blocks) = block_hashes
        .into_par_iter()
        .map(|block_hash| {
            let tx_ro = tx_ro.get_or_try(|| env_inner.tx_ro())?;
            let tables = get_tables!(env_inner, tx_ro, tables)?.as_ref();

            match get_block_complete_entry(&block_hash, tables) {
                Err(RuntimeError::KeyNotFound) => Ok(Either::Left(block_hash)),
                res => res.map(Either::Right),
            }
        })
        .collect::<DbResult<_>>()?;

    let tx_ro = tx_ro.get_or_try(|| env_inner.tx_ro())?;
    let tables = get_tables!(env_inner, tx_ro, tables)?.as_ref();

    let blockchain_height = crate::ops::blockchain::chain_height(tables.block_heights())?;

    Ok(BlockchainResponse::BlockCompleteEntries {
        blocks,
        missing_hashes,
        blockchain_height,
    })
}

/// [`BlockchainReadRequest::BlockCompleteEntriesByHeight`].
fn block_complete_entries_by_height<E>(
    env: &E,
    block_heights: Vec<BlockHeight>,
) -> ResponseResult 
where
    E: Env + 'static,
    for <'a> <E as Env>::EnvInner<'a>: Sync,
    for <'a, 'b> <<E as Env>::EnvInner<'a> as OpenTables<'a>>::Ro<'b>: Send,
{
    // Prepare tx/tables in `ThreadLocal`.
    let env_inner = env.env_inner();
    let tx_ro = thread_local(env);
    let tables = thread_local(env);

    let blocks = block_heights
        .into_par_iter()
        .map(|height| {
            let tx_ro = tx_ro.get_or_try(|| env_inner.tx_ro())?;
            let tables = get_tables!(env_inner, tx_ro, tables)?.as_ref();
            get_block_complete_entry_from_height(&height, tables)
        })
        .collect::<DbResult<_>>()?;

    let tx_ro = tx_ro.get_or_try(|| env_inner.tx_ro())?;
    let tables = get_tables!(env_inner, tx_ro, tables)?.as_ref();

    Ok(BlockchainResponse::BlockCompleteEntriesByHeight(blocks))
}

/// [`BlockchainReadRequest::BlockExtendedHeader`].
#[inline]
fn block_extended_header<E: Env>(env: &E, block_height: BlockHeight) -> ResponseResult {
    // Single-threaded, no `ThreadLocal` required.
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;
    let tables = env_inner.open_tables(&tx_ro)?;

    Ok(BlockchainResponse::BlockExtendedHeader(
        get_block_extended_header_from_height(&block_height, &tables)?,
    ))
}

/// [`BlockchainReadRequest::BlockHash`].
#[inline]
fn block_hash<E: Env>(env: &E, block_height: BlockHeight, chain: Chain) -> ResponseResult {
    // Single-threaded, no `ThreadLocal` required.
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;
    let table_block_infos = env_inner.open_db_ro::<BlockInfos>(&tx_ro)?;

    let block_hash = match chain {
        Chain::Main => get_block_info(&block_height, &table_block_infos)?.block_hash,
        Chain::Alt(chain) => {
            get_alt_block_hash(&block_height, chain, &env_inner.open_tables(&tx_ro)?)?
        }
    };

    Ok(BlockchainResponse::BlockHash(block_hash))
}

/// [`BlockchainReadRequest::BlockHashInRange`].
#[inline]
fn block_hash_in_range<E>(env: &E, range: Range<usize>, chain: Chain) -> ResponseResult
where
    E: Env + 'static,
    for <'a> <E as Env>::EnvInner<'a>: Sync,
    for <'a, 'b> <<E as Env>::EnvInner<'a> as OpenTables<'a>>::Ro<'b>: Send,
{
    // Prepare tx/tables in `ThreadLocal`.
    let env_inner = env.env_inner();
    let tx_ro = thread_local(env);

    let block_hash = range
        .into_par_iter()
        .map(|block_height| {
            let tx_ro = tx_ro.get_or_try(|| env_inner.tx_ro())?;

            let table_block_infos = env_inner.open_db_ro::<BlockInfos>(tx_ro)?;

            let block_hash = match chain {
                Chain::Main => get_block_info(&block_height, &table_block_infos)?.block_hash,
                Chain::Alt(chain) => {
                    get_alt_block_hash(&block_height, chain, &env_inner.open_tables(tx_ro)?)?
                }
            };

            Ok(block_hash)
        })
        .collect::<Result<_, RuntimeError>>()?;

    Ok(BlockchainResponse::BlockHashInRange(block_hash))
}

/// [`BlockchainReadRequest::FindBlock`]
fn find_block<E: Env>(env: &E, block_hash: BlockHash) -> ResponseResult {
    // Single-threaded, no `ThreadLocal` required.
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;

    let table_block_heights = env_inner.open_db_ro::<BlockHeights>(&tx_ro)?;

    // Check the main chain first.
    match table_block_heights.get(&block_hash) {
        Ok(height) => return Ok(BlockchainResponse::FindBlock(Some((Chain::Main, height)))),
        Err(RuntimeError::KeyNotFound) => (),
        Err(e) => return Err(e),
    }

    let table_alt_block_heights = env_inner.open_db_ro::<AltBlockHeights>(&tx_ro)?;

    match table_alt_block_heights.get(&block_hash) {
        Ok(height) => Ok(BlockchainResponse::FindBlock(Some((
            Chain::Alt(height.chain_id.into()),
            height.height,
        )))),
        Err(RuntimeError::KeyNotFound) => Ok(BlockchainResponse::FindBlock(None)),
        Err(e) => Err(e),
    }
}

/// [`BlockchainReadRequest::FilterUnknownHashes`].
#[inline]
fn filter_unknown_hashes<E: Env>(env: &E, mut hashes: HashSet<BlockHash>) -> ResponseResult {
    // Single-threaded, no `ThreadLocal` required.
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;

    let table_block_heights = env_inner.open_db_ro::<BlockHeights>(&tx_ro)?;

    let mut err = None;

    hashes.retain(
        |block_hash| match block_exists(block_hash, &table_block_heights) {
            Ok(exists) => exists,
            Err(e) => {
                err.get_or_insert(e);
                false
            }
        },
    );

    if let Some(e) = err {
        Err(e)
    } else {
        Ok(BlockchainResponse::FilterUnknownHashes(hashes))
    }
}

/// [`BlockchainReadRequest::BlockExtendedHeaderInRange`].
#[inline]
fn block_extended_header_in_range<E>(
    env: &E,
    range: Range<BlockHeight>,
    chain: Chain,
) -> ResponseResult
where
    E: Env + 'static,
    for <'a> <E as Env>::EnvInner<'a>: Sync,
    for <'a, 'b> <<E as Env>::EnvInner<'a> as OpenTables<'a>>::Ro<'b>: Send,
{
    // Prepare tx/tables in `ThreadLocal`.
    let env_inner = env.env_inner();
    let tx_ro = thread_local(env);
    let tables = thread_local(env);

    // Collect results using `rayon`.
    let vec = match chain {
        Chain::Main => range
            .into_par_iter()
            .map(|block_height| {
                let tx_ro = tx_ro.get_or_try(|| env_inner.tx_ro())?;
                let tables = get_tables!(env_inner, tx_ro, tables)?.as_ref();
                get_block_extended_header_from_height(&block_height, tables)
            })
            .collect::<DbResult<Vec<ExtendedBlockHeader>>>()?,
        Chain::Alt(chain_id) => {
            let ranges = {
                let tx_ro = tx_ro.get_or_try(|| env_inner.tx_ro())?;
                let tables = get_tables!(env_inner, tx_ro, tables)?.as_ref();
                let alt_chains = tables.alt_chain_infos();

                get_alt_chain_history_ranges(range, chain_id, alt_chains)?
            };

            ranges
                .par_iter()
                .rev()
                .flat_map(|(chain, range)| {
                    range.clone().into_par_iter().map(|height| {
                        let tx_ro = tx_ro.get_or_try(|| env_inner.tx_ro())?;
                        let tables = get_tables!(env_inner, tx_ro, tables)?.as_ref();

                        match *chain {
                            Chain::Main => get_block_extended_header_from_height(&height, tables),
                            Chain::Alt(chain_id) => get_alt_block_extended_header_from_height(
                                &AltBlockHeight {
                                    chain_id: chain_id.into(),
                                    height,
                                },
                                tables,
                            ),
                        }
                    })
                })
                .collect::<DbResult<Vec<_>>>()?
        }
    };

    Ok(BlockchainResponse::BlockExtendedHeaderInRange(vec))
}

/// [`BlockchainReadRequest::ChainHeight`].
#[inline]
fn chain_height<E: Env>(env: &E) -> ResponseResult {
    // Single-threaded, no `ThreadLocal` required.
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;
    let table_block_heights = env_inner.open_db_ro::<BlockHeights>(&tx_ro)?;
    let table_block_infos = env_inner.open_db_ro::<BlockInfos>(&tx_ro)?;

    let chain_height = crate::ops::blockchain::chain_height(&table_block_heights)?;
    let block_hash =
        get_block_info(&chain_height.saturating_sub(1), &table_block_infos)?.block_hash;

    Ok(BlockchainResponse::ChainHeight(chain_height, block_hash))
}

/// [`BlockchainReadRequest::GeneratedCoins`].
#[inline]
fn generated_coins<E: Env>(env: &E, height: usize) -> ResponseResult {
    // Single-threaded, no `ThreadLocal` required.
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;
    let table_block_infos = env_inner.open_db_ro::<BlockInfos>(&tx_ro)?;

    Ok(BlockchainResponse::GeneratedCoins(
        cumulative_generated_coins(&height, &table_block_infos)?,
    ))
}

/// [`BlockchainReadRequest::Outputs`].
#[inline]
fn outputs<E>(
    env: &E,
    outputs: IndexMap<Amount, IndexSet<AmountIndex>>,
    get_txid: bool,
) -> ResponseResult
where
    E: Env + 'static,
    for <'a> <E as Env>::EnvInner<'a>: Sync,
    for <'a, 'b> <<E as Env>::EnvInner<'a> as OpenTables<'a>>::Ro<'b>: Send,
{
    // Prepare tx/tables in `ThreadLocal`.
    let env_inner = env.env_inner();
    let tx_ro = thread_local(env);
    let tables = thread_local(env);

    let amount_of_outs = outputs
        .par_iter()
        .map(|(&amount, _)| {
            let tx_ro = tx_ro.get_or_try(|| env_inner.tx_ro())?;
            let tables = get_tables!(env_inner, tx_ro, tables)?.as_ref();

            if amount == 0 {
                Ok((amount, tables.rct_outputs().len()?))
            } else {
                // v1 transactions.
                match tables.num_outputs().get(&amount) {
                    Ok(count) => Ok((amount, count)),
                    // If we get a request for an `amount` that doesn't exist,
                    // we return `0` instead of an error.
                    Err(RuntimeError::KeyNotFound) => Ok((amount, 0)),
                    Err(e) => Err(e),
                }
            }
        })
        .collect::<Result<_, _>>()?;

    // The 2nd mapping function.
    // This is pulled out from the below `map()` for readability.
    let inner_map = |amount, amount_index| {
        let tx_ro = tx_ro.get_or_try(|| env_inner.tx_ro())?;
        let tables = get_tables!(env_inner, tx_ro, tables)?.as_ref();

        let id = PreRctOutputId {
            amount,
            amount_index,
        };

        let output_on_chain = match id_to_output_on_chain(&id, get_txid, tables) {
            Ok(output) => output,
            Err(RuntimeError::KeyNotFound) => return Ok(Either::Right(amount_index)),
            Err(e) => return Err(e),
        };

        Ok(Either::Left((amount_index, output_on_chain)))
    };

    // Collect results using `rayon`.
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
fn outputs_vec<E: Env>(
    env: &E,
    outputs: Vec<(Amount, AmountIndex)>,
    get_txid: bool,
) -> ResponseResult {
    Ok(BlockchainResponse::OutputsVec(todo!()))
}

/// [`BlockchainReadRequest::NumberOutputsWithAmount`].
#[inline]
fn number_outputs_with_amount<'a, E>(env: &'a E, amounts: Vec<Amount>) -> ResponseResult 
where
    E: Env,
    <E as Env>::EnvInner<'a>: Sync,
    for <'b> <<E as Env>::EnvInner<'a> as OpenTables<'a>>::Ro<'b>: Send,
{
    // Prepare tx/tables in `ThreadLocal`.
    let env_inner = env.env_inner();
    let tx_ro = thread_local(env);
    let tables = thread_local(env);

    // Cache the amount of RCT outputs once.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "INVARIANT: #[cfg] @ lib.rs asserts `usize == u64`"
    )]
    let num_rct_outputs = {
        let tx_ro = env_inner.tx_ro()?;
        let tables = env_inner.open_tables(&tx_ro)?;
        tables.rct_outputs().len()? as usize
    };

    // Collect results using `rayon`.
    let map = amounts
        .into_par_iter()
        .map(|amount| {
            let tx_ro = tx_ro.get_or_try(|| env_inner.tx_ro())?;
            let tables = get_tables!(env_inner, tx_ro, tables)?.as_ref();

            if amount == 0 {
                // v2 transactions.
                Ok((amount, num_rct_outputs))
            } else {
                // v1 transactions.
                match tables.num_outputs().get(&amount) {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "INVARIANT: #[cfg] @ lib.rs asserts `usize == u64`"
                    )]
                    Ok(count) => Ok((amount, count as usize)),
                    // If we get a request for an `amount` that doesn't exist,
                    // we return `0` instead of an error.
                    Err(RuntimeError::KeyNotFound) => Ok((amount, 0)),
                    Err(e) => Err(e),
                }
            }
        })
        .collect::<DbResult<HashMap<Amount, usize>>>()?;

    Ok(BlockchainResponse::NumberOutputsWithAmount(map))
}

/// [`BlockchainReadRequest::KeyImagesSpent`].
#[inline]
fn key_images_spent<E>(env: &E, key_images: HashSet<KeyImage>) -> ResponseResult
where
    E: Env + 'static,
    for <'a> <E as Env>::EnvInner<'a>: Sync,
    for <'a, 'b> <<E as Env>::EnvInner<'a> as OpenTables<'a>>::Ro<'b>: Send,
{
    // Prepare tx/tables in `ThreadLocal`.
    let env_inner = env.env_inner();
    let tx_ro = thread_local(env);
    let tables = thread_local(env);

    // Key image check function.
    let key_image_exists = |key_image| {
        let tx_ro = tx_ro.get_or_try(|| env_inner.tx_ro())?;
        let tables = get_tables!(env_inner, tx_ro, tables)?.as_ref();
        key_image_exists(&key_image, tables.key_images())
    };

    // FIXME:
    // Create/use `enum cuprate_types::Exist { Does, DoesNot }`
    // or similar instead of `bool` for clarity.
    // <https://github.com/Cuprate/cuprate/pull/113#discussion_r1581536526>
    //
    // Collect results using `rayon`.
    match key_images
        .into_par_iter()
        .map(key_image_exists)
        // If the result is either:
        // `Ok(true)` => a key image was found, return early
        // `Err` => an error was found, return early
        //
        // Else, `Ok(false)` will continue the iterator.
        .find_any(|result| !matches!(result, Ok(false)))
    {
        None | Some(Ok(false)) => Ok(BlockchainResponse::KeyImagesSpent(false)), // Key image was NOT found.
        Some(Ok(true)) => Ok(BlockchainResponse::KeyImagesSpent(true)), // Key image was found.
        Some(Err(e)) => Err(e), // A database error occurred.
    }
}

/// [`BlockchainReadRequest::KeyImagesSpentVec`].
fn key_images_spent_vec<E>(env: &E, key_images: Vec<KeyImage>) -> ResponseResult 
where
    E: Env + 'static,
    for <'a> <E as Env>::EnvInner<'a>: Sync,
    for <'a, 'b> <<E as Env>::EnvInner<'a> as OpenTables<'a>>::Ro<'b>: Send,
{
    // Prepare tx/tables in `ThreadLocal`.
    let env_inner = env.env_inner();
    let tx_ro = thread_local(env);
    let tables = thread_local(env);

    // Key image check function.
    let key_image_exists = |key_image| {
        let tx_ro = tx_ro.get_or_try(|| env_inner.tx_ro())?;
        let tables = get_tables!(env_inner, tx_ro, tables)?.as_ref();
        key_image_exists(&key_image, tables.key_images())
    };

    // Collect results using `rayon`.
    Ok(BlockchainResponse::KeyImagesSpentVec(
        key_images
            .into_par_iter()
            .map(key_image_exists)
            .collect::<DbResult<_>>()?,
    ))
}

/// [`BlockchainReadRequest::CompactChainHistory`]
fn compact_chain_history<E: Env>(env: &E) -> ResponseResult {
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;

    let table_block_heights = env_inner.open_db_ro::<BlockHeights>(&tx_ro)?;
    let table_block_infos = env_inner.open_db_ro::<BlockInfos>(&tx_ro)?;

    let top_block_height = top_block_height(&table_block_heights)?;

    let top_block_info = get_block_info(&top_block_height, &table_block_infos)?;
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
        .map(|height| Ok(get_block_info(&height, &table_block_infos)?.block_hash))
        .collect::<DbResult<Vec<_>>>()?;

    if compact_history_genesis_not_included::<INITIAL_BLOCKS>(top_block_height) {
        block_ids.push(get_block_info(&0, &table_block_infos)?.block_hash);
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
fn next_chain_entry<E: Env>(
    env: &E,
    block_ids: &[BlockHash],
    next_entry_size: usize,
) -> ResponseResult {
    // Single-threaded, no `ThreadLocal` required.
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;

    let tables = env_inner.open_tables(&tx_ro)?;
    let table_block_heights = tables.block_heights();
    let table_alt_block_heights = tables.alt_block_heights();
    let table_block_infos = tables.block_infos_iter();

    let idx = find_split_point(
        block_ids,
        false,
        false,
        table_block_heights,
        table_alt_block_heights,
    )?;

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
    let first_known_height = table_block_heights.get(&first_known_block_hash)?;

    let chain_height = crate::ops::blockchain::chain_height(table_block_heights)?;
    let last_height_in_chain_entry = min(first_known_height + next_entry_size, chain_height);

    let (block_ids, block_weights) = (first_known_height..last_height_in_chain_entry)
        .map(|height| {
            let block_info = table_block_infos.get(&height)?;

            Ok((block_info.block_hash, block_info.weight))
        })
        .collect::<DbResult<(Vec<_>, Vec<_>)>>()?;

    let top_block_info = table_block_infos.get(&(chain_height - 1))?;

    let first_block_blob = if block_ids.len() >= 2 {
        Some(get_block_blob_with_tx_indexes(&(first_known_height + 1), &tables)?.0)
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
fn find_first_unknown<E: Env>(env: &E, block_ids: &[BlockHash]) -> ResponseResult {
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;

    let table_block_heights = env_inner.open_db_ro::<BlockHeights>(&tx_ro)?;
    let table_alt_block_heights = env_inner.open_db_ro::<AltBlockHeights>(&tx_ro)?;

    let idx = find_split_point(
        block_ids,
        true,
        true,
        &table_block_heights,
        &table_alt_block_heights,
    )?;

    Ok(if idx == block_ids.len() {
        BlockchainResponse::FindFirstUnknown(None)
    } else if idx == 0 {
        BlockchainResponse::FindFirstUnknown(Some((0, 0)))
    } else {
        let last_known_height = get_block_height(&block_ids[idx - 1], &table_block_heights)?;

        BlockchainResponse::FindFirstUnknown(Some((idx, last_known_height + 1)))
    })
}

/// [`BlockchainReadRequest::TxsInBlock`]
fn txs_in_block<E: Env>(env: &E, block_hash: [u8; 32], missing_txs: Vec<u64>) -> ResponseResult {
    // Single-threaded, no `ThreadLocal` required.
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;
    let tables = env_inner.open_tables(&tx_ro)?;

    let block_height = tables.block_heights().get(&block_hash)?;

    let (block, miner_tx_index, numb_txs) = get_block_blob_with_tx_indexes(&block_height, &tables)?;
    let first_tx_index = miner_tx_index + 1;

    if numb_txs < missing_txs.len() {
        return Ok(BlockchainResponse::TxsInBlock(None));
    }

    let txs = missing_txs
        .into_iter()
        .map(|index_offset| Ok(tables.tx_blobs().get(&(first_tx_index + index_offset))?.0))
        .collect::<DbResult<_>>()?;

    Ok(BlockchainResponse::TxsInBlock(Some(TxsInBlock {
        block,
        txs,
    })))
}

/// [`BlockchainReadRequest::AltBlocksInChain`]
fn alt_blocks_in_chain<E>(env: &E, chain_id: ChainId) -> ResponseResult
where
    E: Env + 'static,
    for <'a> <E as Env>::EnvInner<'a>: Sync,
    for <'a, 'b> <<E as Env>::EnvInner<'a> as OpenTables<'a>>::Ro<'b>: Send,
{
    // Prepare tx/tables in `ThreadLocal`.
    let env_inner = env.env_inner();
    let tx_ro = thread_local(env);
    let tables = thread_local(env);

    // Get the history of this alt-chain.
    let history = {
        let tx_ro = tx_ro.get_or_try(|| env_inner.tx_ro())?;
        let tables = get_tables!(env_inner, tx_ro, tables)?.as_ref();
        get_alt_chain_history_ranges(0..usize::MAX, chain_id, tables.alt_chain_infos())?
    };

    // Get all the blocks until we join the main-chain.
    let blocks = history
        .par_iter()
        .rev()
        .skip(1)
        .flat_map(|(chain_id, range)| {
            let Chain::Alt(chain_id) = chain_id else {
                panic!("Should not have main chain blocks here we skipped last range");
            };

            range.clone().into_par_iter().map(|height| {
                let tx_ro = tx_ro.get_or_try(|| env_inner.tx_ro())?;
                let tables = get_tables!(env_inner, tx_ro, tables)?.as_ref();

                get_alt_block(
                    &AltBlockHeight {
                        chain_id: (*chain_id).into(),
                        height,
                    },
                    tables,
                )
            })
        })
        .collect::<DbResult<_>>()?;

    Ok(BlockchainResponse::AltBlocksInChain(blocks))
}

/// [`BlockchainReadRequest::Block`]
fn block<E: Env>(env: &E, block_height: BlockHeight) -> ResponseResult {
    // Single-threaded, no `ThreadLocal` required.
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;
    let tables = env_inner.open_tables(&tx_ro)?;

    Ok(BlockchainResponse::Block(get_block(
        &tables,
        &block_height,
    )?))
}

/// [`BlockchainReadRequest::BlockByHash`]
fn block_by_hash<E: Env>(env: &E, block_hash: BlockHash) -> ResponseResult {
    // Single-threaded, no `ThreadLocal` required.
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;
    let tables = env_inner.open_tables(&tx_ro)?;

    Ok(BlockchainResponse::Block(get_block_by_hash(
        &tables,
        &block_hash,
    )?))
}

/// [`BlockchainReadRequest::TotalTxCount`]
fn total_tx_count<E: Env>(env: &E) -> ResponseResult {
    Ok(BlockchainResponse::TotalTxCount(todo!()))
}

/// [`BlockchainReadRequest::DatabaseSize`]
fn database_size<E: Env>(env: &E) -> ResponseResult {
    Ok(BlockchainResponse::DatabaseSize {
        database_size: todo!(),
        free_space: todo!(),
    })
}

/// [`BlockchainReadRequest::OutputHistogram`]
fn output_histogram<E: Env>(env: &E, input: OutputHistogramInput) -> ResponseResult {
    Ok(BlockchainResponse::OutputHistogram(todo!()))
}

/// [`BlockchainReadRequest::CoinbaseTxSum`]
fn coinbase_tx_sum<E: Env>(env: &E, height: usize, count: u64) -> ResponseResult {
    Ok(BlockchainResponse::CoinbaseTxSum(todo!()))
}

/// [`BlockchainReadRequest::AltChains`]
fn alt_chains<E: Env>(env: &E) -> ResponseResult {
    Ok(BlockchainResponse::AltChains(todo!()))
}

/// [`BlockchainReadRequest::AltChainCount`]
fn alt_chain_count<E: Env>(env: &E) -> ResponseResult {
    Ok(BlockchainResponse::AltChainCount(todo!()))
}

/// [`BlockchainReadRequest::Transactions`]
fn transactions<E: Env>(env: &E, tx_hashes: HashSet<[u8; 32]>) -> ResponseResult {
    Ok(BlockchainResponse::Transactions {
        txs: todo!(),
        missed_txs: todo!(),
    })
}

/// [`BlockchainReadRequest::TotalRctOutputs`]
fn total_rct_outputs<E: Env>(env: &E) -> ResponseResult {
    // Single-threaded, no `ThreadLocal` required.
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;
    let len = env_inner.open_db_ro::<RctOutputs>(&tx_ro)?.len()?;

    Ok(BlockchainResponse::TotalRctOutputs(len))
}

/// [`BlockchainReadRequest::TxOutputIndexes`]
fn tx_output_indexes<E: Env>(env: &E, tx_hash: &[u8; 32]) -> ResponseResult {
    // Single-threaded, no `ThreadLocal` required.
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;
    let tx_id = env_inner.open_db_ro::<TxIds>(&tx_ro)?.get(tx_hash)?;
    let o_indexes = env_inner.open_db_ro::<TxOutputs>(&tx_ro)?.get(&tx_id)?;

    Ok(BlockchainResponse::TxOutputIndexes(o_indexes.0))
}

/// [`BlockchainReadRequest::OutputDistribution`]
fn output_distribution<E: Env>(env: &E, input: OutputDistributionInput) -> ResponseResult {
    Ok(BlockchainResponse::OutputDistribution(todo!()))
}
