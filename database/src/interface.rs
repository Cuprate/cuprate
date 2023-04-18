//! ### Interface module
//! This module contains all the implementations of interface. 
//! These are all the functions that can be executed through DatabaseRequest.

// TODO: add_transaction() not finished due to ringct zeroCommit missing function
// TODO: in add_transaction_data() Investigate unprunable_size == 0 condition of monerod
// TODO: Do we need correct_block_cumulative_difficulties()
// TODO: remove_tx_outputs() can be done otherwise since we don't use global output index
// TODO: Check all documentations

use monero::{cryptonote::hash::Hashable, Hash, Block, TxOut, util::ringct::Key, TxIn, BlockHeader};
use crate::{error::DB_FAILURES, database::{Database, Interface}, table::{self}, transaction::{Transaction, Cursor, WriteTransaction, DupCursor, DupWriteCursor, self, WriteCursor}, BINCODE_CONFIG, types::{TransactionPruned, OutputMetadata, KeyImage, get_transaction_prunable_blob, TxIndex, TxOutputIdx, AltBlock, BlockMetadata}};

// Implementation of Interface
impl<'service, D: Database<'service>> Interface<'service,D> {













	// --------------------------------| Blockchain |--------------------------------
	
	/// `height` fetch the current blockchain height.
    ///
    /// Return the current blockchain height. In case of failures, a DB_FAILURES will be return.
    ///
    /// No parameters is required.
	fn height(&'service self) -> Result<u64,DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		ro_tx.num_entries::<table::blockhash>().map(|n| n as u64)
	}

	/// `update_hard_fork_version` update which hardfork version a height is on.<br>
    ///
    /// In case of failures, a `DB_FAILURES` will be return.
    ///
    /// Parameters:<br>
 	/// `height`: is the height where the hard fork happen.<br>
    /// `version`: is the version of the hard fork.
    fn update_hard_fork_version(&'service self, height: u64, hardfork_version: u8) -> Result<(), DB_FAILURES> {
		let b_hf = self.get::<table::blockhfversion>(&height)?
			.ok_or(DB_FAILURES::NotFound("Can't find block hf version"))?;

		if b_hf != hardfork_version {
			self.put::<table::blockhfversion>(&height, &hardfork_version)?;
		}
		Ok(())
	}

	/// `get_hard_fork_version` checks which hardfork version a height is on.
    ///
    /// In case of failures, a `DB_FAILURES` will be return.
    ///
    /// Parameters:<br>
    /// `height`: is the height to check.
    fn get_hard_fork_version(&'service self, height: u64) -> Result<u8, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;

		let hf_version = ro_tx.get::<table::blockhfversion>(&height)?
			.ok_or(DB_FAILURES::NotFound("Can't find block hf version"))?;

		Ok(hf_version)
	}


















	// ----------------------------------| Blocks |---------------------------------

	/// `add_block` add the block and metadata to the db.
    ///
    /// In case of failures, a `DB_FAILURES` will be return. Precisely, a BLOCK_EXISTS error will be returned if
    /// the block to be added already exist. a BLOCK_INVALID will be returned if the block to be added did not pass validation.
    ///
    /// Parameters:
    /// `blk`: is the block to be added
    /// `block_weight`: is the weight of the block (data's total)
    /// `long_term_block_weight`: is the long term weight of the block (data's total)
    /// `cumulative_difficulty`: is the accumulated difficulty at this block.
    /// `coins_generated` is the number of coins generated after this block.
    /// `blk_hash`: is the hash of the block.
    fn add_block(&'service self, blk: Block, txs: Vec<monero::Transaction>, block_weight: u64, long_term_block_weight: u64, cumulative_difficulty: u128, coins_generated: u64) 
	-> Result<(), DB_FAILURES> 
	{

		// *sanity*
		if blk.tx_hashes.len() != txs.len() {
			return Err(DB_FAILURES::Other("sanity : Inconsistent tx/hashed sizes"));
		}

		let blk_blob = monero::consensus::serialize(&blk);
		let blk_hash = Hash::new(blk_blob);

		// let parent_height = self.height()?;

		let mut num_rct_outs = 0u64;
		self.add_transaction(blk.miner_tx.clone())?;
		
		if blk.miner_tx.prefix.version.0 == 2 {
			num_rct_outs += blk.miner_tx.prefix.outputs.len() as u64;
		}

		// let mut tx_hash = Hash::null();
		for tx in txs.into_iter()/*.zip(0usize..)*/ {
			// tx_hash = blk.tx_hashes[tx.1];
			for out in tx.prefix.outputs.iter() {
				if out.amount.0 == 0 {
					num_rct_outs += 1;
				}
			}
			self.add_transaction(tx/*.0*/)?;
		}

		let blk_metadata = BlockMetadata {
    		timestamp: blk.header.timestamp.0,
    		total_coins_generated: coins_generated,
    		weight: block_weight,
    		cumulative_difficulty,
    		block_hash: blk_hash.into(),
    		cum_rct: num_rct_outs,
    		long_term_block_weight,
		};

		self.add_block_data(blk, blk_metadata)
	}
	/// `add_block` add the block and metadata to the db.
    ///
    /// In case of failures, a `DB_FAILURES` will be return. Precisely, a BLOCK_EXISTS error will be returned if
    /// the block to be added already exist. a BLOCK_INVALID will be returned if the block to be added did not pass validation.
    ///
    /// Parameters:
    /// `blk`: is the block to be added
    /// `block_weight`: is the weight of the block (data's total)
    /// `long_term_block_weight`: is the long term weight of the block (data's total)
    /// `cumulative_difficulty`: is the accumulated difficulty at this block.
    /// `coins_generated` is the number of coins generated after this block.
    /// `blk_hash`: is the hash of the block.
    fn add_block_data(&'service self, blk: Block, mut blk_metadata: BlockMetadata) 
	-> Result<(), DB_FAILURES>
	{
		let height = self.height()?;

		if self.get::<table::blockmetadata>(&height)?.is_some() {
			return Err(DB_FAILURES::AlreadyExist("Attempting to insert a block alreayd existent in the database"))?
		}

		if height > 0 {
			let parent_height = self.get::<table::blockhash>(&blk.header.prev_id.into())?
				.ok_or(DB_FAILURES::NotFound("Can't find parent block"))?;
			
			if parent_height != height-1 {
				return Err(DB_FAILURES::Other("Top lock is not a new block's parent"))
			}
		}

		if blk.header.major_version.0 > 3 {
			let last_height = height-1;

			let parent_cum_rct = self.get_block_cumulative_rct_outputs(last_height)?;
			blk_metadata.cum_rct += parent_cum_rct;
		}
		self.put::<table::blocks>(&height, &blk.into())?;
		self.put::<table::blockmetadata>(&height, &blk_metadata)
	}

	/// `pop_block` pops the top block off the blockchain.
    ///
    /// Return the block that was popped. In case of failures, a `DB_FAILURES` will be return.
    ///
    /// No parameters is required.
    fn pop_block(&'service self) -> Result<Block, DB_FAILURES> {

		// First we delete block from table
		let height = self.height()?;
		if height == 0 {
			return Err(DB_FAILURES::Other("Attempting to remove block from an empty blockchain"))
		}

		let blk = self.get::<table::blocks>(&(height-1))?
			.ok_or(DB_FAILURES::NotFound("Attempting to remove block that's not in the db"))?.0;

		let hash = self.get::<table::blockmetadata>(&height)?
			.ok_or(DB_FAILURES::NotFound("Failed to retrieve block metadata"))?.block_hash;

		self.delete::<table::blockhash>(&hash, &None)?;
		self.delete::<table::blocks>(&height, &None)?;
		self.delete::<table::blockmetadata>(&height, &None)?;
		self.delete::<table::blockhfversion>(&height, &None)?;
		
		// Then we delete all its revelent txs
		for tx_hash in blk.tx_hashes.iter() {
			// 1 more condition in monerod TODO:
			self.remove_transaction(*tx_hash)?;
		}
		self.remove_transaction(blk.miner_tx.hash())?;
		Ok(blk)
	}

	/// `blocks_exists` check if the given block exists
    ///
    /// Return `true` if the block exist, `false` otherwise. In case of failures, a `DB_FAILURES` will be return.
    ///
    /// Parameters:
    /// `h`: is the given hash of the requested block.
    fn block_exists(&'service self, hash: Hash) -> Result<bool, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		Ok(ro_tx.get::<table::blockhash>(&hash.into())?.is_some())
	}

	/// `get_block_hash` fetch the block's hash located at the give height.
    ///
    /// Return the hash of the last block. In case of failures, a DB_FAILURES will be return.
    ///
    /// No parameters is required
    fn get_block_hash(&'service self, height: u64) -> Result<Hash, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		let metadata = ro_tx.get::<table::blockmetadata>(&height)?
			.ok_or(DB_FAILURES::NotFound("Failed to find block's metadata"))?;

		Ok(metadata.block_hash.0)
	}

	/// `get_block_height` gets the height of the block with a given hash
    ///
    /// Return the requested height.
    fn get_block_height(&'service self, hash: Hash) -> Result<u64, DB_FAILURES> {
		let ro_tx= self.db.tx().map_err(Into::into)?;
		ro_tx.get::<table::blockhash>(&hash.into())?
			.ok_or(DB_FAILURES::NotFound("Failed to find block height"))
	}

	/// `get_block_weights` fetch the block's weight located at the given height.
    ///
    /// Return the requested block weight. In case of failures, a `DB_FAILURES` will be return. Precisely, a `BLOCK_DNE`
    /// error will be returned if the requested block can't be found.
    ///
    /// Parameters:
    /// `height`: is the given height where the requested block is located.
    fn get_block_weight(&'service self, height: u64) -> Result<u64, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		let metadata = ro_tx.get::<table::blockmetadata>(&height)?
			.ok_or(DB_FAILURES::NotFound("Failed to find block's metadata"))?;

		Ok(metadata.weight)
	}

	/// `get_block_already_generated_coins` fetch a block's already generated coins
    ///
    /// Return the total coins generated as of the block with the given height. In case of failures, a `DB_FAILURES` will be return. Precisely, a `BLOCK_DNE`
    /// error will be returned if the requested block can't be found.
    ///
    /// Parameters:
    /// `height`: is the given height of the block to seek.
    fn get_block_already_generated_coins(&'service self, height: u64) -> Result<u64, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		let metadata = ro_tx.get::<table::blockmetadata>(&height)?
			.ok_or(DB_FAILURES::NotFound("Failed to find block's metadata"))?;

		Ok(metadata.total_coins_generated)
	}

    /// `get_block_long_term_weight` fetch a block's long term weight.
    ///
    /// Should return block's long term weight. In case of failures, a DB_FAILURES will be return. Precisely, a `BLOCK_DNE`
    /// error will be returned if the requested block can't be found.
    ///
    /// Parameters:
    /// `height`: is the given height where the requested block is located.
    fn get_block_long_term_weight(&'service self, height: u64) -> Result<u64, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		let metadata = ro_tx.get::<table::blockmetadata>(&height)?
			.ok_or(DB_FAILURES::NotFound("Failed to find block's metadata"))?;

		Ok(metadata.long_term_block_weight)
	}

	/// `get_block_timestamp` fetch a block's timestamp.
    ///
    /// Should return the timestamp of the block with given height. In case of failures, a DB_FAILURES will be return. Precisely, a `BLOCK_DNE`
    /// error will be returned if the requested block can't be found.
    ///
    /// Parameters:
    /// `height`: is the given height where the requested block to fetch timestamp is located.
    fn get_block_timestamp(&'service self, height: u64) -> Result<u64, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		let metadata = ro_tx.get::<table::blockmetadata>(&height)?
			.ok_or(DB_FAILURES::NotFound("Failed to find block's metadata"))?;

		Ok(metadata.timestamp)	
	}

	/// `get_block_cumulative_rct_outputs` fetch a blocks' cumulative number of RingCT outputs
    ///
    /// Should return the number of RingCT outputs in the blockchain up to the blocks located at the given heights. In case of failures, a DB_FAILURES will be return. Precisely, a `BLOCK_DNE`
    /// error will be returned if the requested block can't be found.
    ///
    /// Parameters:
    /// `heights`: is the collection of height to check for RingCT distribution.
    fn get_block_cumulative_rct_outputs(&'service self, height: u64) -> Result<u64, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		let metadata = ro_tx.get::<table::blockmetadata>(&height)?
			.ok_or(DB_FAILURES::NotFound("Failed to find block's metadata"))?;

		Ok(metadata.cum_rct)
	}

	fn get_block(&'service self, hash: Hash) -> Result<Block, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		let blk_height = ro_tx.get::<table::blockhash>(&hash.into())?
			.ok_or(DB_FAILURES::NotFound("Can't find block"))?;

		Ok(ro_tx.get::<table::blocks>(&blk_height)?
					.ok_or(DB_FAILURES::NotFound("Can't find block"))?.0)
	}

	fn get_block_from_height(&'service self, height: u64) -> Result<Block, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;

		Ok(ro_tx.get::<table::blocks>(&height)?
					.ok_or(DB_FAILURES::NotFound("Can't find block"))?.0)
	}

	/// `get_block_header` fetches the block's header with the given hash.
    ///
    /// Return the requested block header. In case of failures, a `DB_FAILURES` will be return. Precisely, a `BLOCK_DNE`
    /// error will be returned if the requested block can't be found.
    ///
    /// Parameters:
    /// `h`: is the given hash of the requested block.
    fn get_block_header(&'service self, hash: Hash) -> Result<BlockHeader, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		let blk_height = ro_tx.get::<table::blockhash>(&hash.into())?
			.ok_or(DB_FAILURES::NotFound("Can't find block"))?;

		Ok(ro_tx.get::<table::blocks>(&blk_height)?
					.ok_or(DB_FAILURES::NotFound("Can't find block"))?.0.header)
	}

	fn get_block_header_from_height(&'service self, height: u64) -> Result<BlockHeader, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;

		Ok(ro_tx.get::<table::blocks>(&height)?
					.ok_or(DB_FAILURES::NotFound("Can't find block"))?.0.header)
	}

	/// `get_top_block` fetch the last/top block of the blockchain
    ///
    /// Return the last/top block of the blockchain. In case of failures, a DB_FAILURES, will be return.
    ///
    /// No parameters is required.
    fn get_top_block(&'service self) -> Result<Block, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		let blk_height = self.height()?;

		Ok(ro_tx.get::<table::blocks>(&blk_height)?
					.ok_or(DB_FAILURES::NotFound("Can't find block"))?.0)
	}

    /// `get_top_block_hash` fetch the block's hash located at the top of the blockchain (the last one).
    ///
    /// Return the hash of the last block. In case of failures, a DB_FAILURES will be return.
    ///
    /// No parameters is required
    fn get_top_block_hash(&'service self) -> Result<Hash, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		let height = self.height()?;
		let metadata = ro_tx.get::<table::blockmetadata>(&height)?
			.ok_or(DB_FAILURES::NotFound("Failed to find block's metadata"))?;

		Ok(metadata.block_hash.0)
	}














	// ------------------------------|  Transactions  |-----------------------------

	/// `add_transaction` add the corresponding transaction and its hash to the specified block.
    ///
    /// In case of failures, a DB_FAILURES will be return. Precisely, a TX_EXISTS will be returned if the
    /// transaction to be added already exists in the database.
    ///
    /// Parameters:
    /// `blk_hash`: is the hash of the block which inherit the transaction
    /// `tx`: is obviously the transaction to add
    /// `tx_hash`: is the hash of the transaction.
    /// `tx_prunable_hash_ptr`: is the hash of the prunable part of the transaction.
    fn add_transaction(&'service self, tx: monero::Transaction) 
	-> Result<(), DB_FAILURES> 
	{
		let is_coinbase: bool = tx.prefix.inputs.is_empty();
		let tx_hash = tx.hash();

		let mut tx_prunable_blob = Vec::new();
		get_transaction_prunable_blob(&tx, &mut tx_prunable_blob).unwrap();

		let tx_prunable_hash: [u8; 32] = monero::Hash::new(&tx_prunable_blob).0;
		
		for txin in tx.prefix.inputs.iter() {
			if let TxIn::ToKey { amount: _, key_offsets: _, k_image } = txin {
				self.add_spent_key(crate::types::KeyImage(k_image.image.into()))?;
			} 
			else {
				return Err(DB_FAILURES::Other("Unsupported input type, aborting transaction addition"))
			}
		}
		
		let tx_id = self.add_transaction_data((tx.clone(), tx_prunable_blob), tx_hash, Hash(tx_prunable_hash))?;

		let tx_num_outputs = tx.prefix.outputs.len();
		let amount_output_dinces: Vec<u64> = Vec::with_capacity(tx_num_outputs);

		for txout in tx.prefix.outputs.iter().zip(0..tx_num_outputs) {

			if is_coinbase && tx.prefix.version.0 == 2 {

				let commitment: Option<Key> = None;
				// ZeroCommit is from RingCT Module, not finishable yet
			}
		}
		todo!()
	}

	/// `add_transaction_data` add the specified transaction data to its storage.
    ///
    /// It only add the transaction blob and tx's metadata, not the collection of outputs.
    ///
    /// Return the hash of the transaction added. In case of failures, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `blk_hash`: is the hash of the block containing the transaction
    /// `tx_and_hash`: is a tuple containing the transaction and it's hash
    /// `tx_prunable_hash`: is the hash of the prunable part of the transaction
    fn add_transaction_data(&'service self, txp: (monero::Transaction, Vec<u8>), tx_hash: Hash, tx_prunable_hash: Hash) 
	-> Result<u64, DB_FAILURES> 
	{
		// Checking if the transaction already exist in the database
		let res = self.get::<table::txsidentifier>(&tx_hash.into())?;
		if res.is_some() {
			return Err(DB_FAILURES::AlreadyExist("Attempting to add transaction that's already in the db"))
		}

		// Inserting tx index in table::txsindetifier
		let height = self.height()?;
		let tx_id = self.get_num_tx()?;

		let txindex = TxIndex {
    		tx_id,
    		unlock_time: txp.0.prefix.unlock_time.0,
    		height,
		};

		self.put::<table::txsidentifier>(&tx_hash.into(), &txindex)?;

		// TODO: Investigate unprunable_size == 0 condition
		// Inserting tx pruned part in table::txsprefix
		let tx_pruned = TransactionPruned {
			prefix: txp.0.prefix.clone(),
			rct_signatures: txp.0.rct_signatures
		};
		self.put::<table::txsprefix>(&tx_id, &tx_pruned)?;

		// Inserting tx prunable part in table::txs
		self.put::<table::txsprunable>(&tx_id, &txp.1)?;

		// Checking pruning seed and inserting into table::txsprunabletip accordingly
		if self.get_blockchain_pruning_seed()? > 0 {
			self.put::<table::txsprunabletip>(&tx_id, &height)?;
		}

		// V2 Tx store hash of their prunable part
		if txp.0.prefix.version.0 > 1 {
			self.put::<table::txsprunablehash>(&tx_id, &tx_prunable_hash.into())?;
		}
		Ok(tx_id)
	}

	fn remove_transaction(&'service self, tx_hash: Hash) -> Result<(), DB_FAILURES> {
		let txpruned = self.get_pruned_tx(tx_hash)?;

		for input in txpruned.prefix.inputs.iter() {
			if let TxIn::ToKey { amount: _, key_offsets: _, k_image } = input {
				self.remove_spent_key(KeyImage(k_image.image.into()))?;
			}
		}

		self.remove_transaction_data(txpruned.prefix, tx_hash)
	}

	fn remove_transaction_data(&'service self, txprefix: monero::TransactionPrefix, tx_hash: Hash) -> Result<(), DB_FAILURES> {

		// Checking if the transaction exist and fetching its index
		let txindex = self.get::<table::txsidentifier>(&tx_hash.into())?
			.ok_or(DB_FAILURES::NotFound("Attempting to remove transaction that isn't in the db"))?;

		self.delete::<table::txsprefix>(&txindex.tx_id, &None)?;
		self.delete::<table::txsprunable>(&txindex.tx_id, &None)?;
		// If Its in Tip blocks range we must delete it
		if self.get::<table::txsprunabletip>(&txindex.tx_id)?.is_some() {
			self.delete::<table::txsprunabletip>(&txindex.tx_id, &None)?;
		}
		// If v2 Tx we must delete the prunable hash
		if txprefix.version.0 > 1 {
			self.delete::<table::txsprunablehash>(&txindex.tx_id, &None)?;
		}

		self.remove_tx_outputs(txprefix, txindex.tx_id)?;

		self.delete::<table::txsoutputs>(&txindex.tx_id, &None)?;
		self.delete::<table::txsidentifier>(&tx_hash.into(), &None)
	}

	fn remove_tx_outputs(&'service self, txprefix: monero::TransactionPrefix, tx_id: u64) -> Result<(), DB_FAILURES> {

		let amount_output_indices: TxOutputIdx = self.get::<table::txsoutputs>(&tx_id)?
			.ok_or(DB_FAILURES::NotFound("Failed to find tx's outputs indices"))?;

		if amount_output_indices.0.is_empty() {
			return Err(DB_FAILURES::Other("Attempting to remove outputs of a an empty tx"));
		}
		
		#[allow(clippy::match_like_matches_macro)]
		let is_pseudo_rct: bool = match &txprefix.inputs[0] {
			TxIn::Gen {height:_} if txprefix.version.0 > 1 && txprefix.inputs.len() == 1 => { true },
			_ => { false }
		};
		for o in 0..txprefix.outputs.len() {
			let amount = match is_pseudo_rct {
				true => 0,
				false => txprefix.outputs[o].amount.0
			};
			self.remove_output(Some(amount), amount_output_indices.0[o])?;
		}
		Ok(())
	}

	/// `get_num_tx` fetches the total number of transactions stored in the database
    ///
    /// Should return the count. In case of failure, a DB_FAILURES will be return.
    ///
    /// No parameters is required.
    fn get_num_tx(&'service self) -> Result<u64, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		ro_tx.num_entries::<table::txsprefix>().map(|n| n as u64)
	}

	/// `tx_exists` check if a transaction exist with the given hash.
    ///
    /// Return `true` if the transaction exist, `false` otherwise. In case of failure, a DB_FAILURES will be return.
    ///
    /// Parameters :
    /// `hash` is the given hash of transaction to check.
    fn tx_exists(&'service self, hash: Hash) -> Result<bool, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		Ok(ro_tx.get::<table::txsidentifier>(&hash.into())?.is_some())
	}

	/// `get_tx_unlock_time` fetch a transaction's unlock time/height
    ///
    /// Should return the unlock time/height in u64. In case of failure, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `hash`: is the given hash of the transaction to check.
    fn get_tx_unlock_time(&'service self, hash: Hash) -> Result<u64, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;

		// Getting the tx index
		let txindex = ro_tx.get::<table::txsidentifier>(&hash.into())?
			.ok_or(DB_FAILURES::NotFound("wasn't able to find a transaction in the database"))?;

		Ok(txindex.unlock_time)
	}

	/// `get_tx` fetches the transaction with the given hash.
    ///
    /// Should return the transaction. In case of failure, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `hash`: is the given hash of transaction to fetch.
    fn get_tx(&'service self, hash: Hash) -> Result<monero::Transaction, DB_FAILURES> {

		// Getting the pruned tx
		let pruned_tx = self.get_pruned_tx(hash)?;
		
		// Getting the tx index
		let ro_tx = self.db.tx().map_err(Into::into)?;
		let txindex = ro_tx.get::<table::txsidentifier>(&hash.into())?
			.ok_or(DB_FAILURES::NotFound("failed to find index of a transaction"))?;

		// Getting its prunable part
		let prunable_part = ro_tx.get::<table::txsprunable>(&txindex.tx_id)?
			.ok_or(DB_FAILURES::NotFound("failed to find prunable part of a transaction"))?;

		// Making it a Transaction
		pruned_tx.into_transaction(&prunable_part)
			.map_err(|err| DB_FAILURES::SerializeIssue(err.into()))
	}

	/// `get_tx_list` fetches the transactions with given hashes.
    ///
    /// Should return a vector with the requested transactions. In case of failures, a DB_FAILURES will be return.
    /// Precisly, a HASH_DNE error will be returned with the correspondig hash of transaction that is not found in the DB.
    ///
    /// `hlist`: is the given collection of hashes correspondig to the transactions to fetch.
    fn get_tx_list(&'service self, hash_list: Vec<Hash>) -> Result<Vec<monero::Transaction>, DB_FAILURES> {
		let mut result: Vec<monero::Transaction> = Vec::new();
		
		for hash in hash_list {
			result.push(self.get_tx(hash)?);
		}
		Ok(result)
	}

	/// `get_pruned_tx` fetches the transaction base with the given hash.
    ///
    /// Should return the transaction. In case of failure, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `h`: is the given hash of transaction to fetch.
    fn get_pruned_tx(&'service self, hash: Hash) -> Result<TransactionPruned, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;

		let txindex = ro_tx.get::<table::txsidentifier>(&hash.into())?
			.ok_or(DB_FAILURES::NotFound("wasn't able to find a transaction in the database"))?;

		ro_tx.get::<table::txsprefix>(&txindex.tx_id)?
			.ok_or(DB_FAILURES::NotFound("failed to find prefix of a transaction"))
	}

	/// `get_tx_block_height` fetches the height of a transaction's block
    ///
    /// Should return the height of the block containing the transaction with the given hash. In case
    /// of failures, a DB FAILURES will be return. Precisely, a TX_DNE error will be return if the transaction cannot be found.
    ///
    /// Parameters:
    /// `hash`: is the fiven hash of the first transaction
    fn get_tx_block_height(&'service self, hash: Hash) -> Result<u64, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		let txindex = ro_tx.get::<table::txsidentifier>(&hash.into())?
			.ok_or(DB_FAILURES::NotFound("txindex not found"))?;
		Ok(txindex.height)
	}
























	
	// --------------------------------|  Outputs  |--------------------------------

	/// `add_output` add an output data to it's storage .
    ///
    /// It internally keep track of the global output count. The global output count is also used to index outputs based on
    /// their order of creations.
    ///
    /// Should return the amount output index. In case of failures, a DB_FAILURES will be return.
	///
    /// Parameters:
    /// `tx_hash`: is the hash of the transaction where the output comes from.
    /// `output`: is the output's publickey to store.
	/// `index`: is the local output's index (from transaction).
	/// `unlock_time`: is the unlock time (height) of the output.
	/// `commitment`: is the RingCT commitment of this output.
    fn add_output(&'service self, tx_hash: Hash, output: TxOut, local_index: u64, unlock_time: u64, commitment: Option<Key>) 
	-> Result<u64, DB_FAILURES> 
	{
		let height = self.height()?;

		let pubkey = output.target.as_one_time_key().map(Into::into);
		let mut out_metadata = OutputMetadata {
			tx_hash: tx_hash.into(),
			local_index,
			pubkey,
			unlock_time,
			height,
			commitment: None,
		};

		// RingCT Outputs
		if let Some(commitment) = commitment {

			out_metadata.commitment = Some(commitment.into());

			let amount_index = self.get_rct_num_outputs()? + 1;
			self.put::<table::outputmetadata>(&amount_index, &out_metadata)?;
			Ok(amount_index)
		}
		// Pre-RingCT Outputs
		else {

			let amount_index = self.get_pre_rct_num_outputs(output.amount.0)? + 1;
			let mut cursor = self.write_cursor_dup::<table::prerctoutputmetadata>()?;
			cursor.put_cursor_dup(&output.amount.0, &amount_index, &out_metadata)?;
			Ok(amount_index)
		}
	}

	fn remove_output(&'service self, amount: Option<u64>, index: u64) -> Result<(), DB_FAILURES> {
		if let Some(amount) = amount {
			if amount == 0 {
				return self.delete::<table::outputmetadata>(&index, &None)
			}
			let mut cursor = self.write_cursor_dup::<table::prerctoutputmetadata>()?;
			let _ = cursor.get_dup(&amount, &index)?
				.ok_or(DB_FAILURES::NotFound("Failed to find PreRCT output metadata"))?;
			cursor.del()
		} 
		else {
			self.delete::<table::outputmetadata>(&index, &None)
		}
	}

	/// `get_output` get an output's data
    ///
    /// Return the public key, unlock time, and block height for the output with the given amount and index, collected in a struct
    /// In case of failures, a `DB_FAILURES` will be return. Precisely, if the output cannot be found, an `OUTPUT_DNE` error will be return.
    /// If any of the required part for the final struct isn't found, a `DB_ERROR` will be return
    ///
    /// Parameters:
    /// `amount`: is the corresponding amount of the output
    /// `index`: is the output's index (indexed by amount)
    /// `include_commitment` : `true` by default.
    fn get_output(&'service self, amount: Option<u64>, index: u64) -> Result<OutputMetadata, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		if let Some(amount) = amount {
			let mut cursor = ro_tx.cursor_dup::<table::prerctoutputmetadata>()?;
			cursor.get_dup(&amount, &index)?
				.ok_or(DB_FAILURES::NotFound("Failed to find PreRCT output metadata"))
		} else {
			ro_tx.get::<table::outputmetadata>(&index)?
				.ok_or(DB_FAILURES::NotFound("Failed to find PostRCT output metadata"))
		}
	}

    /// `get_output_list` gets a collection of output's data from a corresponding index collection.
    ///
    /// Return a collection of output's data. In case of failurse, a `DB_FAILURES` will be return.
    ///
    /// Parameters:
    /// `amounts`: is the collection of amounts corresponding to the requested outputs.
    /// `offsets`: is a collection of outputs' index (indexed by amount).
    /// `allow partial`: `false` by default.
    fn get_output_list(
		&'service self,
		amounts: Option<Vec<u64>>,
		offsets: Vec<u64>,
    	) -> Result<Vec<OutputMetadata>, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		let mut result: Vec<OutputMetadata> = Vec::new();
		
		// Pre-RingCT output to be found.
		if let Some(amounts) = amounts {
			let mut cursor = ro_tx.cursor_dup::<table::prerctoutputmetadata>()?;

			for ofs in amounts.into_iter().zip(offsets) {

				if ofs.0 == 0 {
					let output = ro_tx.get::<table::outputmetadata>(&ofs.1)?
						.ok_or(DB_FAILURES::NotFound("An output hasn't been found in the database"))?;
					result.push(output);
				} else {
					let output = cursor.get_dup(&ofs.0, &ofs.1)?
						.ok_or(DB_FAILURES::NotFound("An output hasn't been found in the database"))?;
					result.push(output);
				}
			}
		// No Pre-RingCT outputs to be found.
		} else {
			for ofs in offsets {

				let output = ro_tx.get::<table::outputmetadata>(&ofs)?
					.ok_or(DB_FAILURES::NotFound("An output hasn't been found in the database"))?;
				result.push(output);				
			}
		}

		Ok(result)
	}

    /// `get_num_outputs` fetches the number post-RingCT output.
    ///
    /// Return the number of post-RingCT outputs. In case of failures a `DB_FAILURES` will be return.
    ///
    /// Parameters:
    /// `amount`: is the output amount being looked up.
    fn get_rct_num_outputs(&'service self) -> Result<u64, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		
		ro_tx.num_entries::<table::outputmetadata>().map(|n| n as u64)
	}

	/// `get_pre_rct_num_outputs` fetches the number of preRCT outputs of a given amount.
    ///
    /// Return a count of outputs of the given amount. in case of failures a `DB_FAILURES` will be return.
    ///
    /// Parameters:
    /// `amount`: is the output amount being looked up.
	fn get_pre_rct_num_outputs(&'service self, amount: u64) -> Result<u64, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		let mut cursor = ro_tx.cursor_dup::<table::prerctoutputmetadata>()?;

		transaction::Cursor::set(&mut cursor, &amount)?;
		let out_metadata: Option<(u64, OutputMetadata)> = transaction::DupCursor::last_dup(&mut cursor)?;
		if let Some(out_metadata) = out_metadata {
			return Ok(out_metadata.0)
		}
		Err(DB_FAILURES::Other("failed to decode the subkey and value"))
	}




















	// ------------------------------| Spent Keys |------------------------------

	/// `add_spent_key` add the supplied key image to the spent key image record
	fn add_spent_key(&'service self, key_image: KeyImage) -> Result<(), DB_FAILURES> {
		self.put::<table::spentkeys>(&key_image, &())
	}

	/// `remove_spent_key` remove the specified key image from the spent key image record
    fn remove_spent_key(&'service self, key_image: KeyImage) -> Result<(), DB_FAILURES> {
		self.delete::<table::spentkeys>(&key_image, &None)
	}

	/// `is_spent_key_recorded` check if the specified key image has been spent
	fn is_spent_key_recorded(&'service self, key_image: KeyImage) -> Result<bool, DB_FAILURES> {
		Ok(self.get::<table::spentkeys>(&key_image)?.is_some())
	}














	// --------------------------------------------|  Alt-Block  |------------------------------------------------------------

    /// `add_alt_block` add a new alternative block.
    ///
    /// In case of failures, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// blkid: is the hash of the original block
    /// data: is the metadata for the block
    /// blob: is the blobdata of this alternative block.
    fn add_alt_block(&'service self, altblock_hash: Hash, data: AltBlock) -> Result<(), DB_FAILURES> {
		self.put::<table::altblock>(&altblock_hash.into(), &data)
	}

    /// `get_alt_block` gets the specified alternative block.
    ///
    /// Return a tuple containing the blobdata of the alternative block and its metadata. In case of failures, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `blkid`: is the hash of the requested alternative block.
    fn get_alt_block(&'service self, altblock_hash: Hash) -> Result<AltBlock, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		ro_tx.get::<table::altblock>(&altblock_hash.into())?
			.ok_or(DB_FAILURES::NotFound("Failed to find an AltBLock in the db"))
	}

    /// `remove_alt_block` remove the specified alternative block
    ///
    /// In case of failures, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `blkid`: is the hash of the alternative block to remove.
    fn remove_alt_block(&mut self, altblock_hash: Hash) -> Result<(), DB_FAILURES> {
		self.delete::<table::altblock>(&altblock_hash.into(), &None)
	}

    /// `get_alt_block` gets the total number of alternative blocks stored
    ///
    /// In case of failures, a DB_FAILURES will be return.
    ///
    /// No parameters is required.
    fn get_alt_block_count(&'service self) -> Result<u64, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		ro_tx.num_entries::<table::altblock>().map(|n| n as u64)
	}

    /// `drop_alt_block` drop all alternative blocks.
    ///
    /// In case of failures, a DB_FAILURES will be return.
    ///
    /// No parameters is required.
    fn drop_alt_blocks(&mut self) -> Result<(), DB_FAILURES> {
		self.clear::<table::altblock>()
	}

	// --------------------------------| Properties |--------------------------------

	// No pruning yet
	fn get_blockchain_pruning_seed(&'service self) -> Result<u32, DB_FAILURES> {
		Ok(0)
	} 




	// --------------------------------| Blocks |--------------------------------
	/*
	/// `pop_block` pops the top block off the blockchain. This cause the last block to be deleted
	/// from all its reference tables, including transactions it was refering to.
    ///
	/// Return the block that was popped. In case of failures, a `DB_FAILURES` will be return.
	///
    /// No parameters is required.
	
    fn pop_block(&'service self) -> Result<Option<Block>, DB_FAILURES> {
		let current_height = self.height()?;

		let blk = self.get::<table::blocks>(&current_height)?;

		self.delete::<table::blocks>(&current_height, &None)?;
		self.delete::<table::blockmetadata>(&current_height, &None)?;

		// Re-encoding the slice and get the hash
		let e_blk = bincode::encode_to_vec(&blk, BINCODE_CONFIG)
			.map_err(|e| DB_FAILURES::SerializeIssue(error::DB_SERIAL::BincodeEncode(e)))?;
		let hash = Hash::new(keccak_256(e_blk.as_slice())).into();
				
		self.delete::<table::blockhash>(&hash, &None)?;

		// Now let's delete all its transactions
		blk.0.tx_hashes.iter().for_each(|_tx| {

			todo!()
		});
		return Ok(Some(blk.0))
	}*/
}