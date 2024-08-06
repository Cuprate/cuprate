//! General free functions used (related to `cuprate_blockchain::service`).

//---------------------------------------------------------------------------------------------------- Import
use std::sync::Arc;

use cuprate_database::{ConcreteEnv, InitError};

use crate::service::{init_read_service, init_write_service};
use crate::{
    config::Config,
    service::types::{BlockchainReadHandle, BlockchainWriteHandle},
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
    let readers = init_read_service(db.clone(), reader_threads);
    let writer = init_write_service(db.clone());

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

//---------------------------------------------------------------------------------------------------- Tests

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn compact_history(top_height in 0_usize..500_000_000) {
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
