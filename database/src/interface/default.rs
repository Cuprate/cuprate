#![allow(unused_variables, unused_imports)]

use monero::{Hash, BlockHeader, Block};
use monero::blockdata::transaction::KeyImage;

use crate::{encoding::Value, database::{Interface, Database, transaction::Transaction}, error::DBException, interface::{ReadInterface, WriteInterface}, table, types::{AltBlock, TransactionPruned}};

impl<'thread, D: Database<'thread>> ReadInterface<'thread> for Interface<'thread, D> {

	// --------------------------------| Blockchain |--------------------------------

    fn height(&'thread self) -> Result<u64, DBException> {
        todo!()
    }

	// ----------------------------------| Blocks |---------------------------------

    fn block_exists(&'thread self, hash: &Hash) -> Result<bool, DBException> {
		todo!()
    }

    fn get_block_hash(&'thread self, height: &u64) -> Result<Hash, DBException> {
        todo!()
    }

    fn get_block_height(&'thread self, hash: &Hash) -> Result<u64, DBException> {
        todo!()
    }

    fn get_block_weight(&'thread self, height: &u64) -> Result<u64, DBException> {
        todo!()
    }

    fn get_block_already_generated_coins(&'thread self, height: &u64) -> Result<u64, DBException> {
        todo!()
    }

    fn get_block_long_term_weight(&'thread self, height: &u64) -> Result<u64, DBException> {
        todo!()
    }

    fn get_block_timestamp(&'thread self, height: &u64) -> Result<u64, DBException> {
        todo!()
    }

    fn get_block_cumulative_rct_outputs(&'thread self, height: &u64) -> Result<u64, DBException> {
        todo!()
    }

    fn get_block(&'thread self, hash: &Hash) -> Result<Block, DBException> {
        todo!()
    }

    fn get_block_from_height(&'thread self, height: &u64) -> Result<Block, DBException> {
        todo!()
    }

    fn get_block_header(&'thread self, hash: &Hash) -> Result<BlockHeader, DBException> {
        todo!()
    }

    fn get_block_header_from_height(&'thread self, height: &u64) -> Result<BlockHeader, DBException> {
        todo!()
    }

    fn get_top_block(&'thread self) -> Result<Block, DBException> {
        todo!()
    }

    fn get_top_block_hash(&'thread self) -> Result<Hash, DBException> {
        todo!()
    }

	// ------------------------------|  Transactions  |-----------------------------

    fn get_num_tx(&'thread self) -> Result<u64, DBException> {
        todo!()
    }

    fn tx_exists(&'thread self, hash: &Hash) -> Result<bool, DBException> {
        todo!()
    }

    fn get_tx_unlock_time(&'thread self, hash: &Hash) -> Result<u64, DBException> {
        todo!()
    }

    fn get_tx(&'thread self, hash: &Hash) -> Result<monero::Transaction, DBException> {
        todo!()
    }

    fn get_tx_list(&'thread self, hash_list: &[Hash]) -> Result<Vec<monero::Transaction>, DBException> {
        todo!()
    }

    fn get_pruned_tx(&'thread self, hash: &Hash) -> Result<TransactionPruned, DBException> {
        todo!()
    }

    fn get_tx_block_height(&'thread self, hash: &Hash) -> Result<u64, DBException> {
        todo!()
    }

	// --------------------------------|  Outputs  |--------------------------------

    fn get_output(&'thread self, amount: &Option<u64>, index: &u64) -> Result<crate::types::OutputMetadata, DBException> {
        todo!()
    }

    fn get_output_list(&'thread self, amounts: &Option<Vec<u64>>, offsets: &[u64]) -> Result<Vec<crate::types::OutputMetadata>, DBException> {
        todo!()
    }

    fn get_rct_num_outputs(&'thread self) -> Result<u64, DBException> {
        todo!()
    }

    fn get_pre_rct_num_outputs(&'thread self, amount: &u64) -> Result<u64, DBException> {
        todo!()
    }

	// ------------------------------|  Spent Keys  |------------------------------

    fn is_spent_key_recorded(&'thread self, key_image: &KeyImage) -> Result<bool, DBException> {
        todo!()
    }

	// ------------------------------|  Alt-Block  |-------------------------------

    fn get_alt_block(&'thread self, altblock_hash: &Hash) -> Result<AltBlock, DBException> {
        todo!()
    }

    fn get_alt_block_count(&'thread self) -> Result<u64, DBException> {
        todo!()
    }

	// --------------------------------|  Other  |---------------------------------

    fn get_blockchain_pruning_seed(&'thread self) -> Result<u32, DBException> {
        todo!()
    }

    fn get_core_sync_data(&'thread self) -> Result<u32, DBException> {
        todo!()
    }
}