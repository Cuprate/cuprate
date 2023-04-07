//! ### Types module
//! This module contains definition and implementations of some of the structures stored in the database.
//! Some of these types are just Wrapper for convenience or re-definition of `monero-rs` database type (see Boog900/monero-rs, "db" branch)
//! Since the database do not use dummy keys, these redefined structs are the same as monerod without the prefix data used as a key.
//! All these types implement [`bincode::Encode`] and [`bincode::Decode`]. They can store `monero-rs` types in their field. In this case, these field
//! use the [`Compat<T>`] wrapper.

use bincode::{Encode, Decode, enc::write::Writer};
use crate::encoding::{Compat, ReaderCompat};
use monero::{Hash, Block, PublicKey, util::ringct::{Key, RctSigBase, RctSig}, TransactionPrefix, consensus::Decodable};

// ---- BLOCKS ----

#[derive(Clone, Debug, Encode, Decode)]
/// [`BlockMetadata`] is a struct containing metadata of a block such as  the block's `timestamp`, the `total_coins_generated` at this height, its `weight`, its difficulty (`diff_lo`) 
/// and cumulative difficulty (`diff_hi`), the `block_hash`, the cumulative RingCT (`cum_rct`) and its long term weight (`long_term_block_weight`). The monerod's struct equivalent is `mdb_block_info_4`
/// This struct is used in [`crate::table::blockmetadata`] table.
pub struct BlockMetadata {
	/// Block's timestamp (the time at which it started to be mined)
	pub timestamp: u64,
	/// Total monero supply, this block included
	pub total_coins_generated: u64,
	/// Block's weight (sum of all transactions weights)
	pub weight: u64,
	/// Block's difficulty. In monerod this field would have been split into two `u64`, since cpp don't support *natively* `uint_128t`/`u128`
	pub difficulty: u128,
	/// Block's hash
	pub block_hash: Compat<Hash>,
	/// bi_cum_rct
	pub cum_rct: u64,
	/// bi_long_term_block_weight
	pub long_term_block_weight: u64,
}

#[derive(Clone, Debug, Encode, Decode)]
/// [`AltBlock`] is a struct contaning an alternative `block` (defining an alternative mainchain) and its metadata (`block_height`, `cumulative_weight`, 
/// `cumulative_difficulty_low`, `cumulative_difficulty_high`, `already_generated_coins`).
/// This struct is used in [`crate::table::altblock`] table.
pub struct AltBlock {
	/// Alternative block's height.
	pub height: u64,
	/// Cumulative weight median at this block
	pub cumulative_weight: u64,
	/// Cumulative difficulty
	pub cumulative_difficulty: u128,
	/// Total generated coins excluding this block's coinbase reward + fees
	pub already_generated_coins: u64,
	/// Actual block data, with Prefix and Transactions.
	/// It is worth noting that monerod implementation do not contain the block in its struct, but still append it at the end of metadata.
	pub block: Compat<Block>,
}

// ---- TRANSACTIONS ----

#[derive(Clone, Debug)]
/// [`TransactionPruned`] is, as its name suggest, the pruned part of a transaction, which is the Transaction Prefix and its RingCT signatures.
/// This struct is used in the [`crate::table::txsprefix`] table.
pub struct TransactionPruned {
	/// The transaction prefix.
	pub prefix: TransactionPrefix,
	/// The RingCT signatures, will only contain the 'sig' field.
	pub rct_signatures: RctSig,
}

impl bincode::Decode for TransactionPruned {
	
	fn decode<D: bincode::de::Decoder>(decoder: &mut D) -> Result<Self, bincode::error::DecodeError> {
		let mut r = ReaderCompat(decoder.reader());

		// We first decode the TransactionPrefix and get the n° of inputs/outputs
        	let prefix: TransactionPrefix = Decodable::consensus_decode(&mut r)
			.map_err(|_| bincode::error::DecodeError::Other("Monero-rs decoding failed"))?;

		let (inputs, outputs) = (prefix.inputs.len(), prefix.outputs.len());

		// Handle the prefix accordingly to its version
		match *prefix.version {
			// First transaction format, Pre-RingCT, so the signatures are None
			1 => Ok(TransactionPruned {
				prefix,
				rct_signatures: RctSig { sig: None, p: None },
			}),
			_ => {
				let mut rct_signatures = RctSig { sig: None, p: None };
				// No inputs so no RingCT
				if inputs == 0 {
					return Ok(TransactionPruned { prefix, rct_signatures})
				}
				// Otherwise get the RingCT signatures for the tx inputs
				if let Some(sig) = RctSigBase::consensus_decode(&mut r, inputs, outputs)
					.map_err(|_| bincode::error::DecodeError::Other("Monero-rs decoding failed"))?
				{
					rct_signatures = RctSig { sig: Some(sig), p: None };
				}
				// And we return it
				Ok(TransactionPruned { prefix, rct_signatures })
			}
		}
    	}
}

impl bincode::Encode for TransactionPruned {
    	fn encode<E: bincode::enc::Encoder>(&self, encoder: &mut E) -> Result<(), bincode::error::EncodeError> {
        	let writer = encoder.writer();
		// Encoding the Transaction prefix first
		let buf = monero::consensus::serialize(&self.prefix);
		writer.write(&buf)?;
		match *self.prefix.version {
			1 => {} // First transaction format, Pre-RingCT, so the there is no signatures to add
			_ => { if let Some(sig) = &self.rct_signatures.sig {
				// If there is signatures then we append it at the end
				let buf = monero::consensus::serialize(sig);
				writer.write(&buf)?;
			}}
		}
		Ok(())
    	}
}

#[derive(Clone, Debug, Encode, Decode)]
/// [`TxIndex`] is a struct used in the [`crate::table::txsidentifier`]. It store the `unlock_time` of a transaction, the `height` of the block 
/// whose transaction belong to and the Transaction ID (`tx_id`)
pub struct TxIndex {
	/// Transaction ID
	pub tx_id: u64,
	/// The unlock time of this transaction (the height at which it is unlocked, it is not a timestamp)
	pub unlock_time: u64,
	/// The height of the block whose transaction belong to
	pub height: u64,
}

#[derive(Clone, Debug, Encode, Decode)]
/// [`TxOutputIdx`] is a single-tuple struct used to contain the indexes of the transactions outputs. It is defined for more clarity on its role.
/// This struct is used in [`crate::table::txsoutputs`] table.
pub struct TxOutputIdx(Vec<u64>);

// ---- OUTPUTS ----

#[derive(Clone, Debug, Encode, Decode)]
/// [`RctOutkey`] is a struct containing RingCT metadata and an output ID
/// This struct is used in [`crate::table::outputamounts`]
pub struct RctOutkey {
	/// amount_index
	pub amount_index: u64,
	/// The output's ID
	pub output_id: u64,
	/// The output's public key (for spend verification)
	pub pubkey: Compat<PublicKey>,
	/// The output's unlock time (the height at which it is unlocked, it is not a timestamp)
	pub unlock_time: u64,
	/// The height of the block which created the output
	pub height: u64,
	/// The output's amount commitment (for spend verification)
	/// For compatibility with Pre-RingCT outputs, this field is an option. In fact, monerod distinguish between `pre_rct_output_data_t` and `output_data_t` field like that :
	/// ```cpp
	/// // This MUST be identical to output_data_t, without the extra rct data at the end
	/// struct pre_rct_output_data_t
	/// ```
	pub commitment: Option<Compat<Key>>,
}

#[derive(Clone, Debug, Encode, Decode)]
/// [`OutAmountIdx`] is a struct tuple used to contain the two keys used in [`crate::table::outputamounts`] table. 
/// In monerod, the database key is the amount while the *cursor key* (the amount index) is the prefix of the actual data being returned. 
/// As we prefere to note use cursor with partial data, we prefer to concat these two into a unique key
pub struct OutAmountIdx(u64,u64);

#[derive(Clone, Debug, Encode, Decode)]
/// [`OutTx`] is a struct containing the hash of the transaction whose output belongs to, and the local index of this output.
/// This struct is used in [`crate::table::outputinherit`].
pub struct OutTx {
	/// Output's transaction hash
	pub tx_hash: Compat<Hash>,
	/// Local index of the output
	pub local_index: u64,
}

// ---- SPENT ----
#[derive(Clone, Debug, Encode, Decode)]
/// [`KeyImage`] is a single-tuple struct used to contain a [`monero::Hash`]. It is defined for more clarity on its role. This 
/// struct is used in [`crate::table::spentkeys`] table.
pub struct KeyImage(Compat<Hash>);