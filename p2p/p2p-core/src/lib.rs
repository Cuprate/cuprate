//! # Cuprate P2P Core
//!
//! This crate is general purpose P2P networking library for working with Monero. This is a low level
//! crate, which means it may seem verbose for a lot of use cases, if you want a crate that handles
//! more of the P2P logic have a look at `cuprate-p2p`.
//!
//! # Network Zones
//!
//! This crate abstracts over network zones, Tor/I2p/clearnet with the [NetworkZone] trait. Currently only clearnet is implemented: [ClearNet].
//!
//! # Usage
//!
//! ## Connecting to a peer
//!
//! ```rust
//! # use std::{net::SocketAddr, str::FromStr};
//! #
//! # use tower::ServiceExt;
//! #
//! # use cuprate_p2p_core::{
//! #    client::{ConnectRequest, Connector, HandshakerBuilder},
//! #    ClearNet, Network,
//! # };
//! # use cuprate_wire::{common::PeerSupportFlags, BasicNodeData};
//! # use cuprate_test_utils::monerod::monerod;
//! #
//! # tokio_test::block_on(async move {
//! #
//! # let _monerod = monerod::<&str>([]).await;
//! # let addr = _monerod.p2p_addr();
//! #
//! // The information about our local node.
//! let our_basic_node_data = BasicNodeData {
//!     my_port: 0,
//!     network_id: Network::Mainnet.network_id(),
//!     peer_id: 0,
//!     support_flags: PeerSupportFlags::FLUFFY_BLOCKS,
//!     rpc_port: 0,
//!     rpc_credits_per_hash: 0,
//! };
//!
//! // See [`HandshakerBuilder`] for information about the default values set, they may not be
//! // appropriate for every use case.
//! let handshaker = HandshakerBuilder::<ClearNet>::new(our_basic_node_data).build();
//!
//! // The outbound connector.
//! let mut connector = Connector::new(handshaker);
//!
//! // The connection.
//! let connection = connector
//!     .oneshot(ConnectRequest {
//!         addr,
//!         permit: None,
//!     })
//!     .await
//!     .unwrap();
//! # });
//! ```
use std::{fmt::Debug, future::Future, hash::Hash};

use futures::{Sink, Stream};

use cuprate_wire::{
    levin::LevinMessage, network_address::NetworkAddressIncorrectZone, BucketError, Message,
    NetworkAddress,
};

pub mod client;
mod constants;
pub mod error;
pub mod handles;
mod network_zones;
pub mod protocol;
pub mod services;

pub use error::*;
pub use network_zones::{ClearNet, ClearNetServerCfg};
pub use protocol::*;
use services::*;
//re-export
pub use cuprate_helper::network::Network;
pub use cuprate_wire::CoreSyncData;

/// The direction of a connection.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ConnectionDirection {
    /// An inbound connection to our node.
    Inbound,
    /// An outbound connection from our node.
    Outbound,
}

/// An address on a specific [`NetworkZone`].
pub trait NetZoneAddress:
    TryFrom<NetworkAddress, Error = NetworkAddressIncorrectZone>
    + Into<NetworkAddress>
    + std::fmt::Display
    + Hash
    + Eq
    + Copy
    + Send
    + Sync
    + Unpin
    + 'static
{
    /// Cuprate needs to be able to ban peers by IP addresses and not just by SocketAddr as
    /// that include the port, to be able to facilitate this network addresses must have a ban ID
    /// which for hidden services could just be the address it self but for clear net addresses will
    /// be the IP address.
    ///
    /// - TODO: IP zone banning?
    /// - TODO: rename this to Host.

    type BanID: Debug + Hash + Eq + Clone + Copy + Send + 'static;

    /// Changes the port of this address to `port`.
    fn set_port(&mut self, port: u16);

    /// Turns this address into its canonical form.
    fn make_canonical(&mut self);

    /// Returns the [`Self::BanID`] for this address.
    fn ban_id(&self) -> Self::BanID;

    fn should_add_to_peer_list(&self) -> bool;
}

/// An abstraction over a network zone (tor/i2p/clear)
#[async_trait::async_trait]
pub trait NetworkZone: Clone + Copy + Send + 'static {
    /// The network name.
    const NAME: &'static str;
    /// Allow syncing over this network.
    ///
    /// Not recommended for anonymity networks.
    const ALLOW_SYNC: bool;
    /// Enable dandelion++ for this network.
    ///
    /// This is unneeded on anonymity networks.
    const DANDELION_PP: bool;
    /// Check if our node ID matches the incoming peers node ID for this network.
    ///
    /// This has privacy implications on an anonymity network if true so should be set
    /// to false.
    const CHECK_NODE_ID: bool;
    /// Fixed seed nodes for this network.
    const SEEDS: &'static [Self::Addr];

    /// The address type of this network.
    type Addr: NetZoneAddress;

    /// The stream (incoming data) type for this network.
    type Stream: Stream<Item = Result<Message, BucketError>> + Unpin + Send + 'static;
    /// The sink (outgoing data) type for this network.
    type Sink: Sink<LevinMessage<Message>, Error = BucketError> + Unpin + Send + 'static;
    /// The inbound connection listener for this network.
    type Listener: Stream<Item = Result<(Option<Self::Addr>, Self::Stream, Self::Sink), std::io::Error>>
        + Send
        + 'static;
    /// Config used to start a server which listens for incoming connections.
    type ServerCfg: Clone + Debug + Send + 'static;

    /// Connects to a peer with the given address.
    ///
    /// <div class="warning">    
    ///
    /// This does not complete a handshake with the peer, to do that see the [crate](crate) docs.
    ///
    /// </div>
    ///
    /// Returns the [`Self::Stream`] and [`Self::Sink`] to send messages to the peer.
    async fn connect_to_peer(
        addr: Self::Addr,
    ) -> Result<(Self::Stream, Self::Sink), std::io::Error>;

    async fn incoming_connection_listener(
        config: Self::ServerCfg,
        port: u16,
    ) -> Result<Self::Listener, std::io::Error>;
}

// ####################################################################################
// Below here is just helper traits, so we don't have to type out tower::Service bounds
// everywhere but still get to use tower.

pub trait PeerSyncSvc<Z: NetworkZone>:
    tower::Service<
        PeerSyncRequest<Z>,
        Response = PeerSyncResponse<Z>,
        Error = tower::BoxError,
        Future = Self::Future2,
    > + Send
    + 'static
{
    // This allows us to put more restrictive bounds on the future without defining the future here
    // explicitly.
    type Future2: Future<Output = Result<Self::Response, Self::Error>> + Send + 'static;
}

impl<T, Z: NetworkZone> PeerSyncSvc<Z> for T
where
    T: tower::Service<PeerSyncRequest<Z>, Response = PeerSyncResponse<Z>, Error = tower::BoxError>
        + Send
        + 'static,
    T::Future: Future<Output = Result<Self::Response, Self::Error>> + Send + 'static,
{
    type Future2 = T::Future;
}

pub trait AddressBook<Z: NetworkZone>:
    tower::Service<
        AddressBookRequest<Z>,
        Response = AddressBookResponse<Z>,
        Error = tower::BoxError,
        Future = Self::Future2,
    > + Send
    + 'static
{
    // This allows us to put more restrictive bounds on the future without defining the future here
    // explicitly.
    type Future2: Future<Output = Result<Self::Response, Self::Error>> + Send + 'static;
}

impl<T, Z: NetworkZone> AddressBook<Z> for T
where
    T: tower::Service<
            AddressBookRequest<Z>,
            Response = AddressBookResponse<Z>,
            Error = tower::BoxError,
        > + Send
        + 'static,
    T::Future: Future<Output = Result<Self::Response, Self::Error>> + Send + 'static,
{
    type Future2 = T::Future;
}

pub trait CoreSyncSvc:
    tower::Service<
        CoreSyncDataRequest,
        Response = CoreSyncDataResponse,
        Error = tower::BoxError,
        Future = Self::Future2,
    > + Send
    + 'static
{
    // This allows us to put more restrictive bounds on the future without defining the future here
    // explicitly.
    type Future2: Future<Output = Result<Self::Response, Self::Error>> + Send + 'static;
}

impl<T> CoreSyncSvc for T
where
    T: tower::Service<
            CoreSyncDataRequest,
            Response = CoreSyncDataResponse,
            Error = tower::BoxError,
        > + Send
        + 'static,
    T::Future: Future<Output = Result<Self::Response, Self::Error>> + Send + 'static,
{
    type Future2 = T::Future;
}

pub trait ProtocolRequestHandler:
    tower::Service<
        ProtocolRequest,
        Response = ProtocolResponse,
        Error = tower::BoxError,
        Future = Self::Future2,
    > + Send
    + 'static
{
    // This allows us to put more restrictive bounds on the future without defining the future here
    // explicitly.
    type Future2: Future<Output = Result<Self::Response, Self::Error>> + Send + 'static;
}

impl<T> ProtocolRequestHandler for T
where
    T: tower::Service<ProtocolRequest, Response = ProtocolResponse, Error = tower::BoxError>
        + Send
        + 'static,
    T::Future: Future<Output = Result<Self::Response, Self::Error>> + Send + 'static,
{
    type Future2 = T::Future;
}
