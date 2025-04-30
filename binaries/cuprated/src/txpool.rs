//! Transaction Pool
//!
//! Handles initiating the tx-pool, providing the preprocessor required for the dandelion pool.
use cuprate_consensus::BlockchainContextService;
use cuprate_p2p::NetworkInterface;
use cuprate_p2p_core::ClearNet;
use cuprate_txpool::service::{TxpoolReadHandle, TxpoolWriteHandle};

mod dandelion;
mod incoming_tx;
mod relay_rules;
mod txs_being_handled;

pub use incoming_tx::{IncomingTxError, IncomingTxHandler, IncomingTxs};
