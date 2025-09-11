//! P2P
//!
//! Will handle initiating the P2P and contains a protocol request handler.

use std::{convert::From, str::FromStr};

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
    client::InternalPeerID, transports::Tcp, ClearNet, NetworkZone, Tor, Transport,
};
use cuprate_p2p_transport::{Arti, ArtiClientConfig, Daemon, Socks, SocksClientConfig};
use cuprate_txpool::service::{TxpoolReadHandle, TxpoolWriteHandle};
use cuprate_types::blockchain::BlockchainWriteRequest;

use crate::{
    blockchain,
    config::Config,
    constants::PANIC_CRITICAL_SERVICE_ERROR,
    tor::{
        transport_arti_config, transport_clearnet_arti_config, transport_clearnet_daemon_config, transport_daemon_config, TorContext, TorMode
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

impl TryFrom<ProxySettings> for SocksClientConfig {
    type Error = anyhow::Error;

    fn try_from(value: ProxySettings) -> Result<Self, Self::Error> {
        let ProxySettings::Socks(url) = value else {
            panic!("Tor proxy setting should not be parsed!")
        };

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

        Ok(Self {
            proxy: addr.parse()?,
            authentication,
        })
    }
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

/// Initialize all P2P network zones. Returning a [`NetworkInterfaces`] collection and
/// a [`Vec<Sender<IncomingTxHandler>>`] for propagating the tx handler.
pub async fn initialize_zones_p2p(
    config: &Config,
    context_svc: BlockchainContextService,
    mut blockchain_read_handle: BlockchainReadHandle,
    txpool_read_handle: TxpoolReadHandle,
    tor_ctx: TorContext,
) -> (NetworkInterfaces, Vec<Sender<IncomingTxHandler>>) {
    // Start clearnet P2P.
    let (clearnet, incoming_tx_handler_tx) = {
        // If proxy is set
        match config.p2p.clear_net.proxy {
            ProxySettings::Tor => match tor_ctx.mode {
                TorMode::Arti => {
                    tracing::info!("Anonymizing clearnet connections through Arti.");
                    start_zone_p2p::<ClearNet, Arti>(
                        blockchain_read_handle.clone(),
                        context_svc.clone(),
                        txpool_read_handle.clone(),
                        config.clearnet_p2p_config(),
                        transport_clearnet_arti_config(&tor_ctx),
                    )
                    .await
                    .unwrap()
                }
                TorMode::Daemon => {
                    start_zone_p2p::<ClearNet, Socks>(
                        blockchain_read_handle.clone(),
                        context_svc.clone(),
                        txpool_read_handle.clone(),
                        config.clearnet_p2p_config(),
                        transport_clearnet_daemon_config(config),
                    )
                    .await
                    .unwrap()
                }
                TorMode::Off => {
                    tracing::error!("Clearnet proxy set to \"tor\" but Tor is actually off. Please be sure to set a mode in the configuration or command line");
                    std::process::exit(0);
                }
            },
            ProxySettings::Socks(ref s) => {
                if s.is_empty() {
                    start_zone_p2p::<ClearNet, Tcp>(
                        blockchain_read_handle.clone(),
                        context_svc.clone(),
                        txpool_read_handle.clone(),
                        config.clearnet_p2p_config(),
                        config.p2p.clear_net.tcp_transport_config(config.network),
                    )
                    .await
                    .unwrap()
                } else {
                    start_zone_p2p::<ClearNet, Socks>(
                        blockchain_read_handle.clone(),
                        context_svc.clone(),
                        txpool_read_handle.clone(),
                        config.clearnet_p2p_config(),
                        config.p2p.clear_net.socks_transport_config(),
                    )
                    .await
                    .unwrap()
                }
            }
        }
    };

    // Create network interface collection
    let mut network_interfaces = NetworkInterfaces::new(clearnet);
    let mut tx_handler_subscribers = vec![incoming_tx_handler_tx];

    // Start Tor P2P (if enabled)
    let tor = if config.p2p.tor_net.enabled {
        match tor_ctx.mode {
            TorMode::Off => None,
            TorMode::Daemon => Some(
                start_zone_p2p::<Tor, Daemon>(
                    blockchain_read_handle.clone(),
                    context_svc.clone(),
                    txpool_read_handle.clone(),
                    config.tor_p2p_config(&tor_ctx),
                    transport_daemon_config(config),
                )
                .await
                .unwrap(),
            ),
            TorMode::Arti => Some(
                start_zone_p2p::<Tor, Arti>(
                    blockchain_read_handle.clone(),
                    context_svc.clone(),
                    txpool_read_handle.clone(),
                    config.tor_p2p_config(&tor_ctx),
                    transport_arti_config(config, tor_ctx),
                )
                .await
                .unwrap(),
            ),
        }
    } else {
        None
    };
    if let Some((tor, incoming_tx_handler_tx)) = tor {
        network_interfaces.tor_network_interface = Some(tor);
        tx_handler_subscribers.push(incoming_tx_handler_tx);
    }

    (network_interfaces, tx_handler_subscribers)
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
        incoming_tx_handler: None,
        incoming_tx_handler_fut: incoming_tx_handler_rx.shared(),
    };

    Ok((
        cuprate_p2p::initialize_network::<N, T, _, _>(
            request_handler_maker.map_response(|s| s.map_err(Into::into)),
            core_sync_service::CoreSyncService(blockchain_context_service),
            config,
            transport_config,
        )
        .await?,
        incoming_tx_handler_tx,
    ))
}
