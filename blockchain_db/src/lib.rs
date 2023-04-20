// Copyright (C) 2023 Cuprate Contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//!
//! blockchain_db crates:
//! Contains the implementation of interaction between the blockchain and the database backend.
//! There is actually only one storage engine available:
//! - RocksDB
//! There is two other storage engine planned:
//! - HSE (Heteregeonous Storage Engine)
//! - LMDB (like monerod)

#![deny(unused_attributes)]
#![forbid(unsafe_code)]
#![allow(non_camel_case_types)]
#![deny(clippy::expect_used, clippy::panic)]

use monero::{consensus::Encodable, util::ringct::RctSig, Block, BlockHeader, Hash, Transaction};
use std::{error::Error, ops::Range};
use thiserror::Error;

const MONERO_DEFAULT_LOG_CATEGORY: &str = "blockchain.db";

pub type difficulty_type = u128;
type Blobdata = Vec<u8>;
type BlobdataRef = [u8];
type TxOutIndex = (Hash, u64);

/// Methods tracking how a tx was received and relayed
pub enum RelayMethod {
    none,    //< Received via RPC with `do_not_relay` set
    local,   //< Received via RPC; trying to send over i2p/tor, etc.
    forward, //< Received over i2p/tor; timer delayed before ipv4/6 public broadcast
    stem,    //< Received/send over network using Dandelion++ stem
    fluff,   //< Received/sent over network using Dandelion++ fluff
    block,   //< Received in block, takes precedence over others
}

// the database types are going to be defined in the monero rust library.
pub enum RelayCategory {
    broadcasted, //< Public txes received via block/fluff
    relayable,   //< Every tx not marked `relay_method::none`
    legacy, //< `relay_category::broadcasted` + `relay_method::none` for rpc relay requests or historical reasons
    all,    //< Everything in the db
}

fn matches_category(relay_method: RelayMethod, relay_category: RelayCategory) -> bool {
    todo!()
}

/**
 * @brief a struct containing output metadata
 */
pub struct output_data_t {
    pubkey: monero::util::key::PublicKey, //< the output's public key (for spend verification)
    unlock_time: u64,                     //< the output's unlock time (or height)
    height: u64,                          //< the height of the block which created the output
    commitment: monero::util::ringct::Key, //< the output's amount commitment (for spend verification)
}

pub struct tx_data_t {
    tx_id: u64,
    unlock_time: u64,
    block_id: u64,
}

pub struct alt_block_data_t {
    height: u64,
    cumulative_weight: u64,
    cumulative_difficulty_low: u64,
    cumulative_difficulty_high: u64,
    already_generated_coins: u64,
}

pub struct txpool_tx_meta_t {
    max_used_block_id: monero::cryptonote::hash::Hash,
    last_failed_id: monero::cryptonote::hash::Hash,
    weight: u64,
    fee: u64,
    max_used_block_height: u64,
    last_failed_height: u64,
    receive_time: u64,
    last_relayed_time: u64, //< If received over i2p/tor, randomized forward time. If Dandelion++stem, randomized embargo time. Otherwise, last relayed timestamp
    // 112 bytes
    kept_by_block: u8,
    relayed: u8,
    do_not_relay: u8,
    double_spend_seen: u8,
    pruned: u8,
    is_local: u8,
    dandelionpp_stem: u8,
    is_forwarding: u8,
    bf_padding: u8,

    padding: [u8; 76],
}

impl txpool_tx_meta_t {
    fn set_relay_method(relay_method: RelayMethod) {}

    fn upgrade_relay_method(relay_method: RelayMethod) -> bool {
        todo!()
    }

    /// See `relay_category` description
    fn matches(category: RelayCategory) -> bool {
        return matches_category(todo!(), category);
    }
}

pub enum OBJECT_TYPE {
    BLOCK,
    BLOCK_BLOB,
    TRANSACTION,
    TRANSACTION_BLOB,
    OUTPUTS,
    TXPOOL,
}

#[non_exhaustive] // < to remove
#[allow(dead_code)] // < to remove
#[derive(Error, Debug)]
pub enum DB_FAILURES {
    #[error("DB_ERROR: `{0}`. The database is likely corrupted.")]
    DB_ERROR(String),
    #[error("DB_ERROR_TXN_START: `{0}`. The database failed starting a txn.")]
    DB_ERROR_TXN_START(String),
    #[error("DB_OPEN_FAILURE: Failed to open the database.")]
    DB_OPEN_FAILURE,
    #[error("DB_CREATE_FAILURE: Failed to create the database.")]
    DB_CREATE_FAILURE,
    #[error("DB_SYNC_FAILURE: Failed to sync the database.")]
    DB_SYNC_FAILURE,
    #[error("BLOCK_DNE: `{0}`. The block requested does not exist")]
    BLOCK_DNE(String),
    #[error("BLOCK_PARENT_DNE: `{0}` The parent of the block does not exist")]
    BLOCK_PARENT_DNE(String),
    #[error("BLOCK_EXISTS. The block to be added already exists!")]
    BLOCK_EXISTS,
    #[error("BLOCK_INVALID: `{0}`. The block to be added did not pass validation!")]
    BLOCK_INVALID(String),
    #[error("TX_EXISTS. The transaction to be added already exists!")]
    TX_EXISTS,
    #[error("TX_DNE: `{0}`. The transaction requested does not exist!")]
    TX_DNE(String),
    #[error("OUTPUTS_EXISTS. The output to be added already exists!")]
    OUTPUT_EXISTS,
    #[error("OUTPUT_DNE: `{0}`. The output requested does not exist")]
    OUTPUT_DNE(String),
    #[error("KEY_IMAGE_EXISTS. The spent key imge to be added already exists!")]
    KEY_IMAGE_EXISTS,

    #[error("ARITHMETIC_COUNT: `{0}`. An error occured due to a bad arithmetic/count logic")]
    ARITHEMTIC_COUNT(String),
    #[error("HASH_DNE. ")]
    HASH_DNE(Option<Hash>),
}

pub trait KeyValueDatabase {
    fn add_data_to_cf<D: ?Sized + Encodable>(cf: &str, data: &D) -> Result<Hash, DB_FAILURES>;
}

pub trait BlockchainDB: KeyValueDatabase {
    // supposed to be private

    // TODO: understand

    //fn remove_block() -> Result<(), DB_FAILURES>; useless as it just delete data from the table. pop_block that use it internally also use remove_transaction to literally delete the block. Can be implemented without

    fn add_spent_key() -> Result<(), DB_FAILURES>;

    fn remove_spent_key() -> Result<(), DB_FAILURES>;

    fn get_tx_amount_output_indices(tx_id: u64, n_txes: usize) -> Vec<Vec<u64>>;

    fn has_key_image(img: &monero::blockdata::transaction::KeyImage) -> bool;

    fn prune_outputs(amount: u64);

    fn get_blockchain_pruning_seed() -> u32;

    // variables part.
    //      uint64_t num_calls = 0;  //!< a performance metric
    //      uint64_t time_blk_hash = 0;  //!< a performance metric
    //      uint64_t time_add_block1 = 0;  //!< a performance metric
    //      uint64_t time_add_transaction = 0;  //!< a performance metric

    // supposed to be protected

    //      mutable uint64_t time_tx_exists = 0;  //!< a performance metric
    //      uint64_t time_commit1 = 0;  //!< a performance metric
    //      bool m_auto_remove_logs = true;  //!< whether or not to automatically remove old logs

    //      HardFork* m_hardfork;       |     protected: int *m_hardfork

    // bool m_open;
    // mutable epee::critical_section m_synchronization_lock;  //!< A lock, currently for when BlockchainLMDB needs to resize the backing db file

    // supposed to be public

    /* handled by the DB.
    fn batch_start(batch_num_blocks: u64, batch_bytes: u64) -> Result<bool,DB_FAILURES>;

    fn batch_abort() -> Result<(),DB_FAILURES>;

    fn set_batch_transactions() -> Result<(),DB_FAILURES>;

    fn block_wtxn_start();
    fn block_wtxn_stop();
    fn block_wtxn_abort();

    fn block_rtxn_start();
    fn block_rtxn_stop();
    fn block_rtxn_abort();
    */

    //fn set_hard_fork(); // (HardFork* hf)

    //fn add_block_public() -> Result<u64, DB_FAILURES>;

    //fn block_exists(h: monero::cryptonote::hash::Hash,  height: u64) -> bool;

    // fn tx_exists(h: monero::cryptonote::hash::Hash, tx_id: Option<u64>) -> Result<(),()>; // Maybe error should be DB_FAILURES, not specified in docs

    //fn tx_exist(h: monero::cryptonote::hash::Hash, tx: monero::Transaction) -> bool;

    //fn get_tx_blob(h: monero::cryptonote::hash::Hash, tx: String) -> bool;

    //fn get_pruned_tx_blob(h: monero::cryptonote::hash::Hash, tx: &mut String) -> bool;

    //fn update_pruning() -> bool;

    //fn check_pruning() -> bool;

    //fn get_max_block_size() -> u64; It is never used

    //fn add_max_block_size() -> u64; For reason above

    //fn for_all_txpool_txes(wat: fn(wat1: &monero::Hash, wat2: &txpool_tx_meta_t, wat3: &String) -> bool, include_blob: bool, category: RelayCategory) -> Result<bool,DB_FAILURES>;

    //fn for_all_keys_images(wat: fn(ki: &monero::blockdata::transaction::KeyImage) -> bool) -> Result<bool,DB_FAILURES>;

    //fn for_blocks_range(h1: &u64, h2: &u64, wat: fn(u: u64, h: &monero::Hash, blk: &Block) -> bool) -> Result<bool,DB_FAILURES>; // u: u64 should be mut u: u64

    //fn for_all_transactions(wat: fn(h: &monero::Hash, tx: &monero::Transaction) -> bool, pruned: bool) -> Result<bool,DB_FAILURES>;

    //fn for_all_outputs();

    //fn for_all_alt_blocks();

    // ------------------------------------------|  Blockchain  |------------------------------------------------------------

    /// `height` fetch the current blockchain height.
    ///
    /// Return the current blockchain height. In case of failures, a DB_FAILURES will be return.
    ///
    /// No parameters is required.
    fn height(&mut self) -> Result<u64, DB_FAILURES>;

    /// `set_hard_fork_version` sets which hardfork version a height is on.
    ///
    /// In case of failures, a `DB_FAILURES` will be return.
    ///
    /// Parameters:
    /// `height`: is the height where the hard fork happen.
    /// `version`: is the version of the hard fork.
    fn set_hard_fork_version(&mut self);

    /// `get_hard_fork_version` checks which hardfork version a height is on.
    ///
    /// In case of failures, a `DB_FAILURES` will be return.
    ///
    /// Parameters:
    /// `height:` is the height to check.
    fn get_hard_fork_version(&mut self);

    /// May not need to be used
    fn fixup(&mut self);

    // -------------------------------------------|  Outputs  |------------------------------------------------------------

    /// `add_output` add an output data to it's storage .
    ///
    /// It internally keep track of the global output count. The global output count is also used to index outputs based on
    /// their order of creations.
    ///
    /// Should return the amount output index. In case of failures, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `tx_hash`: is the hash of the transaction where the output comes from.
    /// `output`: is the output to store.
    /// `index`: is the local output's index (from transaction).
    /// `unlock_time`: is the unlock time (height) of the output.
    /// `commitment`: is the RingCT commitment of this output.
    fn add_output(
        &mut self,
        tx_hash: &Hash,
        output: &Hash,
        index: TxOutIndex,
        unlock_time: u64,
        commitment: RctSig,
    ) -> Result<u64, DB_FAILURES>;

    /// `add_tx_amount_output_indices` store amount output indices for a tx's outputs
    ///
    /// TODO
    fn add_tx_amount_output_indices() -> Result<(), DB_FAILURES>;

    /// `get_output_key` get some of an output's data
    ///
    /// Return the public key, unlock time, and block height for the output with the given amount and index, collected in a struct
    /// In case of failures, a `DB_FAILURES` will be return. Precisely, if the output cannot be found, an `OUTPUT_DNE` error will be return.
    /// If any of the required part for the final struct isn't found, a `DB_ERROR` will be return
    ///
    /// Parameters:
    /// `amount`: is the corresponding amount of the output
    /// `index`: is the output's index (indexed by amount)
    /// `include_commitment` : `true` by default.
    fn get_output_key(
        &mut self,
        amount: u64,
        index: u64,
        include_commitmemt: bool,
    ) -> Result<output_data_t, DB_FAILURES>;

    /// `get_output_tx_and_index_from_global`gets an output's transaction hash and index from output's global index.
    ///
    /// Return a tuple containing the transaction hash and the output index. In case of failures, a `DB_FAILURES` will be return.
    ///
    /// Parameters:
    /// `index`: is the output's global index.
    fn get_output_tx_and_index_from_global(
        &mut self,
        index: u64,
    ) -> Result<TxOutIndex, DB_FAILURES>;

    /// `get_output_key_list` gets outputs' metadata from a corresponding collection.
    ///
    /// Return a collection of output's metadata. In case of failurse, a `DB_FAILURES` will be return.
    ///
    /// Parameters:
    /// `amounts`: is the collection of amounts corresponding to the requested outputs.
    /// `offsets`: is a collection of outputs' index (indexed by amount).
    /// `allow partial`: `false` by default.
    fn get_output_key_list(
        &mut self,
        amounts: &Vec<u64>,
        offsets: &Vec<u64>,
        allow_partial: bool,
    ) -> Result<Vec<output_data_t>, DB_FAILURES>;

    /// `get_output_tx_and_index` gets an output's transaction hash and index
    ///
    /// Return a tuple containing the transaction hash and the output index. In case of failures, a `DB_FAILURES` will be return.
    ///
    /// Parameters:
    /// `amount`: is the corresponding amount of the output
    /// `index`: is the output's index (indexed by amount)
    fn get_output_tx_and_index(
        &mut self,
        amount: u64,
        index: u64,
    ) -> Result<TxOutIndex, DB_FAILURES>;

    /// `get_num_outputs` fetches the number of outputs of a given amount.
    ///
    /// Return a count of outputs of the given amount. in case of failures a `DB_FAILURES` will be return.
    ///
    /// Parameters:
    /// `amount`: is the output amount being looked up.
    fn get_num_outputs(amount: &u64) -> Result<u64, DB_FAILURES>;

    // -----------------------------------------| Transactions |----------------------------------------------------------

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
    fn add_transaction(
        &mut self,
        blk_hash: &Hash,
        tx: Transaction,
        tx_hash: &Hash,
        tx_prunable_hash_ptr: &Hash,
    ) -> Result<(), DB_FAILURES>;

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
    fn add_transaction_data(
        &mut self,
        blk_hash: &Hash,
        tx_and_hash: (Transaction, &Hash),
        tx_prunable_hash: &Hash,
    ) -> Result<Hash, DB_FAILURES>;

    /// `remove_transaction_data` remove data about a transaction specified by its hash.
    ///
    /// In case of failures, a `DB_FAILURES` will be return. Precisely, a `TX_DNE` will be return if the specified transaction can't be found.
    ///
    /// Parameters:
    /// `tx_hash`: is the transaction's hash to remove data from.
    fn remove_transaction_data(&mut self, tx_hash: &Hash) -> Result<(), DB_FAILURES>;

    /// `get_tx_count` fetches the total number of transactions stored in the database
    ///
    /// Should return the count. In case of failure, a DB_FAILURES will be return.
    ///
    /// No parameters is required.
    fn get_tx_count(&mut self) -> Result<u64, DB_FAILURES>;

    /// `tx_exists` check if a transaction exist with the given hash.
    ///
    /// Return `true` if the transaction exist, `false` otherwise. In case of failure, a DB_FAILURES will be return.
    ///
    /// Parameters :
    /// `h` is the given hash of transaction to check.
    ///  `tx_id` is an optional mutable reference to get the transaction id out of the found transaction.
    fn tx_exists(&mut self, h: &Hash, tx_id: &mut Option<u64>) -> Result<bool, DB_FAILURES>;

    /// `get_tx_unlock_time` fetch a transaction's unlock time/height
    ///
    /// Should return the unlock time/height in u64. In case of failure, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `h`: is the given hash of the transaction to check.
    fn get_tx_unlock_time(&mut self, h: &Hash) -> Result<u64, DB_FAILURES>;

    /// `get_tx` fetches the transaction with the given hash.
    ///
    /// Should return the transaction. In case of failure, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `h`: is the given hash of transaction to fetch.
    fn get_tx(&mut self, h: &Hash) -> Result<Transaction, DB_FAILURES>;

    /// `get_pruned_tx` fetches the transaction base with the given hash.
    ///
    /// Should return the transaction. In case of failure, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `h`: is the given hash of transaction to fetch.
    fn get_pruned_tx(&mut self, h: &Hash) -> Result<Transaction, DB_FAILURES>;

    /// `get_tx_list` fetches the transactions with given hashes.
    ///
    /// Should return a vector with the requested transactions. In case of failures, a DB_FAILURES will be return.
    /// Precisly, a HASH_DNE error will be returned with the correspondig hash of transaction that is not found in the DB.
    ///
    /// `hlist`: is the given collection of hashes correspondig to the transactions to fetch.
    fn get_tx_list(&mut self, hlist: &Vec<Hash>) -> Result<Vec<monero::Transaction>, DB_FAILURES>;

    /// `get_tx_blob` fetches the transaction blob with the given hash.
    ///
    ///  Should return the transaction blob. In case of failure, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `h`: is the given hash of the transaction to fetch.
    fn get_tx_blob(&mut self, h: &Hash) -> Result<Blobdata, DB_FAILURES>;

    /// `get_pruned_tx_blob` fetches the pruned transaction blob with the given hash.
    ///
    ///  Should return the transaction blob. In case of failure, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `h`: is the given hash of the transaction to fetch.
    fn get_pruned_tx_blob(&mut self, h: &Hash) -> Result<Blobdata, DB_FAILURES>;

    /// `get_prunable_tx_blob` fetches the prunable transaction blob with the given hash.
    ///
    /// Should return the transaction blob, In case of failure, a DB_FAILURES, will be return.
    ///
    /// Parameters:
    /// `h`: is the given hash of the transaction to fetch.
    fn get_prunable_tx_blob(&mut self, h: &Hash) -> Result<Blobdata, DB_FAILURES>;

    /// `get_prunable_tx_hash` fetches the prunable transaction hash
    ///
    /// Should return the hash of the prunable transaction data. In case of failures, a DB_FAILURES, will be return.
    ///
    /// Parameters:
    /// `tx_hash`: is the given hash of the transaction  to fetch.
    fn get_prunable_tx_hash(&mut self, tx_hash: &Hash) -> Result<Hash, DB_FAILURES>;

    /// `get_pruned_tx_blobs_from` fetches a number of pruned transaction blob from the given hash, in canonical blockchain order.
    ///
    /// Should return the pruned transactions stored from the one with the given hash. In case of failure, a DB_FAILURES will be return.
    /// Precisly, an ARITHMETIC_COUNT error will be returned if the first transaction does not exist or their are fewer transactions than the count.
    ///
    /// Parameters:
    /// `h`: is the given hash of the first transaction/
    /// `count`: is the number of transaction to fetch in canoncial blockchain order.
    fn get_pruned_tx_blobs_from(
        &mut self,
        h: &Hash,
        count: usize,
    ) -> Result<Vec<Blobdata>, DB_FAILURES>;

    /// `get_tx_block_height` fetches the height of a transaction's block
    ///
    /// Should return the height of the block containing the transaction with the given hash. In case
    /// of failures, a DB FAILURES will be return. Precisely, a TX_DNE error will be return if the transaction cannot be found.
    ///
    /// Parameters:
    /// `h`: is the fiven hash of the first transaction
    fn get_tx_block_height(&mut self, h: &Hash) -> Result<u64, DB_FAILURES>;

    // -----------------------------------------|  Blocks  |----------------------------------------------------------

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
    fn add_block(
        blk: Block,
        blk_hash: Hash,
        block_weight: u64,
        long_term_block_weight: u64,
        cumulative_difficulty: u128,
        coins_generated: u64,
    ) -> Result<(), DB_FAILURES>;

    /// `pop_block` pops the top block off the blockchain.
    ///
    /// Return the block that was popped. In case of failures, a `DB_FAILURES` will be return.
    ///
    /// No parameters is required.
    fn pop_block(&mut self) -> Result<Block, DB_FAILURES>;

    /// `blocks_exists` check if the given block exists
    ///
    /// Return `true` if the block exist, `false` otherwise. In case of failures, a `DB_FAILURES` will be return.
    ///
    /// Parameters:
    /// `h`: is the given hash of the requested block.
    fn block_exists(&mut self, h: &Hash) -> Result<bool, DB_FAILURES>;

    /// `get_block` fetches the block with the given hash.
    ///
    /// Return the requested block. In case of failures, a `DB_FAILURES` will be return. Precisely, a `BLOCK_DNE`
    /// error will be returned if the requested block can't be found.
    ///
    /// Parameters:
    /// `h`: is the given hash of the requested block.
    fn get_block(&mut self, h: &Hash) -> Result<Block, DB_FAILURES>;

    /// `get_block_from_height` fetches the block located at the given height.
    ///
    /// Return the requested block. In case of failures, a `DB_FAILURES` will be return. Precisely, a `BLOCK_DNE`
    /// error will be returned if the requested block can't be found.
    ///
    /// Parameters:
    /// `height`: is the height where the requested block is located.
    fn get_block_from_height(&mut self, height: u64) -> Result<Block, DB_FAILURES>;

    /// `get_block_from_range` fetches the blocks located from and to the specified heights.
    ///
    /// Return the requested blocks. In case of failures, a `DB_FAILURES` will be return. Precisely, a `BLOCK_DNE`
    /// error will be returned if at least requested block can't be found. If the range requested past the end of the blockchain,
    /// an `ARITHMETIC_COUNT` error will be return.
    ///
    /// Parameters:
    /// `height_range`: is the range of height where the requested blocks are located.
    fn get_blocks_from_range(
        &mut self,
        height_range: Range<u64>,
    ) -> Result<Vec<Block>, DB_FAILURES>;

    /// `get_block_blob` fetches the block blob with the given hash.
    ///
    /// Return the requested block blob. In case of failures, a `DB_FAILURES` will be return. Precisely, a `BLOCK_DNE`
    /// error will be returned if the requested block can't be found.
    ///
    /// Parameters:
    /// `h`: is the given hash of the requested block.
    fn get_block_blob(&mut self, h: &Hash) -> Result<Blobdata, DB_FAILURES>;

    /// `get_block_blob_from_height` fetches the block blob located at the given height in the blockchain.
    ///
    /// Return the requested block blob. In case of failures, a `DB_FAILURES` will be return. Precisely, a `BLOCK_DNE`
    /// error will be returned if the requested block can't be found.
    ///
    /// Parameters:
    /// `height`: is the given height of the corresponding block blob to fetch.
    fn get_block_blob_from_height(&mut self, height: u64) -> Result<Blobdata, DB_FAILURES>;

    /// `get_block_header` fetches the block's header with the given hash.
    ///
    /// Return the requested block header. In case of failures, a `DB_FAILURES` will be return. Precisely, a `BLOCK_DNE`
    /// error will be returned if the requested block can't be found.
    ///
    /// Parameters:
    /// `h`: is the given hash of the requested block.
    fn get_block_header(&mut self, h: &Hash) -> Result<BlockHeader, DB_FAILURES>;

    /// `get_block_hash_from_height` fetch block's hash located at the given height.
    ///
    /// Return the hash of the block with the given height. In case of failures, a DB_FAILURES will be return. Precisely, a `BLOCK_DNE`
    /// error will be returned if the requested block can't be found.
    ///
    /// Parameters:
    /// `height`: is the given height where the requested block is located.
    fn get_block_hash_from_height(&mut self, height: u64) -> Result<Hash, DB_FAILURES>;

    /// `get_blocks_hashes_from_range` fetch blocks' hashes located from, between and to the given heights.
    ///
    /// Return a collection of hases corresponding to the scoped blocks. In case of failures, a DB_FAILURES will be return. Precisely, a `BLOCK_DNE`
    /// error will be returned if at least one of the requested blocks can't be found.
    ///
    /// Parameters:
    /// `height`: is the given height where the requested block is located.
    fn get_blocks_hashes_from_range(&mut self, range: Range<u64>)
        -> Result<Vec<Hash>, DB_FAILURES>;

    /// `get_top_block` fetch the last/top block of the blockchain
    ///
    /// Return the last/top block of the blockchain. In case of failures, a DB_FAILURES, will be return.
    ///
    /// No parameters is required.
    fn get_top_block(&mut self) -> Block;

    /// `get_top_block_hash` fetch the block's hash located at the top of the blockchain (the last one).
    ///
    /// Return the hash of the last block. In case of failures, a DB_FAILURES will be return.
    ///
    /// No parameters is required
    fn get_top_block_hash(&mut self) -> Result<Hash, DB_FAILURES>;

    // ! TODO: redefine the  result & docs. see what could be improved. Do we really need this function?
    /// `get_blocks_from` fetches a variable number of blocks and transactions from the given height, in canonical blockchain order as long as it meets the parameters.
    ///
    /// Should return the blocks stored starting from the given height. The number of blocks returned is variable, based on the max_size defined. There will be at least `min_block_count`
    /// if possible, even if this contravenes max_tx_count. In case of failures, a `DB_FAILURES` error will be return.
    ///
    /// Parameters:
    /// `start_height`: is the given height to start from.    
    /// `min_block_count`: is the minimum number of blocks to return. If there are fewer blocks, it'll return fewer blocks than the minimum.    
    /// `max_block_count`: is the maximum number of blocks to return.    
    /// `max_size`: is the maximum size of block/transaction data to return (can be exceeded on time if min_count is met).    
    /// `max_tx_count`: is the maximum number of txes to return.    
    /// `pruned`: is whether to return full or pruned tx data.    
    /// `skip_coinbase`: is whether to return or skip coinbase transactions (they're in blocks regardless).    
    /// `get_miner_tx_hash`: is whether to calculate and return the miner (coinbase) tx hash.    
    fn get_blocks_from(
        &mut self,
        start_height: u64,
        min_block_count: u64,
        max_block_count: u64,
        max_size: usize,
        max_tx_count: u64,
        pruned: bool,
        skip_coinbase: bool,
        get_miner_tx_hash: bool,
    ) -> Result<Vec<((String, Hash), Vec<(Hash, String)>)>, DB_FAILURES>;

    /// `get_block_height` gets the height of the block with a given hash
    ///
    /// Return the requested height.
    fn get_block_height(&mut self, h: &Hash) -> Result<u64, DB_FAILURES>;

    /// `get_block_weights` fetch the block's weight located at the given height.
    ///
    /// Return the requested block weight. In case of failures, a `DB_FAILURES` will be return. Precisely, a `BLOCK_DNE`
    /// error will be returned if the requested block can't be found.
    ///
    /// Parameters:
    /// `height`: is the given height where the requested block is located.
    fn get_block_weight(&mut self, height: u64) -> Result<u64, DB_FAILURES>;

    /// `get_block_weights` fetch the last `count` blocks' weights.
    ///
    /// Return a collection of weights. In case of failures, a `DB_FAILURES` will be return. Precisely, an 'ARITHMETIC_COUNT'
    /// error will be returned if there are fewer than `count` blocks.
    ///
    /// Parameters:
    /// `start_height`: is the height to seek before collecting block weights.
    /// `count`: is the number of last blocks' weight to fetch.
    fn get_block_weights(
        &mut self,
        start_height: u64,
        count: usize,
    ) -> Result<Vec<u64>, DB_FAILURES>;

    /// `get_block_already_generated_coins` fetch a block's already generated coins
    ///
    /// Return the total coins generated as of the block with the given height. In case of failures, a `DB_FAILURES` will be return. Precisely, a `BLOCK_DNE`
    /// error will be returned if the requested block can't be found.
    ///
    /// Parameters:
    /// `height`: is the given height of the block to seek.
    fn get_block_already_generated_coins(&mut self, height: u64) -> Result<u64, DB_FAILURES>;

    /// `get_block_long_term_weight` fetch a block's long term weight.
    ///
    /// Should return block's long term weight. In case of failures, a DB_FAILURES will be return. Precisely, a `BLOCK_DNE`
    /// error will be returned if the requested block can't be found.
    ///
    /// Parameters:
    /// `height`: is the given height where the requested block is located.
    fn get_block_long_term_weight(&mut self, height: u64) -> Result<u64, DB_FAILURES>;

    /// `get_long_term_block_weights` fetch the last `count` blocks' long term weights
    ///
    /// Should return a collection of blocks' long term weights. In case of failures, a DB_FAILURES will be return. Precisely, a `BLOCK_DNE`
    /// error will be returned if the requested block can't be found. If there are fewer than `count` blocks, the returned collection will be
    /// smaller than `count`.
    ///
    /// Parameters:
    /// `start_height`: is the height to seek before collecting block weights.
    /// `count`: is the number of last blocks' long term weight to fetch.
    fn get_long_term_block_weights(
        &mut self,
        height: u64,
        count: usize,
    ) -> Result<Vec<u64>, DB_FAILURES>;

    /// `get_block_timestamp` fetch a block's timestamp.
    ///
    /// Should return the timestamp of the block with given height. In case of failures, a DB_FAILURES will be return. Precisely, a `BLOCK_DNE`
    /// error will be returned if the requested block can't be found.
    ///
    /// Parameters:
    /// `height`: is the given height where the requested block to fetch timestamp is located.
    fn get_block_timestamp(&mut self, height: u64) -> Result<u64, DB_FAILURES>;

    /// `get_block_cumulative_rct_outputs` fetch a blocks' cumulative number of RingCT outputs
    ///
    /// Should return the number of RingCT outputs in the blockchain up to the blocks located at the given heights. In case of failures, a DB_FAILURES will be return. Precisely, a `BLOCK_DNE`
    /// error will be returned if the requested block can't be found.
    ///
    /// Parameters:
    /// `heights`: is the collection of height to check for RingCT distribution.
    fn get_block_cumulative_rct_outputs(
        &mut self,
        heights: Vec<u64>,
    ) -> Result<Vec<u64>, DB_FAILURES>;

    /// `get_top_block_timestamp` fetch the top block's timestamp
    ///
    /// Should reutnr the timestamp of the most recent block. In case of failures, a DB_FAILURES will be return.
    ///
    /// No parameters is required.
    fn get_top_block_timestamp(&mut self) -> Result<u64, DB_FAILURES>;

    /// `correct_block_cumulative_difficulties` correct blocks cumulative difficulties that were incorrectly calculated due to the 'difficulty drift' bug
    ///
    /// Should return nothing. In case of failures, a DB_FAILURES will be return. Precisely, a `BLOCK_DNE`
    /// error will be returned if the requested block can't be found.
    ///
    /// Parameters:
    /// `start_height`: is the height of the block where the drifts start.
    /// `new_cumulative_difficulties`: is the collection of new cumulative difficulties to be stored
    fn correct_block_cumulative_difficulties(
        &mut self,
        start_height: u64,
        new_cumulative_difficulties: Vec<difficulty_type>,
    ) -> Result<(), DB_FAILURES>;

    // --------------------------------------------|  Alt-Block  |------------------------------------------------------------

    /// `add_alt_block` add a new alternative block.
    ///
    /// In case of failures, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// blkid: is the hash of the original block
    /// data: is the metadata for the block
    /// blob: is the blobdata of this alternative block.
    fn add_alt_block(
        &mut self,
        blkid: &Hash,
        data: &alt_block_data_t,
        blob: &Blobdata,
    ) -> Result<(), DB_FAILURES>;

    /// `get_alt_block` gets the specified alternative block.
    ///
    /// Return a tuple containing the blobdata of the alternative block and its metadata. In case of failures, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `blkid`: is the hash of the requested alternative block.
    fn get_alt_block(&mut self, blkid: &Hash) -> Result<(alt_block_data_t, Blobdata), DB_FAILURES>;

    /// `remove_alt_block` remove the specified alternative block
    ///
    /// In case of failures, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `blkid`: is the hash of the alternative block to remove.
    fn remove_alt_block(&mut self, blkid: &Hash) -> Result<(), DB_FAILURES>;

    /// `get_alt_block` gets the total number of alternative blocks stored
    ///
    /// In case of failures, a DB_FAILURES will be return.
    ///
    /// No parameters is required.
    fn get_alt_block_count(&mut self) -> Result<u64, DB_FAILURES>;

    /// `drop_alt_block` drop all alternative blocks.
    ///
    /// In case of failures, a DB_FAILURES will be return.
    ///
    /// No parameters is required.
    fn drop_alt_blocks(&mut self) -> Result<(), DB_FAILURES>;

    // --------------------------------------------|  TxPool  |------------------------------------------------------------

    /// `add_txpool_tx` add a Pool's transaction to the database.
    ///
    /// In case of failures, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `txid`: is the hash of the transaction to add.
    /// `blob`: is the blobdata of the transaction to add.
    /// `details`: is the metadata of the transaction pool at this specific transaction.
    fn add_txpool_tx(
        &mut self,
        txid: &Hash,
        blob: &BlobdataRef,
        details: &txpool_tx_meta_t,
    ) -> Result<(), DB_FAILURES>;

    /// `update_txpool_tx` replace pool's transaction metadata.
    ///
    /// In case of failures, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `txid`: is the hash of the transaction to edit
    /// `details`: is the new metadata to insert.
    fn update_txpool_tx(
        &mut self,
        txid: &monero::Hash,
        details: &txpool_tx_meta_t,
    ) -> Result<(), DB_FAILURES>;

    /// `get_txpool_tx_count` gets the number of transactions in the txpool.
    ///
    /// Return the number of transaction in the txpool. In case of failures, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `tx_category`: is the relay's category where the tx are coming from. (RelayCategory::broadcasted by default)
    fn get_txpool_tx_count(&mut self, tx_category: RelayCategory) -> Result<u64, DB_FAILURES>;

    /// `txpool_has_tx`checks if the specified transaction exist in the transaction's pool and if it belongs
    /// to the specified category.
    ///
    /// Return `true` if the condition above are met, `false otherwise`. In case of failures, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `txid`: is the hash of the transaction to check for
    /// `tx_category`: is the relay's category where the tx is supposed to come from.
    fn txpool_has_tx(
        &mut self,
        txid: &Hash,
        tx_category: &RelayCategory,
    ) -> Result<bool, DB_FAILURES>;

    /// `remove_txpool_tx` remove the specified transaction from the transaction pool.
    ///
    /// In case of failures, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `txid`: is the hash of the transaction to remove.
    fn remove_txpool_tx(&mut self, txid: &Hash) -> Result<(), DB_FAILURES>;

    /// `get_txpool_tx_meta` gets transaction's pool metadata recorded at the specified transaction.
    ///
    /// In case of failures, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `txid`: is the hash of metadata's transaction hash.
    fn get_txpool_tx_meta(&mut self, txid: &Hash) -> Result<txpool_tx_meta_t, DB_FAILURES>;

    /// `get_txpool_tx_blob` gets the txpool transaction's blob.
    ///
    /// In case of failures, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `txid`: is the hash of the transaction to fetch blobdata from.
    /// `tx_category`: is the relay's category where the tx are coming from. < monerod note: for filtering out hidden/private txes.
    fn get_txpool_tx_blob(
        &mut self,
        txid: &Hash,
        tx_category: RelayCategory,
    ) -> Result<Blobdata, DB_FAILURES>;

    /// `txpool_tx_matches_category` checks if the corresponding transaction belongs to the specified category.
    ///
    /// Return `true` if the transaction belongs to the category, `false` otherwise. In case of failures, a DB_FAILURES will be return.
    ///
    /// Parameters:
    /// `tx_hash`: is the hash of the transaction to lookup.
    /// `category`: is the relay's category to check.
    fn txpool_tx_matches_category(
        &mut self,
        tx_hash: &Hash,
        category: RelayCategory,
    ) -> Result<bool, DB_FAILURES>;
}

// functions defined as useless : init_options(), is_open(), reset_stats(), show_stats(), open(), close(), get_output_histogram(), safesyncmode, get_filenames(), get_db_name(), remove_data_file(), lock(), unlock(), is_read_only(), get_database_size(), get_output_distribution(), set_auto_remove_logs(), check_hard_fork_info(), drop_hard_fork_info(), get_indexing_base();
