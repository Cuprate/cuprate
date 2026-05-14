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
    config::Config,
    constants::CRITICAL_SERVICE_ERROR,
    tor::{transport_clearnet_daemon_config, transport_daemon_config, TorContext, TorMode},
    txpool::{self, IncomingTxHandler},
    NodeContext,
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
            "Tor" => Ok(Self::Tor),
            // TODO: use `if let` guard when on >=1.95
            url if url.starts_with("socks5://") => {
                let url = url.strip_prefix("socks5://").unwrap();

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
    config: &Config,
    node_ctx: &NodeContext,
    tor_ctx: &TorContext,
    peer_sync_callback: PeerSyncCallback,
) -> Result<(NetworkInterface<ClearNet>, Sender<IncomingTxHandler>), anyhow::Error> {
    let result = match &config.p2p.clear_net.proxy {
        ProxySettings::Tor => match tor_ctx.mode {
            #[cfg(feature = "arti")]
            TorMode::Arti => {
                tracing::info!("Anonymizing clearnet connections through Arti.");
                start_zone_p2p::<ClearNet, Arti>(
                    &node_ctx.blockchain,
                    node_ctx.txpool_read.clone(),
                    config.clearnet_p2p_config(),
                    transport_clearnet_arti_config(tor_ctx)?,
                    Some(peer_sync_callback.clone()),
                )
                .await
            }
            TorMode::Daemon => {
                start_zone_p2p::<ClearNet, Socks>(
                    &node_ctx.blockchain,
                    node_ctx.txpool_read.clone(),
                    config.clearnet_p2p_config(),
                    transport_clearnet_daemon_config(config),
                    Some(peer_sync_callback.clone()),
                )
                .await
            }
            TorMode::Auto => unreachable!("Auto mode should be resolved before this point"),
        },
        ProxySettings::Disabled => {
            start_zone_p2p::<ClearNet, Tcp>(
                &node_ctx.blockchain,
                node_ctx.txpool_read.clone(),
                config.clearnet_p2p_config(),
                config.p2p.clear_net.tcp_transport_config(config.network),
                Some(peer_sync_callback.clone()),
            )
            .await
        }
        ProxySettings::Socks(socks_config) => {
            start_zone_p2p::<ClearNet, Socks>(
                &node_ctx.blockchain,
                node_ctx.txpool_read.clone(),
                config.clearnet_p2p_config(),
                TransportConfig {
                    client_config: socks_config.clone(),
                    server_config: None,
                },
                Some(peer_sync_callback.clone()),
            )
            .await
        }
    };

    result.map_err(anyhow::Error::from_boxed)
}

/// Start the Tor P2P network zone. Returns [`NetworkInterface<Tor>`] and
/// a [`Sender<IncomingTxHandler>`] for propagating the tx handler.
pub async fn start_tor_p2p(
    config: &Config,
    tor_ctx: TorContext,
    node_ctx: &NodeContext,
) -> Result<(NetworkInterface<Tor>, Sender<IncomingTxHandler>), anyhow::Error> {
    let result = match tor_ctx.mode {
        TorMode::Daemon => {
            start_zone_p2p::<Tor, Daemon>(
                &node_ctx.blockchain,
                node_ctx.txpool_read.clone(),
                config.tor_p2p_config(&tor_ctx),
                transport_daemon_config(config),
                None,
            )
            .await
        }
        #[cfg(feature = "arti")]
        TorMode::Arti => {
            let p2p_config = config.tor_p2p_config(&tor_ctx);
            let transport = transport_arti_config(config, tor_ctx)?;
            start_zone_p2p::<Tor, Arti>(
                &node_ctx.blockchain,
                node_ctx.txpool_read.clone(),
                p2p_config,
                transport,
                None,
            )
            .await
        }
        TorMode::Auto => unreachable!("Auto mode should be resolved before this point"),
    };

    result.map_err(anyhow::Error::from_boxed)
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
