//! ### Table module
//! This module contains the definition of the [`Table`] and [`DupTable`] trait, and the actual tables used in the database. 
//! [`DupTable`] are just a trait used to define that they support DUPSORT|DUPFIXED operation (as of now we don't know the equivalent for HSE).
//! All tables are defined with docs explaining its purpose, what types are the key and data.
//! For more details please look at Cuprate's book : <link to cuprate book> 

use monero::{Hash, Block};
use bincode::{enc::Encode,de::Decode};
use crate::{types::{BlockMetadata, OutAmountIdx, KeyImage, TxOutputIdx, OutTx, AltBlock, TxIndex, TransactionPruned, RctOutkey}, encoding::Compat};

/// A trait implementing a table interaction for the database. It is implemented to an empty struct to specify the name and table's associated types. These associated 
/// types are used to simplify deserialization process.
pub trait Table: Send + Sync + 'static + Clone {
		
	// name of the table
	const TABLE_NAME: &'static str;

	// Definition of a key & value types of the database
	type Key: Encode + Decode;
	type Value: Encode + Decode;
}

/// A trait implementing a table with DUPFIXED & DUPSORT support.
pub trait DupTable: Table {}

/// This declarative macro declare a new empty struct and impl the specified name, and corresponding types. 
macro_rules! impl_table {
	( $(#[$docs:meta])* $table:ident , $key:ty , $value:ty ) => {
        	#[derive(Clone)]
		$(#[$docs])*
		pub(crate) struct $table;

   		impl Table for $table {
	 		const TABLE_NAME: &'static str = "$table";
        		type Key = $key;
                	type Value = $value;
        	}
	};
}

/// This declarative macro declare extend the original impl_table! macro by implementy DupTable trait.
macro_rules! impl_duptable {
	($(#[$docs:meta])* $table:ident, $key:ty, $value:ty) => {
		impl_table!($(#[$docs])* $table, $key, $value);
	};
}

// ------------------------------------------|      Tables definition    |------------------------------------------

// ----- BLOCKS -----

impl_duptable!(
	/// `blockhash` is table defining a relation between the hash of a block and its height. Its primary use is to quickly find block's hash by its height.
	blockhash, Compat<Hash>, u64);

impl_duptable!(
	/// `blockmetadata` store block metadata alongside their corresponding Hash. The blocks metadata can contains the total_coins_generated, weight, long_term_block_weight & cumulative RingCT
	blockmetadata, u64, BlockMetadata);
	
impl_table!(
	/// `blockbody` store blocks' bodies along their Hash. The blocks body contains the coinbase transaction and its corresponding mined transactions' hashes.
	blocks, u64, Compat<Block>);

impl_table!(
	/// `blockhfversion` keep track of block's hard fork version. If an outdated node continue to run after a hard fork, it needs to know, after updating, what blocks needs to be update.
	blockhfversion, u64, u8);
	
impl_table!( 
	/// `altblock` is a table that permit the storage of blocks from an alternative chains being submitted to the txpool. These blocks can be fetch by their corresponding hash.
	altblock, Compat<Hash>, AltBlock);

// ------- TXNs -------

impl_table!(
	/// `txsprefix` is table storing TransactionPruned (or Pruned Tx). These can be fetch by the corresponding Transaction ID.
	txsprefix, u64, TransactionPruned);
	
impl_table!(
	/// `txsprunable` is a table storing the Prunable part of transactions (Signatures and RctSig), stored as raw bytes. These can be fetch by the corresponding Transaction ID.
	txsprunable, u64, Vec<u8>); 
	
impl_duptable!(
	/// `txsprunablehash` is a table storing hashes of prunable part of transactions. These hash can be fetch by the corresponding Transaction ID.
	txsprunablehash, u64, Compat<Hash>);

impl_duptable!(
	/// `txsprunabletip` is a table used for optimization purpose. It defines at which block's height this transaction belong. These can be fetch by the corresponding Transaction ID.
	txsprunabletip, u64, u64);
	
impl_duptable!(
	/// `txsoutputs` is a table storing output indices used in a transaction. These can be fetch by the corresponding Transaction ID.
	txsoutputs, Compat<Hash>, TxOutputIdx);

impl_duptable!(
	/// `txsidentifier` is a table defining a relation between the hash of a transaction and its transaction Indexes. Its primarly used to quickly find tx's ID by its hash.
	txsidentifier, Compat<Hash>, TxIndex);
	
// ---- OUTPUTS ----

impl_duptable!(
	/// `outputsinherit` is table defining relation between outputs ID and its transaction's hash and its local index in it.
	outputinherit, u64, OutTx);
impl_duptable!(
	/// `outputamounts` is a table permiting to find the RingCT Output Key with the amount of an output and its amount index.
	outputamounts, OutAmountIdx, RctOutkey);

//  ---- SPT KEYS ----

impl_duptable!(
	/// `spentkeys`is a table storing every KeyImage that have been used to create decoys input. As these KeyImage can't be re used they need to marked. 
	spentkeys, KeyImage, ());