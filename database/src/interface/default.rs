#![allow(unused_variables, unused_imports)]

use monero::{Hash, BlockHeader, Block};
use monero::blockdata::transaction::KeyImage;

use crate::database::transaction::DupCursor;
use crate::{encoding::Value, database::{Interface, Database, transaction::Transaction}, error::DBException, interface::{ReadInterface, WriteInterface}, table, types::{AltBlock, TransactionPruned}};

impl<'thread, D: Database<'thread>> ReadInterface<'thread> for Interface<'thread, D> {

	// --------------------------------| Blockchain |--------------------------------

    fn height(&'thread self) -> Result<u64, DBException> {
        let ro_tx = self.db.tx().map_err(Into::into)?;
		
		ro_tx.num_entries::<table::blockhash>().map(|n| n as u64)
    }

	// ----------------------------------| Blocks |---------------------------------

    fn block_exists(&'thread self, hash: &Hash) -> Result<bool, DBException> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		let mut cursor_blockhash = ro_tx.cursor_dup::<table::blockhash>()?;

        Ok(cursor_blockhash.get_dup::<true>(&(), hash)?.is_some())
    }

    fn get_block_hash(&'thread self, height: &u64) -> Result<Hash, DBException> {
        let ro_tx = self.db.tx().map_err(Into::into)?;
		let mut cursor_blockmetadata = ro_tx.cursor_dup::<table::blockmetadata>()?;
        
		let metadata = cursor_blockmetadata
            .get_dup::<false>(&(), height)?
            .ok_or(DBException::NotFound(format!("Failed to find block's metadata at height : {}", height)))?;

        Ok(metadata.as_type().block_hash)
    }

    fn get_block_height(&'thread self, hash: &Hash) -> Result<u64, DBException> {
		let ro_tx = self.db.tx().map_err(Into::into)?;        
		let mut cursor_blockhash = ro_tx.cursor_dup::<table::blockhash>()?;

        cursor_blockhash
            .get_dup::<false>(&(), hash)?
            .ok_or(DBException::NotFound(format!("Failed to find height of block: {}", hash)))
			.map(|res| *res.as_type())
    }

    fn get_block_weight(&'thread self, height: &u64) -> Result<u64, DBException> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
        let mut cursor_blockmetadata = ro_tx.cursor_dup::<table::blockmetadata>()?;
        
		let metadata = cursor_blockmetadata
            .get_dup::<false>(&(), height)?
            .ok_or(DBException::NotFound(format!("Failed to find block's metadata at height : {}", height)))?;

        Ok(metadata.as_type().weight)
    }

    fn get_block_already_generated_coins(&'thread self, height: &u64) -> Result<u64, DBException> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
        let mut cursor_blockmetadata = ro_tx.cursor_dup::<table::blockmetadata>()?;
        
		let metadata = cursor_blockmetadata
            .get_dup::<false>(&(), height)?
            .ok_or(DBException::NotFound(format!("Failed to find block's metadata at height : {}", height)))?;

        Ok(metadata.as_type().total_coins_generated)
    }

    fn get_block_long_term_weight(&'thread self, height: &u64) -> Result<u64, DBException> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
        let mut cursor_blockmetadata = ro_tx.cursor_dup::<table::blockmetadata>()?;
        
		let metadata = cursor_blockmetadata
            .get_dup::<false>(&(), height)?
            .ok_or(DBException::NotFound(format!("Failed to find block's metadata at height : {}", height)))?;

        Ok(metadata.as_type().long_term_block_weight)
    }

    fn get_block_timestamp(&'thread self, height: &u64) -> Result<u64, DBException> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
        let mut cursor_blockmetadata = ro_tx.cursor_dup::<table::blockmetadata>()?;
        
		let metadata = cursor_blockmetadata
            .get_dup::<false>(&(), height)?
            .ok_or(DBException::NotFound(format!("Failed to find block's metadata at height : {}", height)))?;

        Ok(metadata.as_type().timestamp)
    }

    fn get_block_cumulative_rct_outputs(&'thread self, height: &u64) -> Result<u64, DBException> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
        let mut cursor_blockmetadata = ro_tx.cursor_dup::<table::blockmetadata>()?;
        
		let metadata = cursor_blockmetadata
            .get_dup::<false>(&(), height)?
            .ok_or(DBException::NotFound(format!("Failed to find block's metadata at height : {}", height)))?;

        Ok(metadata.as_type().cum_rct)
    }

	fn get_block_cumulative_difficulty(&'thread self, height: &u64) -> Result<u128, DBException> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		let mut cursor_blockmetadata = ro_tx.cursor_dup::<table::blockmetadata>()?;

		let metadata = cursor_blockmetadata
            .get_dup::<false>(&(), height)?
            .ok_or(DBException::NotFound(format!("Failed to find block's metadata at height : {}", height)))?;

        Ok(metadata.as_type().cumulative_difficulty)
	}

	fn get_block_difficulty(&'thread self, height: &u64) -> Result<u128, DBException> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		let mut cursor_blockmetadata = ro_tx.cursor_dup::<table::blockmetadata>()?;

		let diff1 = cursor_blockmetadata
            .get_dup::<false>(&(), height)?
            .ok_or(DBException::NotFound(format!("Failed to find block's metadata at height : {}", height)))?
			.as_type()
			.cumulative_difficulty;

		let diff2 = cursor_blockmetadata
			.get_dup::<false>(&(), &(height-1))?
			.ok_or(DBException::NotFound(format!("Failed to find block's metadata at height : {}", height)))?
			.as_type()
			.cumulative_difficulty;
        Ok(diff1-diff2)
	}

    fn get_block<const B: bool>(&'thread self, hash: &Hash) -> Result<Block, DBException> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
        todo!()
    }

    fn get_block_from_height<const B: bool>(&'thread self, height: &u64) -> Result<Block, DBException> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
        todo!()
    }

    fn get_block_header(&'thread self, hash: &Hash) -> Result<BlockHeader, DBException> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
        todo!()
    }

    fn get_block_header_from_height(&'thread self, height: &u64) -> Result<BlockHeader, DBException> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
        todo!()
    }

    fn get_top_block(&'thread self) -> Result<Block, DBException> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
        todo!()
    }

    fn get_top_block_hash(&'thread self) -> Result<Hash, DBException> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
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

    fn get_tx<const B: bool>(&'thread self, hash: &Hash) -> Result<monero::Transaction, DBException> {
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