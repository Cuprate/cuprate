//! P2P
//!
//! Will handle initiating the P2P and contains a protocol request handler.

use std::{convert::From, str::FromStr};

use anyhow::anyhow;
use futures::{FutureExt, TryFutureExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{
    mpsc,
    oneshot::{self, Sender},
};
use tower::{Service, ServiceExt};

use cuprate_p2p::{config::TransportConfig, NetworkInterface, P2PConfig};
use cuprate_p2p_core::{
    client::{InternalPeerID, PeerSyncCallback},
    transports::Tcp,
    ClearNet, NetworkZone, Tor, Transport,
};
use cuprate_p2p_transport::{Daemon, Socks, SocksClientConfig};
use cuprate_txpool::service::{TxpoolReadHandle, TxpoolWriteHandle};
use cuprate_types::blockchain::BlockchainWriteRequest;

use crate::{
    blockchain::BlockchainInterface,
    constants::PANIC_CRITICAL_SERVICE_ERROR,
    tor::{transport_clearnet_daemon_config, transport_daemon_config, TorContext, TorMode},
    txpool::{self, IncomingTxHandler},
    LaunchContext,
};

#[cfg(feature = "arti")]
use {
    crate::tor::{transport_arti_config, transport_clearnet_arti_config},
    arti_client::TorClient,
    cuprate_p2p_transport::{Arti, ArtiClientConfig},
    tor_rtcompat::PreferredRuntime,
};

mod core_sync_service;
mod network_address;
pub mod request_handler;

pub use network_address::CrossNetworkInternalPeerId;

/// A simple parsing enum for the `p2p.clear_net.proxy` field
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
#[serde(try_from = "String", into = "String")]
pub enum ProxySettings {
    Disabled,
    Tor,
    Socks(SocksClientConfig),
}

impl TryFrom<String> for ProxySettings {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.as_str() {
            "" => Ok(Self::Disabled),
            s if s.eq_ignore_ascii_case("tor") => Ok(Self::Tor),
            url if url
                .get(..9)
                .is_some_and(|scheme| scheme.eq_ignore_ascii_case("socks5://")) =>
            {
                let url = url.get(9..).unwrap_or_default();

                let (authentication, addr) = match url.rsplit_once('@') {
                    Some((userpass, addr)) => {
                        let (user, pass) = userpass.split_once(':').ok_or_else(|| {
                            anyhow!("Invalid proxy authentication, expected user:pass format, got: {userpass}")
                        })?;

                        if user.is_empty() || pass.is_empty() {
                            return Err(anyhow!("Proxy username and password must not be empty."));
                        }

                        (Some((user.to_string(), pass.to_string())), addr)
                    }
                    None => (None, url),
                };

                Ok(Self::Socks(SocksClientConfig {
                    proxy: addr.parse()?,
                    authentication,
                }))
            }
            _ => Err(anyhow!("Unsupported proxy: '{s}'")),
        }
    }
}

impl From<ProxySettings> for String {
    fn from(settings: ProxySettings) -> Self {
        match settings {
            ProxySettings::Disabled => Self::new(),
            ProxySettings::Tor => "Tor".into(),
            ProxySettings::Socks(config) => match config.authentication {
                Some((u, p)) => format!("socks5://{u}:{p}@{}", config.proxy),
                None => format!("socks5://{}", config.proxy),
            },
        }
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

/// Initialize the clearnet P2P network zone. Returns [`NetworkInterface<ClearNet>`] and
/// [`Sender<IncomingTxHandler>`] for propagating the tx handler.
pub async fn initialize_clearnet_p2p(
    launch_ctx: &LaunchContext,
    tor_ctx: &TorContext,
) -> (NetworkInterface<ClearNet>, Sender<IncomingTxHandler>) {
    let config = launch_ctx.config.as_ref();
    let peer_sync_callback = launch_ctx.syncer.callback(&launch_ctx.blockchain);

    match &config.p2p.clear_net.proxy {
        ProxySettings::Tor => match tor_ctx.mode {
            #[cfg(feature = "arti")]
            TorMode::Arti => {
                tracing::info!("Anonymizing clearnet connections through Arti.");
                start_zone_p2p::<ClearNet, Arti>(
                    &launch_ctx.blockchain,
                    launch_ctx.txpool_read.clone(),
                    config.clearnet_p2p_config(),
                    transport_clearnet_arti_config(tor_ctx),
                    Some(peer_sync_callback),
                )
                .await
                .unwrap()
            }
            TorMode::Daemon => start_zone_p2p::<ClearNet, Socks>(
                &launch_ctx.blockchain,
                launch_ctx.txpool_read.clone(),
                config.clearnet_p2p_config(),
                transport_clearnet_daemon_config(config),
                Some(peer_sync_callback),
            )
            .await
            .unwrap(),
            TorMode::Auto => unreachable!("Auto mode should be resolved before this point"),
        },
        ProxySettings::Disabled => start_zone_p2p::<ClearNet, Tcp>(
            &launch_ctx.blockchain,
            launch_ctx.txpool_read.clone(),
            config.clearnet_p2p_config(),
            config.p2p.clear_net.tcp_transport_config(config.network),
            Some(peer_sync_callback),
        )
        .await
        .unwrap(),
        ProxySettings::Socks(socks_config) => start_zone_p2p::<ClearNet, Socks>(
            &launch_ctx.blockchain,
            launch_ctx.txpool_read.clone(),
            config.clearnet_p2p_config(),
            TransportConfig {
                client_config: socks_config.clone(),
                server_config: None,
            },
            Some(peer_sync_callback),
        )
        .await
        .unwrap(),
    }
}

/// Initialize the Tor P2P network zone after the node has synced with the network.
/// Publishes [`NetworkInterface<Tor>`] and forwards the [`IncomingTxHandler`] to the Tor zone.
pub fn initialize_tor_p2p(
    launch_ctx: LaunchContext,
    tor_context: TorContext,
    tx_handler: IncomingTxHandler,
    interface_publisher: Sender<NetworkInterface<Tor>>,
    dandelion_router: Option<Sender<NetworkInterface<Tor>>>,
) {
    tracing::info!("Tor P2P zone will start after sync.");

    let task_executor = launch_ctx.task_executor.clone();
    let shutdown_token = task_executor.cancellation_token();

    task_executor.spawn(async move {
        // Wait for the node to synchronize with the network, or shutdown.
        tokio::select! {
            result = launch_ctx.syncer.wait_for_synced() => {
                if result.is_err() {
                    tracing::info!("Not starting Tor P2P zone, syncer stopped");
                    return;
                }
            }
            () = shutdown_token.cancelled() => {
                return;
            }
        }
        tracing::info!("Starting Tor P2P zone.");

        let config = launch_ctx.config.as_ref();
        let (tor_interface, tor_tx_handler_tx) = match tor_context.mode {
            TorMode::Daemon => start_zone_p2p::<Tor, Daemon>(
                &launch_ctx.blockchain,
                launch_ctx.txpool_read.clone(),
                config.tor_p2p_config(&tor_context),
                transport_daemon_config(config),
                None,
            )
            .await
            .unwrap(),
            #[cfg(feature = "arti")]
            TorMode::Arti => start_zone_p2p::<Tor, Arti>(
                &launch_ctx.blockchain,
                launch_ctx.txpool_read.clone(),
                config.tor_p2p_config(&tor_context),
                transport_arti_config(config, tor_context),
                None,
            )
            .await
            .unwrap(),
            TorMode::Auto => unreachable!("Auto mode should be resolved before this point"),
        };

        // Publish the Tor interface for consumers
        drop(interface_publisher.send(tor_interface.clone()));

        // Send the tx handler to the Tor zone
        if tor_tx_handler_tx.send(tx_handler).is_err() {
            tracing::warn!("Failed to send tx handler to Tor zone.");
            return;
        }

        // Deliver the Tor network interface to the dandelion router.
        if let Some(tx) = dandelion_router {
            if tx.send(tor_interface).is_err() {
                tracing::warn!("Failed to deliver Tor router to dandelion pool.");
            }
        }
    });
}

/// Starts the P2P network zone, returning a [`NetworkInterface`] to interact with it.
///
/// A [`oneshot::Sender`] is also returned to provide the [`IncomingTxHandler`], until this is provided network
/// handshakes can not be completed.
pub async fn start_zone_p2p<N, T>(
    blockchain: &BlockchainInterface,
    txpool_read_handle: TxpoolReadHandle,
    config: P2PConfig<N>,
    transport_config: TransportConfig<N, T>,
    peer_sync_callback: Option<PeerSyncCallback>,
) -> Result<(NetworkInterface<N>, Sender<IncomingTxHandler>), tower::BoxError>
where
    N: NetworkZone,
    T: Transport<N>,
    N::Addr: borsh::BorshDeserialize + borsh::BorshSerialize,
    CrossNetworkInternalPeerId: From<InternalPeerID<<N as NetworkZone>::Addr>>,
{
    let context_svc = blockchain.context_svc();
    let (incoming_tx_handler_tx, incoming_tx_handler_rx) = oneshot::channel();

    let request_handler_maker = request_handler::P2pProtocolRequestHandlerMaker {
        blockchain_read_handle: blockchain.read(),
        blockchain_context_service: context_svc.clone(),
        txpool_read_handle,
        incoming_tx_handler: None,
        incoming_tx_handler_fut: incoming_tx_handler_rx.shared(),
        blockchain_manager: blockchain.manager(),
    };

    Ok((
        cuprate_p2p::initialize_network::<N, T, _, _>(
            request_handler_maker.map_response(|s| s.map_err(Into::into)),
            core_sync_service::CoreSyncService(context_svc),
            config,
            transport_config,
            peer_sync_callback,
        )
        .await?,
        incoming_tx_handler_tx,
    ))
}
