//! Blockchain.

//---------------------------------------------------------------------------------------------------- Import
use monero_serai::transaction::Timelock;

use cuprate_types::VerifiedBlockInformation;

use crate::{
    database::{DatabaseRo, DatabaseRw},
    env::EnvInner,
    error::RuntimeError,
    ops::macros::doc_error,
    tables::{
        BlockBlobs, BlockHeights, BlockInfos, KeyImages, NumOutputs, Outputs, PrunableHashes,
        PrunableTxBlobs, PrunedTxBlobs, RctOutputs, Tables, TablesMut, TxHeights, TxIds,
        TxUnlockTime,
    },
    transaction::{TxRo, TxRw},
    types::{BlockHash, BlockHeight, BlockInfo, KeyImage, Output, PreRctOutputId, RctOutput},
};

//---------------------------------------------------------------------------------------------------- Free Functions
/// Retrieve the height of the chain.
///
/// This returns the chain-tip, not the [`top_block_height`].
///
/// For example:
/// - The blockchain has 0 blocks => this returns `0`
/// - The blockchain has 1 block (height 0) => this returns `1`
/// - The blockchain has 2 blocks (height 1) => this returns `2`
///
/// So the height of a new block would be `chain_height()`.
#[doc = doc_error!()]
#[inline]
pub fn chain_height(
    table_block_heights: &impl DatabaseRo<BlockHeights>,
) -> Result<BlockHeight, RuntimeError> {
    table_block_heights.len()
}

/// Retrieve the height of the top block.
///
/// This returns the height of the top block, not the [`chain_height`].
///
/// For example:
/// - The blockchain has 0 blocks => this returns `Err(RuntimeError::KeyNotFound)`
/// - The blockchain has 1 block (height 0) => this returns `Ok(0)`
/// - The blockchain has 2 blocks (height 1) => this returns `Ok(1)`
///
/// Note that in cases where no blocks have been written to the
/// database yet, an error is returned: `Err(RuntimeError::KeyNotFound)`.
///
#[doc = doc_error!()]
#[inline]
pub fn top_block_height(
    table_block_heights: &impl DatabaseRo<BlockHeights>,
) -> Result<BlockHeight, RuntimeError> {
    match table_block_heights.len()? {
        0 => Err(RuntimeError::KeyNotFound),
        height => Ok(height - 1),
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
#[allow(clippy::significant_drop_tightening)]
mod test {
    use hex_literal::hex;
    use pretty_assertions::assert_eq;

    use cuprate_test_utils::data::{block_v16_tx0, block_v1_tx513, block_v9_tx3, tx_v2_rct3};

    use super::*;
    use crate::{
        ops::{
            block::add_block,
            tx::{get_tx, tx_exists},
        },
        tests::{assert_all_tables_are_empty, dummy_verified_block_information, tmp_concrete_env},
        Env,
    };

    /// Tests all above functions.
    ///
    /// Note that this doesn't test the correctness of values added, as the
    /// functions have a pre-condition that the caller handles this.
    ///
    /// It simply tests if the proper tables are mutated, and if the data
    /// stored and retrieved is the same.
    #[test]
    #[allow(clippy::cognitive_complexity, clippy::cast_possible_truncation)]
    fn all_blockchain_functions() {
        let (env, tmp) = tmp_concrete_env();
        let env_inner = env.env_inner();
        assert_all_tables_are_empty(&env);

        let blocks: Vec<VerifiedBlockInformation> = vec![dummy_verified_block_information(); 3];
        let blocks_len = u64::try_from(blocks.len()).unwrap();

        // Add blocks.
        {
            let tx_rw = env_inner.tx_rw().unwrap();
            let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();

            assert!(matches!(
                top_block_height(tables.block_heights()),
                Err(RuntimeError::KeyNotFound),
            ));

            for (i, mut block) in blocks.into_iter().enumerate() {
                let i = u64::try_from(i).unwrap();
                block.height = i;
                block.block_hash[0] = i as u8; // add_block() doesn't error for same block hash
                add_block(&block, &mut tables).unwrap();
            }

            // Assert reads are correct.
            assert_eq!(blocks_len, chain_height(tables.block_heights()).unwrap());
            assert_eq!(
                blocks_len - 1,
                top_block_height(tables.block_heights()).unwrap()
            );

            drop(tables);
            TxRw::commit(tx_rw).unwrap();
        }
    }
}
