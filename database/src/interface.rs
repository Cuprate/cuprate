//! ### Interface module
//! This module contains all the implementations of interface.

use monero::{cryptonote::hash::keccak_256, Hash, Block, TxOut, util::ringct::Key};
use crate::{error::{DB_FAILURES, self}, database::{Database, Interface}, table, transaction::{Transaction, Cursor, WriteTransaction, DupCursor, DupWriteCursor, self}, BINCODE_CONFIG, types::{TransactionPruned, OutputMetadata}};

// Implementation of Interface
impl<'service, D: Database<'service>> Interface<'service,D> {













	// --------------------------------| Blockchain |--------------------------------
	
	/// `height` fetch the current blockchain height.
    ///
    /// Return the current blockchain height. In case of failures, a DB_FAILURES will be return.
    ///
    /// No parameters is required.
	fn height(&'service self) -> Result<Option<u64>,DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		let mut cursor = ro_tx.cursor::<table::blockhash>()?;

		let last = cursor.last()?;
		if let Some(pair) = last {
			return Ok(Some(pair.1))
		}
		Ok(None)
	}

	/// `update_hard_fork_version` update which hardfork version a height is on.<br>
    ///
    /// In case of failures, a `DB_FAILURES` will be return.
    ///
    /// Parameters:<br>
 	/// `height`: is the height where the hard fork happen.<br>
    /// `version`: is the version of the hard fork.
    fn update_hard_fork_version(&'service self, height: u64, hardfork_version: u8) -> Result<(), DB_FAILURES> {
		let b_hf = self.get::<table::blockhfversion>(&height)?;

		if let Some(b_hf) = b_hf {
			if b_hf != hardfork_version {
				self.put::<table::blockhfversion>(&height, &hardfork_version)?;
			}
			Ok(())
		} else {
			Err(DB_FAILURES::DataNotFound)
		}
	}

	/// `get_hard_fork_version` checks which hardfork version a height is on.
    ///
    /// In case of failures, a `DB_FAILURES` will be return.
    ///
    /// Parameters:<br>
    /// `height`: is the height to check.
    fn get_hard_fork_version(&'service self, height: u64) -> Result<Option<u8>, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		ro_tx.get::<table::blockhfversion>(&height)
	}



















	// ------------------------------|  Transactions  |-----------------------------

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
		let txindex = ro_tx.get::<table::txsidentifier>(&hash.into())?
			.ok_or(DB_FAILURES::NoneFound("wasn't able to find a transaction in the database"))?;
		Ok(txindex.unlock_time)
	}

	/// `get_tx` fetches the transaction with the given hash.
    ///
    /// Should return the transaction. In case of failure, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `hash`: is the given hash of transaction to fetch.
    fn get_tx(&'service self, hash: Hash) -> Result<Option<monero::Transaction>, DB_FAILURES> {
		let pruned_tx = self.get_pruned_tx(hash)?
			.ok_or(DB_FAILURES::NoneFound("failed to find prefix of a transaction"))?;
		
		let ro_tx = self.db.tx().map_err(Into::into)?;
		let txindex = ro_tx.get::<table::txsidentifier>(&hash.into())?
			.ok_or(DB_FAILURES::NoneFound("failed to find index of a transaction"))?;
		let prunable_part = ro_tx.get::<table::txsprunable>(&txindex.tx_id)?
			.ok_or(DB_FAILURES::NoneFound("failed to find prunable part of a transaction"))?;

		Ok(Some(pruned_tx.into_transaction(prunable_part.as_slice())
			.map_err(|_| DB_FAILURES::SerializeIssue(error::DB_SERIAL::ConsensusDecode(prunable_part)))?))
	}

	/// `get_pruned_tx` fetches the transaction base with the given hash.
    ///
    /// Should return the transaction. In case of failure, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `h`: is the given hash of transaction to fetch.
    fn get_pruned_tx(&'service self, hash: Hash) -> Result<Option<TransactionPruned>, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;

		let txindex = ro_tx.get::<table::txsidentifier>(&hash.into())?
			.ok_or(DB_FAILURES::NoneFound("wasn't able to find a transaction in the database"))?;
		ro_tx.get::<table::txsprefix>(&txindex.tx_id)
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
		let height = self.height()?
			.ok_or(DB_FAILURES::NoneFound("add_output() didn't find a blockchain height"))?;

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
    fn get_output(&'service self, amount: Option<u64>, index: u64) -> Result<Option<OutputMetadata>, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		if let Some(amount) = amount {
			let mut cursor = ro_tx.cursor_dup::<table::prerctoutputmetadata>()?;
			cursor.get_dup(&amount, &index)
		} else {
			ro_tx.get::<table::outputmetadata>(&index)
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
    	) -> Result<Option<Vec<OutputMetadata>>, DB_FAILURES> {
		let ro_tx = self.db.tx().map_err(Into::into)?;
		let mut result: Vec<OutputMetadata> = Vec::new();
		
		// Pre-RingCT output to be found.
		if let Some(amounts) = amounts {
			let mut cursor = ro_tx.cursor_dup::<table::prerctoutputmetadata>()?;

			for ofs in amounts.into_iter().zip(offsets) {

				if ofs.0 == 0 {
					let output = ro_tx.get::<table::outputmetadata>(&ofs.1)?
						.ok_or(DB_FAILURES::NoneFound("An output hasn't been found in the database"))?;
					result.push(output);
				} else {
					let output = cursor.get_dup(&ofs.0, &ofs.1)?
						.ok_or(DB_FAILURES::NoneFound("An output hasn't been found in the database"))?;
					result.push(output);
				}
			}
		// No Pre-RingCT outputs to be found.
		} else {
			for ofs in offsets {

				let output = ro_tx.get::<table::outputmetadata>(&ofs)?
					.ok_or(DB_FAILURES::NoneFound("An output hasn't been found in the database"))?;
				result.push(output);				
			}
		}

		Ok(Some(result))
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













	// --------------------------------| Blocks |--------------------------------

	/// `pop_block` pops the top block off the blockchain. This cause the last block to be deleted
	/// from all its reference tables, including transactions it was refering to.
    ///
	/// Return the block that was popped. In case of failures, a `DB_FAILURES` will be return.
	///
    /// No parameters is required.
    fn pop_block(&'service self) -> Result<Option<Block>, DB_FAILURES> {
		let current_height = self.height()?;

		if let Some(current_height) = current_height{
			let blk = self.get::<table::blocks>(&current_height)?;

			// Get the block and delete it from block's tables
			if let Some(blk) = blk {
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
			}
		}
		Ok(None)
	}
}