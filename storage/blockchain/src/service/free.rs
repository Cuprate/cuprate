//! General free functions used (related to `cuprate_blockchain::service`).

//---------------------------------------------------------------------------------------------------- Import
use std::sync::Arc;

use rayon::ThreadPool;

use cuprate_database::{ConcreteEnv, InitError};
use cuprate_types::{AltBlockInformation, VerifiedBlockInformation};

use crate::{
    config::Config,
    service::{
        init_read_service, init_read_service_with_pool, init_write_service,
        types::{BlockchainReadHandle, BlockchainWriteHandle},
    },
};

//---------------------------------------------------------------------------------------------------- Init
#[cold]
#[inline(never)] // Only called once (?)
/// Initialize a database & thread-pool, and return a read/write handle to it.
///
/// Once the returned handles are [`Drop::drop`]ed, the reader
/// thread-pool and writer thread will exit automatically.
///
/// # Errors
/// This will forward the error if [`crate::open`] failed.
pub fn init(
    config: Config,
) -> Result<
    (
        BlockchainReadHandle,
        BlockchainWriteHandle,
        Arc<ConcreteEnv>,
    ),
    InitError,
> {
    let reader_threads = config.reader_threads;

    // Initialize the database itself.
    let db = Arc::new(crate::open(config)?);

    // Spawn the Reader thread pool and Writer.
    let readers = init_read_service(Arc::clone(&db), reader_threads);
    let writer = init_write_service(Arc::clone(&db));

    Ok((readers, writer, db))
}

#[cold]
#[inline(never)] // Only called once (?)
/// Initialize a database, and return a read/write handle to it.
///
/// Unlike [`init`] this will not create a thread-pool, instead using
/// the one passed in.
///
/// Once the returned handles are [`Drop::drop`]ed, the reader
/// thread-pool and writer thread will exit automatically.
///
/// # Errors
/// This will forward the error if [`crate::open`] failed.
pub fn init_with_pool(
    config: Config,
    pool: Arc<ThreadPool>,
) -> Result<
    (
        BlockchainReadHandle,
        BlockchainWriteHandle,
        Arc<ConcreteEnv>,
    ),
    InitError,
> {
    // Initialize the database itself.
    let db = Arc::new(crate::open(config)?);

    // Spawn the Reader thread pool and Writer.
    let readers = init_read_service_with_pool(Arc::clone(&db), pool);
    let writer = init_write_service(Arc::clone(&db));

    Ok((readers, writer, db))
}

//---------------------------------------------------------------------------------------------------- Compact history
/// Given a position in the compact history, returns the height offset that should be in that position.
///
/// The height offset is the difference between the top block's height and the block height that should be in that position.
#[inline]
pub(super) const fn compact_history_index_to_height_offset<const INITIAL_BLOCKS: usize>(
    i: usize,
) -> usize {
    // If the position is below the initial blocks just return the position back
    if i <= INITIAL_BLOCKS {
        i
    } else {
        // Otherwise we go with power of 2 offsets, the same as monerod.
        // So (INITIAL_BLOCKS + 2), (INITIAL_BLOCKS + 2 + 4), (INITIAL_BLOCKS + 2 + 4 + 8)
        // ref: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_core/blockchain.cpp#L727>
        INITIAL_BLOCKS + (2 << (i - INITIAL_BLOCKS)) - 2
    }
}

/// Returns if the genesis block was _NOT_ included when calculating the height offsets.
///
/// The genesis must always be included in the compact history.
#[inline]
pub(super) const fn compact_history_genesis_not_included<const INITIAL_BLOCKS: usize>(
    top_block_height: usize,
) -> bool {
    // If the top block height is less than the initial blocks then it will always be included.
    // Otherwise, we use the fact that to reach the genesis block this statement must be true (for a
    // single `i`):
    //
    // `top_block_height - INITIAL_BLOCKS - 2^i + 2 == 0`
    // which then means:
    // `top_block_height - INITIAL_BLOCKS + 2 == 2^i`
    // So if `top_block_height - INITIAL_BLOCKS + 2` is a power of 2 then the genesis block is in
    // the compact history already.
    top_block_height > INITIAL_BLOCKS && !(top_block_height - INITIAL_BLOCKS + 2).is_power_of_two()
}

//---------------------------------------------------------------------------------------------------- Map Block
/// Maps [`AltBlockInformation`] to [`VerifiedBlockInformation`]
///
/// # Panics
/// This will panic if the block is invalid, so should only be used on blocks that have been popped from
/// the main-chain.
pub(super) fn map_valid_alt_block_to_verified_block(
    alt_block: AltBlockInformation,
) -> VerifiedBlockInformation {
    let total_fees = alt_block.txs.iter().map(|tx| tx.fee).sum::<u64>();
    let total_miner_output = alt_block
        .block
        .miner_transaction
        .prefix()
        .outputs
        .iter()
        .map(|out| out.amount.unwrap_or(0))
        .sum::<u64>();

    VerifiedBlockInformation {
        block: alt_block.block,
        block_blob: alt_block.block_blob,
        txs: alt_block
            .txs
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<_, _>>()
            .unwrap(),
        block_hash: alt_block.block_hash,
        pow_hash: alt_block.pow_hash,
        height: alt_block.height,
        generated_coins: total_miner_output - total_fees,
        weight: alt_block.weight,
        long_term_weight: alt_block.long_term_weight,
        cumulative_difficulty: alt_block.cumulative_difficulty,
    }
}

//---------------------------------------------------------------------------------------------------- Tests

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use cuprate_constants::block::MAX_BLOCK_HEIGHT_USIZE;

    use super::*;

    proptest! {
        #[test]
        fn compact_history(top_height in 0..MAX_BLOCK_HEIGHT_USIZE) {
            let mut heights = (0..)
                .map(compact_history_index_to_height_offset::<11>)
                .map_while(|i| top_height.checked_sub(i))
                .collect::<Vec<_>>();

            if compact_history_genesis_not_included::<11>(top_height) {
                heights.push(0);
            }

            // Make sure the genesis and top block are always included.
            assert_eq!(*heights.last().unwrap(), 0);
            assert_eq!(*heights.first().unwrap(), top_height);

            heights.windows(2).for_each(|window| assert_ne!(window[0], window[1]));
        }
    }
}
