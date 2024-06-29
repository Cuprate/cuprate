//! # Monero P2P
//!
//! This crate is general purpose P2P networking library for working with Monero. This is a low level
//! crate, which means it may seem verbose for a lot of use cases, if you want a crate that handles
//! more of the P2P logic have a look at `cuprate-p2p`.
//!
//! # Network Zones
//!
//! This crate abstracts over network zones, Tor/I2p/clearnet with the [NetworkZone] trait. Currently only clearnet is implemented: [ClearNet](network_zones::ClearNet).
//!
//! # Usage
//!
//! TODO
//!
use std::{fmt::Debug, future::Future, hash::Hash, pin::Pin};

use futures::{Sink, Stream};

use cuprate_wire::{
    levin::LevinMessage, network_address::NetworkAddressIncorrectZone, BucketError, Message,
    NetworkAddress,
};

pub mod client;
mod constants;
pub mod error;
pub mod handles;
pub mod network_zones;
pub mod protocol;
pub mod services;

pub use error::*;
pub use protocol::*;
use services::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ConnectionDirection {
    InBound,
    OutBound,
}

#[cfg(not(feature = "borsh"))]
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
    /// TODO: IP zone banning?
    type BanID: Debug + Hash + Eq + Clone + Copy + Send + 'static;

    /// Changes the port of this address to `port`.
    fn set_port(&mut self, port: u16);

    fn make_canonical(&mut self);

    fn ban_id(&self) -> Self::BanID;

    fn should_add_to_peer_list(&self) -> bool;
}

#[cfg(feature = "borsh")]
pub trait NetZoneAddress:
    TryFrom<NetworkAddress, Error = NetworkAddressIncorrectZone>
    + Into<NetworkAddress>
    + std::fmt::Display
    + borsh::BorshSerialize
    + borsh::BorshDeserialize
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
    /// TODO: IP zone banning?
    type BanID: Debug + Hash + Eq + Clone + Copy + Send + 'static;

    /// Changes the port of this address to `port`.
    fn set_port(&mut self, port: u16);

    fn make_canonical(&mut self);

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
        Future = Pin<
            Box<
                dyn Future<Output = Result<CoreSyncDataResponse, tower::BoxError>> + Send + 'static,
            >,
        >,
    > + Send
    + 'static
{
}

impl<T> CoreSyncSvc for T where
    T: tower::Service<
            CoreSyncDataRequest,
            Response = CoreSyncDataResponse,
            Error = tower::BoxError,
            Future = Pin<
                Box<
                    dyn Future<Output = Result<CoreSyncDataResponse, tower::BoxError>>
                        + Send
                        + 'static,
                >,
            >,
        > + Send
        + 'static
{
}

pub trait PeerRequestHandler:
    tower::Service<
        PeerRequest,
        Response = PeerResponse,
        Error = tower::BoxError,
        Future = Pin<
            Box<dyn Future<Output = Result<PeerResponse, tower::BoxError>> + Send + 'static>,
        >,
    > + Send
    + 'static
{
}

impl<T> PeerRequestHandler for T where
    T: tower::Service<
            PeerRequest,
            Response = PeerResponse,
            Error = tower::BoxError,
            Future = Pin<
                Box<dyn Future<Output = Result<PeerResponse, tower::BoxError>> + Send + 'static>,
            >,
        > + Send
        + 'static
{
}
