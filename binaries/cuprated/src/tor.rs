//! Tor initialization
//!
//! Extract configuration and initialize Arti.

//---------------------------------------------------------------------------------------------------- Imports

use std::{default, sync::Arc};

use arti_client::{
    config::{onion_service::OnionServiceConfigBuilder, CfgPath, TorClientConfigBuilder},
    KeystoreSelector, StreamPrefs, TorClient, TorClientBuilder, TorClientConfig,
};
use futures::Stream;
use serde::{Deserialize, Serialize};
use tor_hsservice::{OnionService, RendRequest, RunningOnionService};
use tor_persist::hsnickname::HsNickname;
use tor_rtcompat::PreferredRuntime;
use tracing::info;

use cuprate_helper::fs::CUPRATE_DATA_DIR;
use cuprate_p2p::TransportConfig;
use cuprate_p2p_core::{ClearNet, Tor};
use cuprate_p2p_transport::{
    Arti, ArtiClientConfig, ArtiServerConfig, Daemon, DaemonClientConfig, DaemonServerConfig,
    Socks, SocksClientConfig,
};
use cuprate_wire::OnionAddr;

use crate::{
    config::{p2p_port, Config},
    p2p::ProxySettings,
};
//---------------------------------------------------------------------------------------------------- Initialization

#[derive(Clone, Default, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Describe if Tor is enabled and how
pub enum TorMode {
    /// Use of the [`arti_client`] library.
    Arti,
    /// Use of external tor daemon
    Daemon,

    #[default]
    /// Tor is disabled
    Off,
}

/// Contains the necessary Tor configuration or structures
/// for initializing P2P.
pub struct TorContext {
    /// Which mode are we using.
    pub mode: TorMode,

    // -------- Only in Arti mode
    /// Arti bootstrapped [`TorClient`].
    pub bootstrapped_client: Option<TorClient<PreferredRuntime>>,
    /// Arti bootstrapped client config
    pub arti_client_config: Option<TorClientConfig>,
    /// Arti onion service address.
    pub arti_onion_service: Option<OnionService>,
}

/// Initialize the Tor network if enabled in configuration
///
/// This function will bootstrap Arti if needed by Tor network zone or
/// clearnet as a proxy.
pub async fn initialize_tor_if_enabled(config: &Config) -> TorContext {
    let mode = config.tor.mode;
    let anonymize_clearnet = matches!(config.p2p.clear_net.proxy, ProxySettings::Tor);

    // Start Arti client
    let (bootstrapped_client, arti_client_config) =
        if mode == TorMode::Arti && (config.p2p.tor_net.enabled || anonymize_clearnet) {
            Some(initialize_arti_client(config).await)
        } else {
            None
        }
        .unzip();

    // Start Arti onion service
    let arti_onion_service = arti_client_config
        .as_ref()
        .map(|client_config| initialize_arti_onion_service(client_config, config));

    TorContext {
        mode,
        bootstrapped_client,
        arti_client_config,
        arti_onion_service,
    }
}

/// Initialize Arti Tor client.
async fn initialize_arti_client(config: &Config) -> (TorClient<PreferredRuntime>, TorClientConfig) {
    // Configuration
    let mut tor_config = TorClientConfig::builder();

    // Storage
    tor_config
        .storage()
        .state_dir(CfgPath::new_literal(config.tor.arti.directory_path.clone()));

    let tor_config = tor_config
        .build()
        .expect("Failed to build Tor client configuration.");

    // Bootstrapping
    info!("Bootstrapping Arti's TorClient...");
    let mut tor_client = TorClient::builder()
        .config(tor_config.clone())
        .create_bootstrapped()
        .await
        .inspect_err(|err| tracing::error!("Unable to bootstrap arti: {err}"))
        .unwrap();

    // Isolation
    if config.tor.arti.isolated_circuit {
        let mut stream_prefs = StreamPrefs::new();
        stream_prefs.isolate_every_stream();
        tor_client.set_stream_prefs(stream_prefs);
    }

    (tor_client, tor_config)
}

fn initialize_arti_onion_service(client_config: &TorClientConfig, config: &Config) -> OnionService {
    let onion_svc_config = OnionServiceConfigBuilder::default()
        .enable_pow(config.tor.arti.onion_service_pow)
        .nickname(HsNickname::new("cuprate".into()).unwrap())
        .build()
        .unwrap();

    TorClient::<PreferredRuntime>::create_onion_service(client_config, onion_svc_config)
        .expect("Unable to start Arti onion service.")
}

//---------------------------------------------------------------------------------------------------- Transport configuration

pub fn transport_arti_config(config: &Config, ctx: TorContext) -> TransportConfig<Tor, Arti> {
    // Extracting
    let (Some(bootstrapped_client), Some(client_config)) =
        (ctx.bootstrapped_client, ctx.arti_client_config)
    else {
        panic!("Arti client should be initialized");
    };

    let server_config = config.p2p.tor_net.inbound_onion.then(|| {
        let Some(onion_svc) = ctx.arti_onion_service else {
            panic!("inbound onion enabled, but no onion service initialized!");
        };

        ArtiServerConfig::new(
            onion_svc,
            p2p_port(config.p2p.tor_net.p2p_port, config.network),
            &bootstrapped_client,
            &client_config,
        )
    });

    TransportConfig::<Tor, Arti> {
        client_config: ArtiClientConfig {
            client: bootstrapped_client,
        },
        server_config,
    }
}

pub fn transport_clearnet_arti_config(ctx: &TorContext) -> TransportConfig<ClearNet, Arti> {
    let Some(bootstrapped_client) = &ctx.bootstrapped_client else {
        panic!("Arti enabled but no TorClient initialized!");
    };

    TransportConfig::<ClearNet, Arti> {
        client_config: ArtiClientConfig {
            client: bootstrapped_client.clone(),
        },
        server_config: None,
    }
}

pub fn transport_daemon_config(config: &Config) -> TransportConfig<Tor, Daemon> {
    let mut invalid_onion = false;

    if config.p2p.tor_net.inbound_onion && config.tor.daemon.anonymous_inbound.is_empty() {
        invalid_onion = true;
        tracing::warn!("Onion inbound is enabled yet no onion host has been defined in configuration. Inbound server disabled.");
    }

    TransportConfig::<Tor, Daemon> {
        client_config: DaemonClientConfig {
            tor_daemon: config.tor.daemon.address,
        },
        server_config: (config.p2p.tor_net.inbound_onion && !invalid_onion).then_some(
            DaemonServerConfig {
                ip: config.tor.daemon.listening_addr.ip(),
                port: config.tor.daemon.listening_addr.port(),
            },
        ),
    }
}

/// Gets the transport config for [`ClearNet`] over [`Socks`].
pub const fn transport_clearnet_daemon_config(config: &Config) -> TransportConfig<ClearNet, Socks> {
    TransportConfig {
        client_config: SocksClientConfig {
            proxy: config.tor.daemon.address,
            authentication: None,
        },
        server_config: None,
    }
}
