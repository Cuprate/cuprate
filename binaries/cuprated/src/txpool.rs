//! Transaction Pool
//!
//! Handles initiating the tx-pool, providing the preprocessor required for the dandelion pool.
use cuprate_consensus::BlockChainContextService;
use cuprate_p2p::NetworkInterface;
use cuprate_p2p_core::ClearNet;
use cuprate_txpool::service::{TxpoolReadHandle, TxpoolWriteHandle};

use crate::blockchain::ConcreteTxVerifierService;

mod dandelion;
mod incoming_tx;
mod txs_being_handled;

pub use incoming_tx::IncomingTxHandler;

/// Initialize the [`IncomingTxHandler`].
#[expect(clippy::significant_drop_tightening)]
pub fn incoming_tx_handler(
    clear_net: NetworkInterface<ClearNet>,
    txpool_write_handle: TxpoolWriteHandle,
    txpool_read_handle: TxpoolReadHandle,
    blockchain_context_cache: BlockChainContextService,
    tx_verifier_service: ConcreteTxVerifierService,
) -> IncomingTxHandler {
    let dandelion_router = dandelion::dandelion_router(clear_net);

    let dandelion_pool_manager = dandelion::start_dandelion_pool_manager(
        dandelion_router,
        txpool_read_handle.clone(),
        txpool_write_handle.clone(),
    );

    IncomingTxHandler {
        txs_being_handled: txs_being_handled::TxsBeingHandled::new(),
        blockchain_context_cache,
        dandelion_pool_manager,
        tx_verifier_service,
        txpool_write_handle,
        txpool_read_handle,
    }
}
