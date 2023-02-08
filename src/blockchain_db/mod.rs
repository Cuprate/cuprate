use monero::{Hash, Transaction};

use crate::{cryptonote_protocol::enums::{RelayMethod}, cryptonote_basic::difficulty::difficulty_type, blockchain_db::{}};
use std::{error::Error, ops::Range};

const MONERO_DEFAULT_LOG_CATEGORY: &str = "blockchain.db";

type Blobdata = Vec<u8>;
type TxOutIndex = (monero::cryptonote::hash::Hash, u64);

pub(in super) enum RelayCategory {
        broadcasted,                                                                        //< Public txes received via block/fluff
        relayable,                                                                               //< Every tx not marked `relay_method::none`
        legacy,                                                                                     //< `relay_category::broadcasted` + `relay_method::none` for rpc relay requests or historical reasons
        all                                                                                              //< Everything in the db
}

fn matches_category(relay_method: RelayMethod, relay_category: RelayCategory) -> bool {
        todo!()
}

/**
 * @brief a struct containing output metadata
 */
pub struct output_data_t {
        pubkey: monero::util::key::PublicKey,                           //< the output's public key (for spend verification)
        unlock_time: u64,                                                                 //< the output's unlock time (or height)
        height: u64,                                                                             //< the height of the block which created the output
        commitment: monero::util::ringct::Key,                       //< the output's amount commitment (for spend verification)
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
        last_relayed_time: u64,                 //< If received over i2p/tor, randomized forward time. If Dandelion++stem, randomized embargo time. Otherwise, last relayed timestamp
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
        fn set_relay_method(relay_method: RelayMethod) {
                
        }

        fn upgrade_relay_method(relay_method: RelayMethod) -> bool {
                todo!()
        }

        /// See `relay_category` description
        fn matches(category: RelayCategory) -> bool {
                return matches_category(todo!(), category)
        }
}




/// TODO : String -> Vec<u8> they are cryptonote::blobdata

#[derive(Debug)]
pub enum DB_FAILURES {
        DB_ERROR,
        DB_EXCEPTION,
        BLOCK_DNE,
        OUTPUT_DNE,
        TX_DNE,
        ARITHEMTIC_COUNT,
        HASH_DNE(Option<Hash>),
}

pub(in super) trait BlockchainDB {

        // supposed to be private

        fn add_block() -> Result<(), DB_FAILURES>;

        fn remove_block() -> Result<(), DB_FAILURES>;

        fn add_transaction_data() -> Result<u64, DB_FAILURES>;

        fn remove_transaction_data() -> Result<(),DB_FAILURES>;

        fn add_output() -> Result<u64, DB_FAILURES>;

        fn add_tx_amount_output_indices() -> Result<(), DB_FAILURES>;

        fn add_spent_key() -> Result<(), DB_FAILURES>;

        fn remove_spent_key() -> Result<(),DB_FAILURES>;

        fn pop_block();

        fn remove_transaction();

        //      uint64_t num_calls = 0;  //!< a performance metric
        //      uint64_t time_blk_hash = 0;  //!< a performance metric
        //      uint64_t time_add_block1 = 0;  //!< a performance metric
        //      uint64_t time_add_transaction = 0;  //!< a performance metric

        // supposed to be protected

        fn add_transaction();

        //      mutable uint64_t time_tx_exists = 0;  //!< a performance metric
        //      uint64_t time_commit1 = 0;  //!< a performance metric
        //      bool m_auto_remove_logs = true;  //!< whether or not to automatically remove old logs

        //      HardFork* m_hardfork;       |     protected: int *m_hardfork

        // supposed to be public

        // a constructor maybe
        fn a_constructor_maybe() -> Self where Self: Sized;

        // a destructor maybe
        fn well_i_really_dont_know_here();

        fn init_options();

        fn reset_stats();

        fn show_stats();

        fn open() -> Result<(),DB_FAILURES>;

        fn is_open() -> bool;

        fn close() -> Result<(),DB_FAILURES>;

        fn sync() -> Result<(),DB_FAILURES>;

        fn safesyncmode(onoff: &'static bool) -> Result<(),DB_FAILURES>;

        fn reset() -> Result<(),DB_FAILURES>;




        // Primarly used by unit tests
        fn get_filenames() -> Vec<String>;

        fn get_db_name() -> String;

        // Reset the database (used only fore core tests, functional tests, etc)
        fn remove_data_file(folder: &'static String) -> bool; // may not be static

        fn lock() -> Result<bool,DB_FAILURES>;

        fn unlock() -> Result<bool,DB_FAILURES>;



        fn batch_start(batch_num_blocks: u64, batch_bytes: u64) -> Result<bool,DB_FAILURES>;

        fn batch_abort() -> Result<(),DB_FAILURES>;

        fn set_batch_transactions() -> Result<(),DB_FAILURES>;

        fn block_wtxn_start();
        fn block_wtxn_stop();
        fn block_wtxn_abort();

        fn block_rtxn_start();
        fn block_rtxn_stop();
        fn block_rtxn_abort();

        fn set_hard_fork(); // (HardFork* hf)

        fn add_block_public() -> Result<u64, DB_FAILURES>;

        //fn block_exists(h: monero::cryptonote::hash::Hash,  height: u64) -> bool;

        

        fn get_block_cumulative_rct_outputs(heights: Vec<u64>) -> Result<Vec<u64>,DB_FAILURES>;

        fn get_top_block_timestamp() -> Result<u64,DB_FAILURES>;

        

        

        fn correct_block_cumulative_difficulties(start_height: u64, new_cumulative_difficulties: Vec<difficulty_type>) -> Result<(),DB_FAILURES>;

        

        fn get_blocks_range(h1: u64, h2: u64) -> Result<Vec<monero::Block>, DB_FAILURES>;

        fn get_hashes_range(h1: u64, h2: u64) -> Result<Vec<monero::cryptonote::hash::Hash>,DB_FAILURES>;

        fn top_block_hash(block_height: u64) -> monero::cryptonote::hash::Hash;

        fn get_top_block() -> monero::Block;

        fn height() -> u64;

        // TODO idk never done a todo in my life
        // Gonna use this as an excuse to split this hell one more time
        //. a dot. because dot. is cool.

        fn pop_block_public(blk: &mut monero::Block, txs: Vec<monero::Transaction>);

        // fn tx_exists(h: monero::cryptonote::hash::Hash, tx_id: Option<u64>) -> Result<(),()>; // Maybe error should be DB_FAILURES, not specified in docs

        // started renaming

        // issues. They want to use mut reference to edit the hash if it exist or not. | arg is return and return in Result
        // Confirmed. These are indirect functions that take return their results through mutable references and report success with a boolean.
        // Seems important to refactor this

        //fn tx_exist(h: monero::cryptonote::hash::Hash, tx: monero::Transaction) -> bool;
        
        

        //fn get_tx_blob(h: monero::cryptonote::hash::Hash, tx: String) -> bool;

        //fn get_pruned_tx_blob(h: monero::cryptonote::hash::Hash, tx: &mut String) -> bool;

        

        

        

        

        

        fn get_num_outputs(amount: &u64) -> u64;

        // Should be hidden but it isn't????
        fn get_indexing_base() -> u64 { return 0;}



        // FINNALY SOME BLOCKCHAIN_DB.H STUFF OUT THERE!!!

        fn get_output_key(amount: &u64, index: &u64, include_commitmemt: bool) -> Result<output_data_t, DB_FAILURES>;

        fn get_output_tx_and_index_from_global(index: &u64) -> TxOutIndex;

        fn get_output_tx_and_index(amount: &u64, index: &u64,) -> TxOutIndex;

        fn get_output_tx_and_index_void(amount: &u64, offsets: &Vec<u64>, indices: &Vec<TxOutIndex>);

        fn get_output_key_void(amounts: &Vec<u64>, offsets: &Vec<u64>, outputs: &Vec<output_data_t>, allow_partial: bool); // wtf is std::span|epee::span. I see no difference with a Rust vector, also allow_partial is be default false

        // LMAO WTF IS CAN_THREAD_BULK_INDICES

        fn get_tx_amount_output_indices(tx_id: u64, n_txes: usize) -> Vec<Vec<u64>>;

        fn has_key_image(img: &monero::blockdata::transaction::KeyImage) -> bool;

        fn add_txpool_tx(txid: &monero::cryptonote::hash::Hash, blob: &Vec<u8>, details: &txpool_tx_meta_t);

        fn update_txpool_tx(txid: &monero::Hash, details: &txpool_tx_meta_t);

        fn get_txpool_tx_count(tx_category: RelayCategory) -> u64;

        fn txpool_has_tx(txid: &monero::Hash, tx_category: &RelayCategory) -> bool;

        fn remove_txpool_tx(txid: &monero::Hash);

        fn get_txpool_tx_meta(txid: &monero::Hash, meta: &txpool_tx_meta_t) -> bool;

        fn get_txpool_tx_blob(txid: &monero::Hash, bd: &mut Vec<u8>, tx_category: RelayCategory) -> bool;

        fn get_txpool_tx_blob_but_blob(txid: &monero::Hash, tx_category: RelayCategory) -> Vec<u8>;

        fn txpool_tx_matches_category(tx_hash: &monero::Hash, category: RelayCategory) -> bool;

        fn prune_outputs(amount: u64);

        // Wtf no more arguments

        fn get_blockchain_pruning_seed() -> u32;

        fn update_pruning() -> bool;

        fn check_pruning() -> bool;

        fn get_max_block_size() -> u64;

        fn add_max_block_size() -> u64;



        fn add_alt_block(blkid: &monero::Hash, data: &alt_block_data_t, blob: &Vec<u8>);

        fn get_alt_block(blkid: &monero::Hash, data: *mut alt_block_data_t, blob: *mut Vec<u8>) -> bool;

        fn remove_alt_block(blkid: &monero::Hash);

        fn get_alt_block_count() -> u64;

        fn drop_alt_blocks();

        fn for_all_txpool_txes(wat: fn(wat1: &monero::Hash, wat2: &txpool_tx_meta_t, wat3: &String) -> bool, include_blob: bool, category: RelayCategory) -> Result<bool,DB_FAILURES>;

        fn for_all_keys_images(wat: fn(ki: &monero::blockdata::transaction::KeyImage) -> bool) -> Result<bool,DB_FAILURES>;

        fn for_blocks_range(h1: &u64, h2: &u64, wat: fn(u: u64, h: &monero::Hash, blk: &monero::Block) -> bool) -> Result<bool,DB_FAILURES>; // u: u64 should be mut u: u64

        fn for_all_transactions(wat: fn(h: &monero::Hash, tx: &monero::Transaction) -> bool, pruned: bool) -> Result<bool,DB_FAILURES>;

        fn for_all_outputs();

        fn for_all_alt_blocks();

        fn set_hard_fork_version();

        fn get_hard_fork_version();

        fn check_hard_fork_info();

        fn drop_hard_fork_info();

        fn get_output_histogram();

        fn get_output_distribution();

        fn is_read_only();

        fn get_database_size();

        fn fixup();

        fn set_auto_remove_logs();

        // some note to help me

        // get_*_tx group : get_tx, get_pruned_tx. they are duplicate

        // Confirmed part that don't need to be redfined or smthg

        // -----------------------------------------| Transactions |----------------------------------------------------------



        /// `get_tx_count` fetches the total number of transactions stored in the database
        /// 
        /// Should return the count. In case of failure, a DB_FAILURES will be return.
        /// 
        /// No parameters is required.
        fn get_tx_count() -> Result<u64,DB_FAILURES>;

        /// `tx_exists` check if a transaction exist with the given hash.
        /// 
        /// Return `true` if the transaction exist, `false` otherwise. In case of failure, a DB_FAILURES will be return.
        /// 
        /// Parameters : 
        /// `h` is the given hash of transaction to check.
        ///  `tx_id` is an optional mutable reference to get the transaction id out of the found transaction.
        fn tx_exists(h: &Hash, tx_id: &mut Option<u64>) -> Result<bool, DB_FAILURES>;

        /// `get_tx_unlock_time` fetch a transaction's unlock time/height
        /// 
        /// Should return the unlock time/height in u64. In case of failure, a DB_FAILURES will be return.
        /// 
        /// Parameters:
        /// `h`: is the given hash of the transaction to check.
        fn get_tx_unlock_time(h: &Hash) -> Result<u64, DB_FAILURES>;

        /// `get_tx` fetches the transaction with the given hash.
        /// 
        /// Should return the transaction. In case of failure, a DB_FAILURES will be return.
        /// 
        /// Parameters:
        /// `h`: is the given hash of transaction to fetch.
        fn get_tx(h: &Hash) -> Result<Transaction,DB_FAILURES>;

        /// `get_pruned_tx` fetches the transaction base with the given hash.
        /// 
        /// Should return the transaction. In case of failure, a DB_FAILURES will be return.
        /// 
        /// Parameters:
        /// `h`: is the given hash of transaction to fetch.
        fn get_pruned_tx(h: &Hash) -> Result<Transaction,DB_FAILURES>;

        /// `get_tx_list` fetches the transactions with given hashes.
        /// 
        /// Should return a vector with the requested transactions. In case of failures, a DB_FAILURES will be return.
        /// Precisly, a HASH_DNE error will be returned with the correspondig hash of transaction that is not found in the DB.
        /// 
        /// `hlist`: is the given collection of hashes correspondig to the transactions to fetch.
        fn get_tx_list(hlist: &Vec<Hash>) -> Result<Vec<monero::Transaction>,DB_FAILURES>;

        /// `get_tx_blob` fetches the transaction blob with the given hash.
        /// 
        ///  Should return the transaction blob. In case of failure, a DB_FAILURES will be return.
        /// 
        /// Parameters:
        /// `h`: is the given hash of the transaction to fetch.
        fn get_tx_blob(h: &Hash) -> Result<Blobdata,DB_FAILURES>;

        /// `get_pruned_tx_blob` fetches the pruned transaction blob with the given hash.
        ///
        ///  Should return the transaction blob. In case of failure, a DB_FAILURES will be return.
        /// 
        /// Parameters:
        /// `h`: is the given hash of the transaction to fetch.
        fn get_pruned_tx_blob(h: &Hash) -> Result<Blobdata,DB_FAILURES>;

        /// `get_prunable_tx_blob` fetches the prunable transaction blob with the given hash.
        /// 
        /// Should return the transaction blob, In case of failure, a DB_FAILURES, will be return.
        /// 
        /// Parameters:
        /// `h`: is the given hash of the transaction to fetch.
        fn get_prunable_tx_blob(h: &Hash) -> Result<Blobdata,DB_FAILURES>;

        /// `get_prunable_tx_hash` fetches the prunable transaction hash
        /// 
        /// Should return the hash of the prunable transaction data. In case of failures, a DB_FAILURES, will be return.
        /// 
        /// Parameters:
        /// `tx_hash`: is the given hash of the transaction  to fetch.
        fn get_prunable_tx_hash(tx_hash: &Hash)  -> Result<Hash,DB_FAILURES>;

        /// `get_pruned_tx_blobs_from` fetches a number of pruned transaction blob from the given hash, in canonical blockchain order.
        /// 
        /// Should return the pruned transactions stored from the one with the given hash. In case of failure, a DB_FAILURES will be return.
        /// Precisly, an ARITHMETIC_COUNT error will be returned if the first transaction does not exist or their are fewer transactions than the count.
        /// 
        /// Parameters:
        /// `h`: is the given hash of the first transaction/
        /// `count`: is the number of transaction to fetch in canoncial blockchain order.
        fn get_pruned_tx_blobs_from(h: &Hash, count: usize) -> Result<Vec<Blobdata>,DB_FAILURES>;

        /// `get_tx_block_height` fetches the height of a transaction's block
        /// 
        /// Should return the height of the block containing the transaction with the given hash. In case
        /// of failures, a DB FAILURES will be return. Precisely, a TX_DNE error will be return if the transaction cannot be found.
        /// 
        /// Parameters:
        /// `h`: is the fiven hash of the first transaction
        fn get_tx_block_height(h: &Hash) -> Result<u64,DB_FAILURES>;



        // -----------------------------------------|  Blocks  |----------------------------------------------------------



        /// `blocks_exists` check if the given block exists
        /// 
        /// Return `true` if the block exist, `false` otherwise. In case of failures, a `DB_FAILURES` will be return.
        /// 
        /// Parameters:
        /// `h`: is the given hash of the requested block.
        fn block_exists(h: &Hash) -> Result<bool,DB_FAILURES>;

        /// `get_block` fetches the block with the given hash.
        /// 
        /// Return the requested block. In case of failures, a `DB_FAILURES` will be return. Precisely, a `BLOCK_DNE`
        /// error will be returned if the requested block can't be found.
        /// 
        /// Parameters:
        /// `h`: is the given hash of the requested block.
        fn get_block(h: &Hash) -> Result<monero::Block,DB_FAILURES>;

        /// `get_block_from_height` fetches the block located at the given height.
        /// 
        /// Return the requested block. In case of failures, a `DB_FAILURES` will be return. Precisely, a `BLOCK_DNE`
        /// error will be returned if the requested block can't be found.
        /// 
        /// Parameters:
        /// `height`: is the height where the requested block is located.
        fn get_block_from_height(height: u64) -> Result<monero::Block,DB_FAILURES>;

        /// `get_block_from_range` fetches the blocks located from and to the specified heights.
        /// 
        /// Return the requested blocks. In case of failures, a `DB_FAILURES` will be return. Precisely, a `BLOCK_DNE`
        /// error will be returned if at least requested block can't be found. If the range requested past the end of the blockchain,
        /// an `ARITHMETIC_COUNT` error will be return.
        /// 
        /// Parameters:
        /// `height_range`: is the range of height where the requested blocks are located.
        fn get_blocks_from_range(height_range: Range<u64>) -> Result<Vec<monero::Block>,DB_FAILURES>;

        /// `get_block_blob` fetches the block blob with the given hash.
        /// 
        /// Return the requested block blob. In case of failures, a `DB_FAILURES` will be return. Precisely, a `BLOCK_DNE`
        /// error will be returned if the requested block can't be found.
        /// 
        /// Parameters:
        /// `h`: is the given hash of the requested block.
        fn get_block_blob(h: monero::cryptonote::hash::Hash) -> Result<Blobdata,DB_FAILURES>;


        //a wat?
        fn get_blocks_from(
                start_height: u64, 
                min_block_count: usize, 
                max_block_count: usize, 
                max_tx_count: usize, 
                max_size: usize, 
                pruned: bool, 
                skip_coinbase: bool, 
                get_miner_tx_hash: bool) -> Result<Vec<((String, monero::cryptonote::hash::Hash), Vec<(monero::cryptonote::hash::Hash, String)>)>,DB_FAILURES>;
        

        

        // specific stats
       
        fn get_block_header(h: monero::cryptonote::hash::Hash) -> Result<monero::BlockHeader,DB_FAILURES>;

        fn get_block_blob_from_height(height: u64) -> Result<String,DB_FAILURES>;

        fn get_block_weight(weight: u64) -> Result<usize, DB_FAILURES>;

        fn get_block_weights(start_height: u64, count: usize) -> Result<Vec<u64>,DB_FAILURES>;

        fn get_block_already_generated_coins(height: u64) -> Result<u64,DB_FAILURES>;

        fn get_block_long_term_weight(height: u64) -> Result<u64,DB_FAILURES>;

        fn get_long_term_block_weights(height: u64, count: usize) -> Result<Vec<u64>,DB_FAILURES>; // Shouldn't have DB_FAILURES

        fn get_block_hash_from_height(height: u64) -> Result<monero::cryptonote::hash::Hash,DB_FAILURES>;

        // global stats

        fn get_block_timestamp(height: u64) -> Result<u64,DB_FAILURES>;
        fn get_block_height(h: monero::cryptonote::hash::Hash) -> Result<u64,DB_FAILURES>;

        // bool m_open;
        // mutable epee::critical_section m_synchronization_lock;  //!< A lock, currently for when BlockchainLMDB needs to resize the backing db file
}




