
use std::collections::HashMap;

use tokio::task::JoinHandle;
use tower::{discover::Discover, BoxError};
use futures::{channel::{mpsc, oneshot}, stream::FuturesUnordered};


use monero_wire::NetworkAddress;

use crate::{peer::LoadTrackedClient, protocol::InternalMessageRequest};
use super::UnreadyService;


/// A signal sent by the [`PeerSet`] when it has no ready peers, and gets a request from Zebra.
///
/// In response to this signal, the crawler tries to open more peer connections.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MorePeers;

/// A signal sent by the [`PeerSet`] to cancel a [`Client`][1]'s current request
/// or response.
///
/// When it receives this signal, the [`Client`][1] stops processing and exits.
///
/// [1]: crate::peer::Client
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CancelClientWork;

/// A [`tower::Service`] that abstractly represents "the rest of the network".
///
/// # Security
///
/// The `Discover::Key` must be the transient remote address of each peer. This
/// address may only be valid for the duration of a single connection. (For
/// example, inbound connections have an ephemeral remote port, and proxy
/// connections have an ephemeral local or proxy port.)
///
/// Otherwise, malicious peers could interfere with other peers' `PeerSet` state.
pub struct PeerSet<D>
where
    D: Discover<Key = NetworkAddress, Service = LoadTrackedClient> + Unpin,
    D::Error: Into<BoxError>,
{
    // Peer Tracking: New Peers
    //
    /// Provides new and deleted peer [`Change`]s to the peer set,
    /// via the [`Discover`] trait implementation.
    discover: D,

    /// A channel that asks the peer crawler task to connect to more peers.
    demand_signal: mpsc::Sender<MorePeers>,

    // Peer Tracking: Ready Peers
    //
    /// Connected peers that are ready to receive requests from Zebra,
    /// or send requests to Zebra.
    ready_services: HashMap<D::Key, D::Service>,

    // Request Routing
    //
    /// A preselected ready service.
    ///
    /// # Correctness
    ///
    /// If this is `Some(addr)`, `addr` must be a key for a peer in `ready_services`.
    /// If that peer is removed from `ready_services`, we must set the preselected peer to `None`.
    ///
    /// This is handled by [`PeerSet::take_ready_service`] and
    /// [`PeerSet::disconnect_from_outdated_peers`].
    preselected_p2c_peer: Option<D::Key>,

    // Peer Tracking: Busy Peers
    //
    /// Connected peers that are handling a Zebra request,
    /// or Zebra is handling one of their requests.
    unready_services: FuturesUnordered<UnreadyService<D::Key, D::Service, InternalMessageRequest>>,

    /// Channels used to cancel the request that an unready service is doing.
    cancel_handles: HashMap<D::Key, oneshot::Sender<CancelClientWork>>,

    /// The configured limit for inbound and outbound connections.
    ///
    /// The peer set panics if this size is exceeded.
    /// If that happens, our connection limit code has a bug.
    peerset_total_connection_limit: usize,

    // Background Tasks
    //
    /// Channel for passing ownership of tokio JoinHandles from PeerSet's background tasks
    ///
    /// The join handles passed into the PeerSet are used populate the `guards` member
    handle_rx: tokio::sync::oneshot::Receiver<Vec<JoinHandle<Result<(), BoxError>>>>,

    /// Unordered set of handles to background tasks associated with the `PeerSet`
    ///
    /// These guards are checked for errors as part of `poll_ready` which lets
    /// the `PeerSet` propagate errors from background tasks back to the user
    guards: futures::stream::FuturesUnordered<JoinHandle<Result<(), BoxError>>>,
}