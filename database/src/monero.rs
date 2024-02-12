//! Abstracted Monero database operations; `trait Monero`.

//---------------------------------------------------------------------------------------------------- Import
#[allow(unused_imports)] // FIXME: these traits will be eventually in the function impls.
use crate::{
    database::Database,
    env::Env,
    table::Table,
    transaction::{RoTx, RwTx},
    ConcreteEnv,
};

//---------------------------------------------------------------------------------------------------- Monero
/// Monero database operations.
///
/// This trait builds on top of:
/// - [`Env`]
/// - [`Table`]
/// - [`Database`]
/// - [`RoTx`], [`RwTx`]
///
/// to provide higher-level `Monero`-specific database operations.
///
/// # Notes
/// Things to note about this trait:
/// 1. All methods are already implemented
/// 2. [`ConcreteEnv`] automatically implements [`Monero`]
///
/// This means that these methods can be
/// called directly on `ConcreteEnv`.
///
/// Normal `tower::Service` readers/writers will still go
/// through messages but the underlying database thread will
/// use these functions.
///
/// Underlying database backends can re-implement any of
/// these functions if the generic version is not good enough.
///
/// # TODO
/// TODO: What is a good name for this trait?
///
/// TODO: These functions should pretty much map 1-1 to the `Request` enum.
///
/// TODO: These are function names from `old_database/` for now.
/// The actual underlying functions (e.g `get()`) aren't implemented.
#[allow(missing_docs)]
pub trait Monero: Env {
    //-------------------------------------------------------- Blockchain
    fn height() {
        todo!()
    }

    //-------------------------------------------------------- Blocks
    fn add_block() {
        todo!()
    }
    fn add_block_data() {
        todo!()
    }
    fn pop_block() {
        todo!()
    }
    fn block_exists() {
        todo!()
    }
    fn get_block_hash() {
        todo!()
    }
    fn get_block_height() {
        todo!()
    }
    fn get_block_weight() {
        todo!()
    }
    fn get_block_already_generated_coins() {
        todo!()
    }
    fn get_block_long_term_weight() {
        todo!()
    }
    fn get_block_timestamp() {
        todo!()
    }
    fn get_block_cumulative_rct_outputs() {
        todo!()
    }
    fn get_block() {
        todo!()
    }
    fn get_block_from_height() {
        todo!()
    }
    fn get_block_header() {
        todo!()
    }
    fn get_block_header_from_height() {
        todo!()
    }
    fn get_top_block() {
        todo!()
    }
    fn get_top_block_hash() {
        todo!()
    }

    //-------------------------------------------------------- Transactions
    fn add_transaction() {
        todo!()
    }
    fn add_transaction_data() {
        todo!()
    }
    fn remove_transaction() {
        todo!()
    }
    fn remove_transaction_data() {
        todo!()
    }
    fn remove_tx_outputs() {
        todo!()
    }
    fn get_num_tx() {
        todo!()
    }
    fn tx_exists() {
        todo!()
    }
    fn get_tx_unlock_time() {
        todo!()
    }
    fn get_tx() {
        todo!()
    }
    fn get_tx_list() {
        todo!()
    }
    fn get_pruned_tx() {
        todo!()
    }
    fn get_tx_block_height() {
        todo!()
    }

    //-------------------------------------------------------- Outputs
    fn add_output() {
        todo!()
    }
    fn remove_output() {
        todo!()
    }
    fn get_output() {
        todo!()
    }
    fn get_output_list() {
        todo!()
    }
    fn get_rct_num_outputs() {
        todo!()
    }
    fn get_pre_rct_num_outputs() {
        todo!()
    }

    //-------------------------------------------------------- Spent Keys
    fn add_spent_key() {
        todo!()
    }
    fn remove_spent_key() {
        todo!()
    }
    fn is_spent_key_recorded() {
        todo!()
    }

    //-------------------------------------------------------- Alt Blocks
    fn add_alt_block() {
        todo!()
    }
    fn get_alt_block() {
        todo!()
    }
    fn remove_alt_block() {
        todo!()
    }
    fn get_alt_block_count() {
        todo!()
    }
    fn drop_alt_blocks() {
        todo!()
    }

    //-------------------------------------------------------- Properties
    fn get_blockchain_pruning_seed() {
        todo!()
    }
}

impl Monero for ConcreteEnv {}
