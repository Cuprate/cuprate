//! Database reader thread-pool definitions and logic.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use rayon::{
    iter::{IntoParallelIterator, ParallelIterator},
    ThreadPool,
};
use thread_local::ThreadLocal;

use cuprate_database::{ConcreteEnv, DatabaseRo, Env, EnvInner, RuntimeError};
use cuprate_database_service::{init_thread_pool, DatabaseReadService, ReaderThreads};
use cuprate_helper::map::combine_low_high_bits_to_u128;
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    Chain, ExtendedBlockHeader, OutputOnChain,
};

use crate::{
    ops::{
        block::{
            block_exists, get_block_extended_header_from_height, get_block_height, get_block_info,
        },
        blockchain::{cumulative_generated_coins, top_block_height},
        key_image::key_image_exists,
        output::id_to_output_on_chain,
    },
    service::{
        free::{compact_history_genesis_not_included, compact_history_index_to_height_offset},
        types::{BlockchainReadHandle, ResponseResult},
    },
    tables::{BlockHeights, BlockInfos, OpenTables, Tables},
    types::{Amount, AmountIndex, BlockHash, BlockHeight, KeyImage, PreRctOutputId},
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
fn map_request(
    env: &ConcreteEnv,              // Access to the database
    request: BlockchainReadRequest, // The request we must fulfill
) -> ResponseResult {
    use BlockchainReadRequest as R;

    /* SOMEDAY: pre-request handling, run some code for each request? */

    match request {
        R::BlockExtendedHeader(block) => block_extended_header(env, block),
        R::BlockHash(block, chain) => block_hash(env, block, chain),
        R::FindBlock(_) => todo!("Add alt blocks to DB"),
        R::FilterUnknownHashes(hashes) => filter_unknown_hashes(env, hashes),
        R::BlockExtendedHeaderInRange(range, chain) => {
            block_extended_header_in_range(env, range, chain)
        }
        R::ChainHeight => chain_height(env),
        R::GeneratedCoins(height) => generated_coins(env, height),
        R::Outputs(map) => outputs(env, map),
        R::NumberOutputsWithAmount(vec) => number_outputs_with_amount(env, vec),
        R::KeyImagesSpent(set) => key_images_spent(env, set),
        R::CompactChainHistory => compact_chain_history(env),
        R::FindFirstUnknown(block_ids) => find_first_unknown(env, &block_ids),
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
            #[allow(clippy::significant_drop_in_scrutinee)]
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

/// [`BlockchainReadRequest::BlockExtendedHeader`].
#[inline]
fn block_extended_header(env: &ConcreteEnv, block_height: BlockHeight) -> ResponseResult {
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
fn block_hash(env: &ConcreteEnv, block_height: BlockHeight, chain: Chain) -> ResponseResult {
    // Single-threaded, no `ThreadLocal` required.
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;
    let table_block_infos = env_inner.open_db_ro::<BlockInfos>(&tx_ro)?;

    let block_hash = match chain {
        Chain::Main => get_block_info(&block_height, &table_block_infos)?.block_hash,
        Chain::Alt(_) => todo!("Add alt blocks to DB"),
    };

    Ok(BlockchainResponse::BlockHash(block_hash))
}

/// [`BlockchainReadRequest::FilterUnknownHashes`].
#[inline]
fn filter_unknown_hashes(env: &ConcreteEnv, mut hashes: HashSet<BlockHash>) -> ResponseResult {
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
fn block_extended_header_in_range(
    env: &ConcreteEnv,
    range: std::ops::Range<BlockHeight>,
    chain: Chain,
) -> ResponseResult {
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
            .collect::<Result<Vec<ExtendedBlockHeader>, RuntimeError>>()?,
        Chain::Alt(_) => todo!("Add alt blocks to DB"),
    };

    Ok(BlockchainResponse::BlockExtendedHeaderInRange(vec))
}

/// [`BlockchainReadRequest::ChainHeight`].
#[inline]
fn chain_height(env: &ConcreteEnv) -> ResponseResult {
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
fn generated_coins(env: &ConcreteEnv, height: u64) -> ResponseResult {
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
fn outputs(env: &ConcreteEnv, outputs: HashMap<Amount, HashSet<AmountIndex>>) -> ResponseResult {
    // Prepare tx/tables in `ThreadLocal`.
    let env_inner = env.env_inner();
    let tx_ro = thread_local(env);
    let tables = thread_local(env);

    // The 2nd mapping function.
    // This is pulled out from the below `map()` for readability.
    let inner_map = |amount, amount_index| -> Result<(AmountIndex, OutputOnChain), RuntimeError> {
        let tx_ro = tx_ro.get_or_try(|| env_inner.tx_ro())?;
        let tables = get_tables!(env_inner, tx_ro, tables)?.as_ref();

        let id = PreRctOutputId {
            amount,
            amount_index,
        };

        let output_on_chain = id_to_output_on_chain(&id, tables)?;

        Ok((amount_index, output_on_chain))
    };

    // Collect results using `rayon`.
    let map = outputs
        .into_par_iter()
        .map(|(amount, amount_index_set)| {
            Ok((
                amount,
                amount_index_set
                    .into_par_iter()
                    .map(|amount_index| inner_map(amount, amount_index))
                    .collect::<Result<HashMap<AmountIndex, OutputOnChain>, RuntimeError>>()?,
            ))
        })
        .collect::<Result<HashMap<Amount, HashMap<AmountIndex, OutputOnChain>>, RuntimeError>>()?;

    Ok(BlockchainResponse::Outputs(map))
}

/// [`BlockchainReadRequest::NumberOutputsWithAmount`].
#[inline]
fn number_outputs_with_amount(env: &ConcreteEnv, amounts: Vec<Amount>) -> ResponseResult {
    // Prepare tx/tables in `ThreadLocal`.
    let env_inner = env.env_inner();
    let tx_ro = thread_local(env);
    let tables = thread_local(env);

    // Cache the amount of RCT outputs once.
    // INVARIANT: #[cfg] @ lib.rs asserts `usize == u64`
    #[allow(clippy::cast_possible_truncation)]
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
                    // INVARIANT: #[cfg] @ lib.rs asserts `usize == u64`
                    #[allow(clippy::cast_possible_truncation)]
                    Ok(count) => Ok((amount, count as usize)),
                    // If we get a request for an `amount` that doesn't exist,
                    // we return `0` instead of an error.
                    Err(RuntimeError::KeyNotFound) => Ok((amount, 0)),
                    Err(e) => Err(e),
                }
            }
        })
        .collect::<Result<HashMap<Amount, usize>, RuntimeError>>()?;

    Ok(BlockchainResponse::NumberOutputsWithAmount(map))
}

/// [`BlockchainReadRequest::KeyImagesSpent`].
#[inline]
fn key_images_spent(env: &ConcreteEnv, key_images: HashSet<KeyImage>) -> ResponseResult {
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

/// [`BlockchainReadRequest::CompactChainHistory`]
fn compact_chain_history(env: &ConcreteEnv) -> ResponseResult {
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
    const INITIAL_BLOCKS: u64 = 11;

    // rayon is not used here because the amount of block IDs is expected to be small.
    let mut block_ids = (0..)
        .map(compact_history_index_to_height_offset::<INITIAL_BLOCKS>)
        .map_while(|i| top_block_height.checked_sub(i))
        .map(|height| Ok(get_block_info(&height, &table_block_infos)?.block_hash))
        .collect::<Result<Vec<_>, RuntimeError>>()?;

    if compact_history_genesis_not_included::<INITIAL_BLOCKS>(top_block_height) {
        block_ids.push(get_block_info(&0, &table_block_infos)?.block_hash);
    }

    Ok(BlockchainResponse::CompactChainHistory {
        cumulative_difficulty,
        block_ids,
    })
}

/// [`BlockchainReadRequest::FindFirstUnknown`]
///
/// # Invariant
/// `block_ids` must be sorted in chronological block order, or else
/// the returned result is unspecified and meaningless, as this function
/// performs a binary search.
fn find_first_unknown(env: &ConcreteEnv, block_ids: &[BlockHash]) -> ResponseResult {
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro()?;

    let table_block_heights = env_inner.open_db_ro::<BlockHeights>(&tx_ro)?;

    let mut err = None;

    // Do a binary search to find the first unknown block in the batch.
    let idx =
        block_ids.partition_point(
            |block_id| match block_exists(block_id, &table_block_heights) {
                Ok(exists) => exists,
                Err(e) => {
                    err.get_or_insert(e);
                    // if this happens the search is scrapped, just return `false` back.
                    false
                }
            },
        );

    if let Some(e) = err {
        return Err(e);
    }

    Ok(if idx == block_ids.len() {
        BlockchainResponse::FindFirstUnknown(None)
    } else if idx == 0 {
        BlockchainResponse::FindFirstUnknown(Some((0, 0)))
    } else {
        let last_known_height = get_block_height(&block_ids[idx - 1], &table_block_heights)?;

        BlockchainResponse::FindFirstUnknown(Some((idx, last_known_height + 1)))
    })
}
