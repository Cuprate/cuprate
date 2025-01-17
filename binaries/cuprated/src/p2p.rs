//! P2P
//!
//! Will handle initiating the P2P and contains a protocol request handler.
use futures::{FutureExt, TryFutureExt};
use tokio::sync::oneshot;
use tower::ServiceExt;

use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_consensus::BlockchainContextService;
use cuprate_p2p::{NetworkInterface, P2PConfig};
use cuprate_p2p_core::ClearNet;
use cuprate_txpool::service::TxpoolReadHandle;

use crate::txpool::IncomingTxHandler;

mod core_sync_service;
mod network_address;
pub mod request_handler;

pub use network_address::CrossNetworkInternalPeerId;

/// Starts the P2P clearnet network, returning a [`NetworkInterface`] to interact with it.
///
/// A [`oneshot::Sender`] is also returned to provide the [`IncomingTxHandler`], until this is provided network
/// handshakes can not be completed.
pub async fn start_clearnet_p2p(
    blockchain_read_handle: BlockchainReadHandle,
    blockchain_context_service: BlockchainContextService,
    txpool_read_handle: TxpoolReadHandle,
    config: P2PConfig<ClearNet>,
) -> Result<
    (
        NetworkInterface<ClearNet>,
        oneshot::Sender<IncomingTxHandler>,
    ),
    tower::BoxError,
> {
    let (incoming_tx_handler_tx, incoming_tx_handler_rx) = oneshot::channel();

    let request_handler_maker = request_handler::P2pProtocolRequestHandlerMaker {
        blockchain_read_handle,
        blockchain_context_service: blockchain_context_service.clone(),
        txpool_read_handle,
        incoming_tx_handler: None,
        incoming_tx_handler_fut: incoming_tx_handler_rx.shared(),
    };

    Ok((
        cuprate_p2p::initialize_network(
            request_handler_maker.map_response(|s| s.map_err(Into::into)),
            core_sync_service::CoreSyncService(blockchain_context_service),
            config,
        )
        .await?,
        incoming_tx_handler_tx,
    ))
}
