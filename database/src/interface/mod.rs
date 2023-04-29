//! This module contains all the interface implementation

use monero::{Block, Hash, BlockHeader, util::ringct::Key, TxOut, blockdata::transaction::KeyImage};
use crate::{error::DBException, types::{BlockMetadata, TransactionPruned, OutputMetadata, AltBlock}};

mod default;

pub trait ReadInterface<'thread> {

	// --------------------------------| Blockchain |--------------------------------
	
	/// `height` fetch the current blockchain height.
	fn height(&'thread self) -> Result<u64, DBException>;

	// ----------------------------------| Blocks |---------------------------------

	/// `block_exist` return true if the requested block exist
	fn block_exists(&'thread self, hash: &Hash) -> Result<bool, DBException>;

	/// `get_block_hash` fetch the requested block's hash (with its given height)
	fn get_block_hash(&'thread self, height: &u64) -> Result<Hash, DBException>;

	/// `get_block_height` fetch the requested block's height (with its given hash)
	fn get_block_height(&'thread self, hash: &Hash) -> Result<u64, DBException>;

	/// `get_block_weight` fetch the requested block's weight (aka size) (with its given height)
	fn get_block_weight(&'thread self, height: &u64) -> Result<u64, DBException>;

	/// `get_block_already_generated_coins` fetch the total amount of piconero generated at the given block's height
	fn get_block_already_generated_coins(&'thread self, height: &u64) -> Result<u64, DBException>;

	/// `get_block_long_term_weight` fetch the total amount of piconero
	fn get_block_long_term_weight(&'thread self, height: &u64) -> Result<u64, DBException>;

	/// `get_block_timestamp` fetch the requested block's height (with its given height)
	fn get_block_timestamp(&'thread self, height: &u64) -> Result<u64, DBException>;

	/// `get_block_cumulative_rct_outputs` fetch the amount of RingCT outputs at the given block's height
	fn get_block_cumulative_rct_outputs(&'thread self, height: &u64) -> Result<u64, DBException>;

	/// `get_block` fetch the requested block (with its given hash)
	fn get_block(&'thread self, hash: &Hash) -> Result<Block, DBException>;

	/// `get_block_from_height` fetch the requested block (with its given height)
	fn get_block_from_height(&'thread self, height: &u64) -> Result<Block, DBException>;

	/// `get_block_header` fetch the requested block's header (with its given hash)
	fn get_block_header(&'thread self, hash: &Hash) -> Result<BlockHeader, DBException>;

	/// `get_block_header_from_height` fetch the requested block's header (with its given height)
	fn get_block_header_from_height(&'thread self, height: &u64) -> Result<BlockHeader, DBException>;

	/// `get_top_block` fetch the top block of the blockchain
	fn get_top_block(&'thread self) -> Result<Block, DBException>;

	/// `get_top_block_hash` fetch the top block's hash
	fn get_top_block_hash(&'thread self) -> Result<Hash, DBException>;

	// ------------------------------|  Transactions  |-----------------------------

	/// `get_num_tx` fetch the total number of transaction (used to determine tx_id of new transactions)
	fn get_num_tx(&'thread self) -> Result<u64, DBException>;

	/// `tx_exists` return true if the requested tx exist (with its given hash)
	fn tx_exists(&'thread self, hash: &Hash) -> Result<bool, DBException>;

	/// `get_tx_unlock_time` fetch the requested transaction's unlock time (with its given hash)
	fn get_tx_unlock_time(&'thread self, hash: &Hash) -> Result<u64, DBException>;

	/// `get_tx` fetch requested transaction (with its given hash)
	fn get_tx(&'thread self, hash: &Hash) -> Result<monero::Transaction, DBException>;

	/// `get_tx_list` fetch the requested transactions (with there given hashes)
	fn get_tx_list(&'thread self, hash_list: &[Hash]) -> Result<Vec<monero::Transaction>, DBException>;

	/// `get_prund_tx` fetch the requested Transaction's pruned part (with its given hash)
	fn get_pruned_tx(&'thread self, hash: &Hash) -> Result<TransactionPruned, DBException>;

	/// `get_tx_block_height` fetch the transaction's block height (with its given hash)
	fn get_tx_block_height(&'thread self, hash: &Hash) -> Result<u64, DBException>;

	// --------------------------------|  Outputs  |--------------------------------

	/// `get_output` get the requested output (with its given amount and index) (a None or 0 amount means it is a RingCT output)
	fn get_output(&'thread self, amount: &Option<u64>, index: &u64) -> Result<OutputMetadata, DBException>;

	/// `get_output_list` get the requested outputs (with there given amount and index offsets) (a None vector of amount means there is no PreRingCT outputs to fetch)
	fn get_output_list(&'thread self, amounts: &Option<Vec<u64>>, offsets: &[u64]) -> Result<Vec<OutputMetadata>, DBException>;

	/// `get_rct_num_outputs` get the number of RingCT output
	fn get_rct_num_outputs(&'thread self) -> Result<u64, DBException>;

	/// `get_pre_rct_num_outputs` get the number of PreRingCT output with the given amount
	fn get_pre_rct_num_outputs(&'thread self, amount: &u64) -> Result<u64, DBException>;

	// ------------------------------|  Spent Keys  |------------------------------

	/// `is_spent_key_recorded` check if the specified key image has been spent
	fn is_spent_key_recorded(&'thread self, key_image: &KeyImage) -> Result<bool, DBException>;

	// ------------------------------|  Alt-Block  |-------------------------------

	/// `get_alt_block` get the requested alternative block and metadata (with its given hash)
	fn get_alt_block(&'thread self, altblock_hash: &Hash) -> Result<AltBlock, DBException>;

	/// `get_alt_block_count` get the number of alternative blocks stored in the database tables.
	fn get_alt_block_count(&'thread self) -> Result<u64, DBException>;

	// --------------------------------|  Other  |--------------------------------

	/// `get_blockchain_pruning_seed` get the pruning seed of the database
	fn get_blockchain_pruning_seed(&'thread self) -> Result<u32, DBException>;

	/// `get_core_sync_data` get all the data needed to initiate sync between peers
	fn get_core_sync_data(&'thread self) -> Result<u32, DBException>;
}

pub trait WriteInterface<'thread>: ReadInterface<'thread> {
	
	// ----------------------------------| Blocks |---------------------------------

	/// `add_block` add the supplied block and related transactions.
	fn add_block(&'thread self, blk: Block, txs: Vec<monero::Transaction>, block_weight: u64, long_term_block_weight: u64, cumulative_difficulty: u128, coins_generated: u64) 
	-> Result<(), DBException>;

	/// `add_block_data` actually insert block's data into database tables.
	fn add_block_data(&'thread self, blk: Block, blk_metadata: BlockMetadata) 
	-> Result<(), DBException>;
	
	/// `pop_block` fetch the top block in the database and remove it.
	fn pop_block(&'thread self) -> Result<Block, DBException>;

	// ------------------------------|  Transactions  |-----------------------------

	/// `add_transaction` add the given transaction to the database
	fn add_transaction(&'thread self, tx: monero::Transaction) -> Result<(), DBException>;

	/// `add_transaction_data` add the given transaction and related data to the database tables
	fn add_transaction_data(&'thread self, tx: monero::Transaction, tx_prunable_blob: Vec<u8>, tx_hash: Hash, tx_prunable_hash: Option<Hash>) 
	-> Result<u64, DBException>;

	/// `remove_transacton` remove the requested transaction from database
	fn remove_transaction(&'thread self, tx_hash: Hash) -> Result<(), DBException>;
	
	/// `remove_transaction_data` remove the given transaction related data from database tables
	fn remove_transaction_data(&'thread self, txprefix: monero::TransactionPrefix, tx_hash: Hash) 
	-> Result<(), DBException>;

	/// `remove_tx_outputs` remove outputs belonging to the given transaction 
	fn remove_tx_outputs(&'thread self, txprefix: monero::TransactionPrefix, tx_id: u64) -> Result<(), DBException>;

	// --------------------------------|  Outputs  |--------------------------------

	/// `add_output` add the given output and related data to the database tables
	fn add_output(&'thread self, tx_hash: Hash, output: TxOut, local_index: u64, unlock_time: u64, commitment: Option<Key>) 
	-> Result<u64, DBException>;

	/// `remove_output` remove the requested output to the database. (a None or 0 amount means it is a RingCT output);
	fn remove_output(&'thread self, amount: Option<u64>, index: u64) -> Result<(), DBException>;

	// ------------------------------|  Spent Keys  |------------------------------

	/// `add_spent_key` add the supplied key image to the spent key image record
    fn add_spent_key(&'thread self, key_image: KeyImage) -> Result<(), DBException>;

	/// `remove_spent_key` remove the specified key image from the spent key image record
    fn remove_spent_key(&'thread self, key_image: KeyImage) -> Result<(), DBException>;

	// ------------------------------|  Alt-Block  |-------------------------------

	/// `add_alt_block` add the given alternative block to the database tables
	fn add_alt_block(&'thread self, altblock_hash: Hash, data: AltBlock) -> Result<(), DBException>;

	/// `add_alt_block` remove the specified alternative block from the database tables
	fn remove_alt_block(&'thread self, altblock_hash: Hash) -> Result<(), DBException>;
	
	/// `drop_alt_blocks` forget every alternative blocks stored in the database.
	fn drop_alt_blocks(&'thread self) -> Result<(), DBException>;
}