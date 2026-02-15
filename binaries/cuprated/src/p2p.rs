//! P2P
//!
//! Will handle initiating the P2P and contains a protocol request handler.

use std::{convert::From, str::FromStr, sync::Arc};

use anyhow::anyhow;
use arti_client::TorClient;
use futures::{FutureExt, TryFutureExt};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot::{self, Sender};
use tor_rtcompat::PreferredRuntime;
use tower::{Service, ServiceExt};

use cuprate_blockchain::service::{BlockchainReadHandle, BlockchainWriteHandle};
use cuprate_consensus::BlockchainContextService;
use cuprate_p2p::{config::TransportConfig, NetworkInterface, P2PConfig};
use cuprate_p2p_core::{
    client::InternalPeerID, transports::Tcp, ClearNet, NetworkZone, SyncerWake, Tor, Transport,
};
use cuprate_p2p_transport::{Arti, ArtiClientConfig, Daemon, Socks, SocksClientConfig};
use cuprate_txpool::service::{TxpoolReadHandle, TxpoolWriteHandle};
use cuprate_types::blockchain::BlockchainWriteRequest;

use crate::{
    blockchain,
    config::Config,
    constants::PANIC_CRITICAL_SERVICE_ERROR,
    tor::{
        transport_arti_config, transport_clearnet_arti_config, transport_clearnet_daemon_config,
        transport_daemon_config, TorContext, TorMode,
    },
    txpool::{self, IncomingTxHandler},
};

mod core_sync_service;
mod network_address;
pub mod request_handler;

pub use network_address::CrossNetworkInternalPeerId;

/// A simple parsing enum for the `p2p.clear_net.proxy` field
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum ProxySettings {
    Tor,
    #[serde(untagged)]
    Socks(String),
}

/// Converts a `str` to a [`SocksClientConfig`].
fn socks_proxy_str_to_config(url: &str) -> Result<SocksClientConfig, anyhow::Error> {
    let Some((_, url)) = url.split_once("socks5://") else {
        return Err(anyhow!("Invalid proxy url header."));
    };

    let (authentication, addr) = url
        .split_once('@')
        .map(|(up, ad)| {
            (
                up.split_once(':')
                    .map(|(a, b)| (a.to_string(), b.to_string())),
                ad,
            )
        })
        .unwrap_or((None, url));

    Ok(SocksClientConfig {
        proxy: addr.parse()?,
        authentication,
    })
}

/// This struct collect all supported and optional network zone interfaces.
pub struct NetworkInterfaces {
    /// Mandatory clearnet network interface
    pub clearnet_network_interface: NetworkInterface<ClearNet>,
    /// Optional tor network interface
    pub tor_network_interface: Option<NetworkInterface<Tor>>,
    // ...one can dream for more!
}

impl NetworkInterfaces {
    pub const fn new(clearnet_network_interface: NetworkInterface<ClearNet>) -> Self {
        Self {
            clearnet_network_interface,
            tor_network_interface: None,
        }
    }
}

/// Initialize the clearnet P2P network zone. Returns [`NetworkInterface<ClearNet>`] and
/// [`Sender<IncomingTxHandler>`] for propagating the tx handler.
pub async fn initialize_clearnet_p2p(
    config: &Config,
    context_svc: BlockchainContextService,
    blockchain_read_handle: BlockchainReadHandle,
    txpool_read_handle: TxpoolReadHandle,
    tor_ctx: &TorContext,
    syncer_wake: Arc<SyncerWake>,
) -> (NetworkInterface<ClearNet>, Sender<IncomingTxHandler>) {
    match config.p2p.clear_net.proxy {
        ProxySettings::Tor => match tor_ctx.mode {
            TorMode::Arti => {
                tracing::info!("Anonymizing clearnet connections through Arti.");
                start_zone_p2p::<ClearNet, Arti>(
                    blockchain_read_handle,
                    context_svc,
                    txpool_read_handle,
                    config.clearnet_p2p_config(),
                    transport_clearnet_arti_config(tor_ctx),
                    Some(syncer_wake),
                )
                .await
                .unwrap()
            }
            TorMode::Daemon => start_zone_p2p::<ClearNet, Socks>(
                blockchain_read_handle,
                context_svc,
                txpool_read_handle,
                config.clearnet_p2p_config(),
                transport_clearnet_daemon_config(config),
                Some(syncer_wake),
            )
            .await
            .unwrap(),
            TorMode::Auto => unreachable!("Auto mode should be resolved before this point"),
        },
        ProxySettings::Socks(ref s) => {
            if s.is_empty() {
                start_zone_p2p::<ClearNet, Tcp>(
                    blockchain_read_handle,
                    context_svc,
                    txpool_read_handle,
                    config.clearnet_p2p_config(),
                    config.p2p.clear_net.tcp_transport_config(config.network),
                    Some(syncer_wake),
                )
                .await
                .unwrap()
            } else {
                start_zone_p2p::<ClearNet, Socks>(
                    blockchain_read_handle,
                    context_svc,
                    txpool_read_handle,
                    config.clearnet_p2p_config(),
                    TransportConfig {
                        client_config: socks_proxy_str_to_config(s).unwrap(),
                        server_config: None,
                    },
                    Some(syncer_wake),
                )
                .await
                .unwrap()
            }
        }
    }
}

/// Start the Tor P2P network zone. Returns [`NetworkInterface<Tor>`] and
/// a [`Sender<IncomingTxHandler>`] for propagating the tx handler.
pub async fn start_tor_p2p(
    config: &Config,
    context_svc: BlockchainContextService,
    blockchain_read_handle: BlockchainReadHandle,
    txpool_read_handle: TxpoolReadHandle,
    tor_ctx: TorContext,
) -> (NetworkInterface<Tor>, Sender<IncomingTxHandler>) {
    match tor_ctx.mode {
        TorMode::Daemon => start_zone_p2p::<Tor, Daemon>(
            blockchain_read_handle,
            context_svc,
            txpool_read_handle,
            config.tor_p2p_config(&tor_ctx),
            transport_daemon_config(config),
            None,
        )
        .await
        .unwrap(),
        TorMode::Arti => start_zone_p2p::<Tor, Arti>(
            blockchain_read_handle,
            context_svc,
            txpool_read_handle,
            config.tor_p2p_config(&tor_ctx),
            transport_arti_config(config, tor_ctx),
            None,
        )
        .await
        .unwrap(),
        TorMode::Auto => unreachable!("Auto mode should be resolved before this point"),
    }
}

/// Starts the P2P network zone, returning a [`NetworkInterface`] to interact with it.
///
/// A [`oneshot::Sender`] is also returned to provide the [`IncomingTxHandler`], until this is provided network
/// handshakes can not be completed.
pub async fn start_zone_p2p<N, T>(
    blockchain_read_handle: BlockchainReadHandle,
    blockchain_context_service: BlockchainContextService,
    txpool_read_handle: TxpoolReadHandle,
    config: P2PConfig<N>,
    transport_config: TransportConfig<N, T>,
    syncer_wake: Option<Arc<SyncerWake>>,
) -> Result<(NetworkInterface<N>, Sender<IncomingTxHandler>), tower::BoxError>
where
    N: NetworkZone,
    T: Transport<N>,
    N::Addr: borsh::BorshDeserialize + borsh::BorshSerialize,
    CrossNetworkInternalPeerId: From<InternalPeerID<<N as NetworkZone>::Addr>>,
{
    let (incoming_tx_handler_tx, incoming_tx_handler_rx) = oneshot::channel();

    let request_handler_maker = request_handler::P2pProtocolRequestHandlerMaker {
        blockchain_read_handle,
        blockchain_context_service: blockchain_context_service.clone(),
        txpool_read_handle,
        syncer_wake: syncer_wake.clone(),
        incoming_tx_handler: None,
        incoming_tx_handler_fut: incoming_tx_handler_rx.shared(),
    };

    Ok((
        cuprate_p2p::initialize_network::<N, T, _, _>(
            request_handler_maker.map_response(|s| s.map_err(Into::into)),
            core_sync_service::CoreSyncService(blockchain_context_service),
            config,
            transport_config,
            syncer_wake,
        )
        .await?,
        incoming_tx_handler_tx,
    ))
}
