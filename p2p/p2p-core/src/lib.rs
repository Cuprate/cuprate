//! # Cuprate P2P Core
//!
//! This crate is general purpose P2P networking library for working with Monero. This is a low level
//! crate, which means it may seem verbose for a lot of use cases, if you want a crate that handles
//! more of the P2P logic have a look at `cuprate-p2p`.
//!
//! # Network Zones
//!
//! This crate abstracts over network zones, Tor/I2p/clearnet with the [`NetworkZone`] trait. Currently only clearnet is implemented: [`ClearNet`].
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
//! #    transports::Tcp
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
//! let handshaker = HandshakerBuilder::<ClearNet, Tcp>::new(our_basic_node_data, ()).build();
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

cfg_if::cfg_if! {
    // Used in `tests/`
    if #[cfg(test)] {
        use cuprate_test_utils as _;
        use tokio_test as _;
        use hex as _;
    }
}

use std::{
    fmt::Debug,
    hash::Hash,
    sync::{Mutex, PoisonError},
};

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
pub mod transports;
pub mod types;

pub use error::*;
pub use network_zones::{ClearNet, Tor};
pub use protocol::*;
use services::*;
//re-export
pub use cuprate_helper::network::Network;
pub use cuprate_wire::CoreSyncData;

/// Wakes the syncer.
#[derive(Debug)]
pub struct SyncerWake {
    our_cumulative_difficulty: Mutex<u128>,
    notify: tokio::sync::Notify,
}

impl SyncerWake {
    /// Create a new [`SyncerWake`] with the given initial cumulative difficulty.
    pub fn new(cumulative_difficulty: u128) -> Self {
        Self {
            our_cumulative_difficulty: Mutex::new(cumulative_difficulty),
            notify: tokio::sync::Notify::new(),
        }
    }

    /// Update our cumulative difficulty.
    pub fn set_cumulative_difficulty(&self, cd: u128) {
        *self
            .our_cumulative_difficulty
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = cd;
    }

    /// A peer reported their cumulative difficulty. Wakes the syncer if they
    /// claim to be ahead of us.
    pub fn peer_reported(&self, peer_cd: u128) {
        let our_cd = *self
            .our_cumulative_difficulty
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if peer_cd > our_cd {
            self.notify.notify_one();
        }
    }

    /// Unconditionally wake the syncer.
    pub fn wake(&self) {
        self.notify.notify_one();
    }

    /// Returns a future that completes when the syncer is woken.
    pub fn notified(&self) -> tokio::sync::futures::Notified<'_> {
        self.notify.notified()
    }
}

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
    /// Cuprate needs to be able to ban peers by IP addresses and not just by `SocketAddr` as
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
pub trait NetworkZone: Clone + Copy + Send + 'static {
    /// The network name.
    const NAME: &'static str;
    /// Check if our node ID matches the incoming peers node ID for this network.
    ///
    /// This has privacy implications on an anonymity network if true so should be set
    /// to false.
    const CHECK_NODE_ID: bool;
    /// If `true`, this network zone requires us to blend our own address and port into
    /// the address book we plan on sharing to other peers.
    const BROADCAST_OWN_ADDR: bool;

    /// The address type of this network.
    type Addr: NetZoneAddress;
}

/// An abstraction over a transport method (TCP/Tor/SOCKS5/...)
///
/// This trait implements the required methods and types for establishing connection to a
/// peer or instantiating a listener for the `NetworkZone` `Z` over a `Transport` method `T`.
///
/// Ultimately, multiple transports can implement the same trait for providing alternative
/// ways for a network zone to operate (example: [`ClearNet`] can operate on both TCP and Tor.)
#[async_trait::async_trait]
pub trait Transport<Z: NetworkZone>: Clone + Send + 'static {
    /// Client configuration necessary when establishing a connection to a peer.
    ///
    /// Note: Currently, this client config is considered immutable during operational runtime. If one
    /// wish to apply modifications on the fly, they will need to make use of an inner shared and mutable
    /// reference to do so.
    type ClientConfig: Clone + Send + Sync + 'static;
    /// Server configuration necessary when instantiating a listener for inbound connections.
    type ServerConfig: Send + Sync + 'static;

    /// The stream (incoming data) type of this transport method.
    type Stream: Stream<Item = Result<Message, BucketError>> + Unpin + Send + 'static;
    /// The sink (outgoing data) type of this transport method.
    type Sink: Sink<LevinMessage<Message>, Error = BucketError> + Unpin + Send + 'static;
    /// The inbound connection listener for this transport method.
    type Listener: Stream<Item = Result<(Option<Z::Addr>, Self::Stream, Self::Sink), std::io::Error>>
        + Send
        + 'static;

    /// Connects to a peer with the given address.
    ///
    /// Take in argument the destination [`NetworkZone::Addr`] and [`Self::ClientConfig`] which should contain mandatory parameters
    /// for a connection to be established.
    ///
    /// <div class="warning">
    ///
    /// This does not complete a handshake with the peer, to do that see the [crate](crate) docs.
    ///
    /// </div>
    ///
    /// Returns the [`Self::Stream`] and [`Self::Sink`] to send messages to the peer.
    async fn connect_to_peer(
        addr: Z::Addr,
        config: &Self::ClientConfig,
    ) -> Result<(Self::Stream, Self::Sink), std::io::Error>;

    /// Instantiate a listener for inbound peer connections
    ///
    /// Take in argument [`Self::ServerConfig`] which should contain mandatory parameters
    /// for the listener.
    ///
    /// Returns the [`Self::Listener`] to listen to new connections.
    async fn incoming_connection_listener(
        config: Self::ServerConfig,
    ) -> Result<Self::Listener, std::io::Error>;
}

// ####################################################################################
// Below here is just helper traits, so we don't have to type out tower::Service bounds
// everywhere but still get to use tower.

pub trait AddressBook<Z: NetworkZone>:
    tower::Service<
        AddressBookRequest<Z>,
        Response = AddressBookResponse<Z>,
        Error = tower::BoxError,
        Future: Send + 'static,
    > + Send
    + 'static
{
}

impl<T, Z: NetworkZone> AddressBook<Z> for T where
    T: tower::Service<
            AddressBookRequest<Z>,
            Response = AddressBookResponse<Z>,
            Error = tower::BoxError,
            Future: Send + 'static,
        > + Send
        + 'static
{
}

pub trait CoreSyncSvc:
    tower::Service<
        CoreSyncDataRequest,
        Response = CoreSyncDataResponse,
        Error = tower::BoxError,
        Future: Send + 'static,
    > + Send
    + 'static
{
}

impl<T> CoreSyncSvc for T where
    T: tower::Service<
            CoreSyncDataRequest,
            Response = CoreSyncDataResponse,
            Error = tower::BoxError,
            Future: Send + 'static,
        > + Send
        + 'static
{
}

pub trait ProtocolRequestHandler:
    tower::Service<
        ProtocolRequest,
        Response = ProtocolResponse,
        Error = tower::BoxError,
        Future: Send + 'static,
    > + Send
    + 'static
{
}

impl<T> ProtocolRequestHandler for T where
    T: tower::Service<
            ProtocolRequest,
            Response = ProtocolResponse,
            Error = tower::BoxError,
            Future: Send + 'static,
        > + Send
        + 'static
{
}

pub trait ProtocolRequestHandlerMaker<Z: NetworkZone>:
    tower::Service<
        client::PeerInformation<Z::Addr>,
        Error = tower::BoxError,
        Response: ProtocolRequestHandler,
        Future: Send + 'static,
    > + Send
    + 'static
{
}

impl<T, Z: NetworkZone> ProtocolRequestHandlerMaker<Z> for T where
    T: tower::Service<
            client::PeerInformation<Z::Addr>,
            Error = tower::BoxError,
            Response: ProtocolRequestHandler,
            Future: Send + 'static,
        > + Send
        + 'static
{
}
