#![allow(unused)]

use std::{future::Future, pin::Pin};

use futures::{Sink, Stream};

use monero_wire::{
    network_address::NetworkAddressIncorrectZone, BucketError, Message, NetworkAddress,
};

pub mod client;
pub mod error;
pub mod network_zones;
pub mod protocol;
pub mod services;

pub use error::*;
pub use protocol::*;
use services::*;

const MAX_PEERS_IN_PEER_LIST_MESSAGE: usize = 250;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ConnectionDirection {
    InBound,
    OutBound,
}

/// An abstraction over a network zone (tor/i2p/clear)
#[async_trait::async_trait]
pub trait NetworkZone: Clone + Send + 'static {
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

    /// The address type of this network.
    type Addr: TryFrom<NetworkAddress, Error = NetworkAddressIncorrectZone>
        + Into<NetworkAddress>
        + std::fmt::Display
        + Clone
        + Send
        + 'static;
    /// The stream (incoming data) type for this network.
    type Stream: Stream<Item = Result<Message, BucketError>> + Unpin + Send + 'static;
    /// The sink (outgoing data) type for this network.
    type Sink: Sink<Message, Error = BucketError> + Unpin + Send + 'static;
    /// Config used to start a server which listens for incoming connections.
    type ServerCfg;

    async fn connect_to_peer(
        addr: Self::Addr,
    ) -> Result<(Self::Stream, Self::Sink), std::io::Error>;

    async fn incoming_connection_listener(config: Self::ServerCfg) -> ();
}

pub(crate) trait AddressBook<Z: NetworkZone>:
    tower::Service<
        AddressBookRequest<Z>,
        Response = AddressBookResponse<Z>,
        Error = tower::BoxError,
        Future = Pin<
            Box<
                dyn Future<Output = Result<AddressBookResponse<Z>, tower::BoxError>>
                    + Send
                    + 'static,
            >,
        >,
    > + Send
    + 'static
{
}

impl<T, Z: NetworkZone> AddressBook<Z> for T where
    T: tower::Service<
            AddressBookRequest<Z>,
            Response = AddressBookResponse<Z>,
            Error = tower::BoxError,
            Future = Pin<
                Box<
                    dyn Future<Output = Result<AddressBookResponse<Z>, tower::BoxError>>
                        + Send
                        + 'static,
                >,
            >,
        > + Send
        + 'static
{
}

pub(crate) trait CoreSyncSvc:
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

pub(crate) trait PeerRequestHandler:
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
