# 🟢 Tor

## Overview

Cuprate can connect to the Tor network using either Arti or an external daemon that exposes a SOCKS5 interface. They are categorized as modes of connection, and Cuprate can use them to also anonymize clearnet connections.

The Tor implementation is concentrated into four main crates. In order of relevance:
1. **cuprated**: contains the configuration, initialization, and Dandelion support for Tor.
2. **cuprate-p2p-transport**: defines all the transport logic necessary for establishing and accepting connections over the two modes.
3. **cuprate-p2p-core**: defines the Tor Zone address.
4. **cuprate-wire**: defines an onion address.

The dependency graph is the following and will define the order in which we will treat the topic:
```
cuprate-wire -> cuprate-p2p-core -> cuprate-p2p-transport -> cuprated
```

## Onion addresses

The first building block of the Tor implementation is the `OnionAddr` type, which is necessary for the definition of the `Tor` network zone.
```rust
/// A v3, `Copy`able onion address.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize)]
pub struct OnionAddr {
    /// 56 characters encoded onion v3 domain without the .onion suffix
    /// <https://spec.torproject.org/rend-spec/encoding-onion-addresses.html>
    domain: [u8; 56],
    /// Virtual port of the peer
    pub port: u16,
}
```

Only V3 onion addresses are supported. The domain is directly written as bytes and not as a string to avoid allocation. Counterintuitively, and in opposition to the Tor project's standard definition, Cuprate calls an onion address the combination of a domain and a virtual port. This is done on purpose to keep the type name clear while acknowledging the uselessness of not having these two components simultaneously.

The port is a public field, but the domain is private, as it requires basic validation. Cuprate makes sure to check the length and character set of an externally supplied domain. This mirrors the behavior of monerod. Similarly, Cuprate does not perform the standard checksum validation. This is not an issue, as trying to connect to these incorrect onion addresses would be immediately rejected.

## Tor Zone

```rust
#[derive(Clone, Copy)]
pub struct Tor;

impl NetworkZone for Tor {
    const NAME: &'static str = "Tor";

    const CHECK_NODE_ID: bool = false;

    const BROADCAST_OWN_ADDR: bool = true;

    type Addr = OnionAddr;
}
```

Aside from the onion address type being used:
- the `CHECK_NODE_ID` is set to `false`, as all Tor peers have the exact same peer ID (0).
- The `BROADCAST_OWN_ADDR` is set to `true`, since incoming connections are not identifiable and therefore not routable. Cuprate, like all conforming Monero agents, will therefore include its own address in the peer list being sent to a peer.

A sharp reader will observe that this configuration is the complete opposite of ClearNet. This is because these constants have been made consequently to the limitations observed in following the, at the time, (and unfortunately still actual) P2P protocol over anonymous networks instead of the internet. Maybe one day, a new network type will be supported for varying the combinations of these two booleans.

## Transport

The Tor implementation originally caused the separation of the connection methods from the zone definition. This is because several transport protocols could be used for the same network. Thus, the distinction between Zone, which relates to addressing, and Transport, which relates to the protocol used to connect to an address.

Within the `cuprate-p2p-transport`, the following Tor-related transports are defined:

- `impl Transport<ClearNet> for Arti`
- `impl Transport<Tor> for Arti`
- `impl Transport<Tor> for Daemon`

There is no `Transport<ClearNet> for Daemon`, as it would be redundant. Instead, `cuprated` (will) make use of the `Transport<ClearNet> for Socks` (when available) with the Tor configuration field.

### Arti

Arti is implemented for both Tor and ClearNet zones. Both establish outgoing connections using Arti's `TorClient<R>::connect()`. However, for the Tor zone, Arti is launching an onion service prepared within cuprated, while for ClearNet, the server is simply disabled.

### Daemon

The daemon implementation is very similar to the classic TCP transport for ClearNet.
The `connect_to_peer` method establishes a TCP connection over the SOCKS5 proxy specified by the Tor daemon config. The Tor daemon makes the job of resolving the address and establishing a virtual connection.
Similarly, the `incoming_connection_listener` method is just a TCP server listening on the `tor.daemon.listening_addr`.

## Cuprated

### User configuration

Under `cuprated/src/config/tor.rs`, the `TorConfig` struct contains Tor-specific (as in non-zone related) configuration fields:

```rust
// attributes & comments trimmed.
pub struct TorConfig {
    /// Enable Tor network by specifying how to connect to it.
    /// Valid values | "Arti", "Daemon", "Off"
    pub mode: TorMode,

    /// Arti config
    pub arti: ArtiConfig,

    /// Tor Daemon config
    pub daemon: TorDaemonConfig,
}
```

The content of both `[tor.arti]` and `[tor.daemon]` is left as an exploratory exercise for the reader.

Under `cuprated/src/config/p2p.rs`, the `TorNetConfig` struct contains the Tor zone-specific fields that are added into the `[p2p.tor_net]` section:

```rust
// attributes & comments trimmed
/// The config values for P2P tor.
pub struct TorNetConfig {
    /// Enable the Tor P2P network.
    pub enabled: bool,

    #[comment_out = true]
    /// Enable Tor inbound onion server.
    ///
    /// In Arti mode, setting this to `true` will enable Arti's onion service for accepting inbound
    /// Tor P2P connections. The keypair and therefore onion address are generated randomly on the first run.
    ///
    /// In Daemon mode, setting this to `true` will enable a TCP server listening for inbound connections
    /// from your Tor daemon. Refer to the `tor.anonymous_inbound` and `tor.listening_addr` fields for onion address
    /// and listening configuration. In this mode, `p2p.tor_net.p2p_port` field is the advertised virtual port
    /// of the hidden service.
    ///
    pub inbound_onion: bool,
}
```

### Dandelion router

The main feature of enabling Tor is being able to send user transactions over Tor. This is in the realm of the Dandelion router's job and, as its name suggests, can be found within `cuprated/src/txpool/dandelion.rs`:

```rust
/// The dandelion router used to send transactions to the network.
pub(super) struct MainDandelionRouter {
    clearnet_router: ConcreteDandelionRouter<ClearNet>,
    tor_router: Option<AnonTxService<Tor>>,
}
```

The `AnonTxService` defined in `cuprated/src/txpool/dandelion/anon_net_service.rs` is a stream of anonymous peers to be selected for relaying transactions.

The `Service<DandelionRouteReq<...>>` `call` function is pretty straightforward. If a Tor router exists (meaning Tor is enabled), and if the transaction is local (meaning it is emitted by this node), then it is stemmed to a peer over the Tor network.

```rust
fn call(...) -> Self::Future {
    if req.state == TxState::Local {
        if let Some(tor_router) = self.tor_router.as_mut() {
            if let Some(mut peer) = tor_router.peer.take() {
                return peer
                    .call(StemRequest(req.tx))
                    //...
            }

            tracing::warn!(
                "failed to route tx over Tor, no connections, falling back to Clearnet"
            );
        }
    }

    self.clearnet_router.call(req)
}
```

The `AnonTxService<Tor>` should be populated soon after the Cuprate P2P zone is initialized. This leads us to the next section of code:

### Tor initialization

The initialization of the different components of Tor is achieved in three places within cuprated.

First, within `cuprated/src/main.rs`:

```rust
fn main() {
    // ...

    rt.block_on(async move {
        //...

        // Bootstrap or configure Tor if enabled.
        let tor_context = initialize_tor_if_enabled(&config).await;

        //...
    }
    // ...
}
```

The `initialize_tor_if_enabled` function (located in `cuprated/src/tor.rs`) will parse the configuration and return a `TorContext` structure:

```rust
pub struct TorContext {
    pub mode: TorMode,

    // -------- Only in Arti mode
    pub bootstrapped_client: Option<TorClient<PreferredRuntime>>,
    pub arti_client_config: Option<TorClientConfig>,
    pub arti_onion_service: Option<OnionService>,
}
```

As per the comment, the last three fields are initialized if cuprated boots with Arti mode enabled. The onion service is only created if the inbound server is enabled.

This structure's purpose is to propagate which mode of Tor, if enabled, is going to be used and the needed resources to the `p2p::initialize_zones_p2p` function.

The relevant logic to this function is the following (`cuprated/src/p2p.rs`):

```rust
// Trimmed for clarity
pub async fn initialize_zones_p2p(
    config: &Config,
    // ...
    tor_ctx: TorContext,
) -> (NetworkInterfaces, Vec<Sender<IncomingTxHandler>>) {

    // Start clearnet P2P.
    let (clearnet, incoming_tx_handler_tx) = {
        // If proxy is set
        match config.p2p.clear_net.proxy {
            ProxySettings::Tor => match tor_ctx.mode {
                TorMode::Arti => {
                    start_zone_p2p::<ClearNet, Arti>(
                        // ...
                        config.clearnet_p2p_config(),
                        transport_clearnet_arti_config(&tor_ctx),
                    )
                    .await
                    .unwrap()
                }
                TorMode::Daemon | TorMode::Off => {
                    // ...
                    std::process::exit(0);
                }
            },
            ProxySettings::Socks(ref s) => {
                // ...
            }
        }
    };

    // ...

    // Start Tor P2P (if enabled)
    let tor = if config.p2p.tor_net.enabled {
        match tor_ctx.mode {
            TorMode::Off => None,
            TorMode::Daemon => Some(
                start_zone_p2p::<Tor, Daemon>(
                    // ...
                    config.tor_p2p_config(&tor_ctx),
                    transport_daemon_config(config),
                )
                .await
                .unwrap(),
            ),
            TorMode::Arti => Some(
                start_zone_p2p::<Tor, Arti>(
                    // ...
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

    // ...
}
```

As you can see, the `TorContext` mode is checked to initialize the `ClearNet` and `Tor` zones with the correct `Transport`.
The `start_zone_p2p::<Z,T>` function requires a `P2PConfig` and `TransportConfig` structures in arguments. You can notice that these structures are returned by a few functions:

- `Config::tor_p2p_config` method from `cuprated/src/config.rs`, will return the `P2PConfig` with the help of a supplied `TorContext`.

- `transport_arti_config`, `transport_daemon_config`, `transport_clearnet_arti_config` helpers from `cuprated/src/tor.rs`, are functions that take into argument `Config` or `TorContext` or both and return `TransportConfig<Tor, Arti>`, `TransportConfig<Tor, Daemon>`, and `TransportConfig<ClearNet, Arti>` respectively.
