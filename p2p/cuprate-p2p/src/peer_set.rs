//! This module contains the peer set and related functionality.
//!
use std::{
    cmp::Ordering,
    future::{ready, Future},
    pin::Pin,
    task::{Context, Poll},
};

use futures::{stream::FuturesUnordered, FutureExt, StreamExt, TryFutureExt};
use indexmap::{IndexMap, IndexSet};
use rand::prelude::*;
use tokio::sync::mpsc;
use tower::{load::Load, Service, ServiceExt};

use monero_p2p::{
    client::{Client, InternalPeerID, PeakEwmaClient},
    handles::ConnectionHandle,
    ConnectionDirection, NetworkZone, PeerRequest, PeerResponse,
};

use crate::connection_maintainer::MakeConnectionRequest;

mod drop_guard_client;
mod pending_svc;

use pending_svc::PendingService;

pub enum PeerSetRequest<N: NetworkZone> {
    /// Returns a ready peer using a load balancing algorithm.
    //TODO?: LoadBalancedPeer,
    /// Returns a ready peer using a load balancing algorithm across the given sub-set of peers.
    /// If a peer in the sub-set is not in the peer list then it is ignored and the peers chosen
    /// will be between the rest of the sub set until there are no peers to select from.
    //TODO?: LoadBalancedPeerSubSet(HashSet<InternalPeerID<N::Addr>>),
    /// Chooses a peer from the provided sub-set of peers using a load balancing algorithm, then sends
    /// a request to that peer.
    ///
    /// If a peer in the sub-set is not in the peer list then it is ignored and the peers chosen
    /// will be between the rest of the sub set until there are no peers to select from, then an error
    /// is returned.
    LoadBalancedPeerSubSetRequest {
        peers: Vec<InternalPeerID<N::Addr>>,
        req: PeerRequest,
    },
    RequestToSpecificPeer {
        peer: InternalPeerID<N::Addr>,
        req: PeerRequest,
    },
}

pub enum PeerSetResponse<N: NetworkZone> {
    PeerResponse(PeerResponse, InternalPeerID<N::Addr>, ConnectionHandle),
}

/// The peer-set.
///
/// This struct holds peers currently connected on a certain [`NetworkZone`].
/// The peer-set is what the routing methods use to get peers.
pub struct PeerSet<N: NetworkZone> {
    new_peers_rx: mpsc::Receiver<Client<N>>,
    /// A channel to the outbound connection maker to make a new connection
    make_new_connections_tx: mpsc::Sender<MakeConnectionRequest>,

    outbound_peers: IndexSet<InternalPeerID<N::Addr>>,

    ready_peers: IndexMap<InternalPeerID<N::Addr>, PeakEwmaClient<N>>,

    pending_peers: FuturesUnordered<PendingService<N>>,
}

impl<N: NetworkZone> PeerSet<N> {
    pub fn new(
        new_peers_rx: mpsc::Receiver<Client<N>>,
        make_new_connections_tx: mpsc::Sender<MakeConnectionRequest>,
    ) -> Self {
        Self {
            new_peers_rx,
            make_new_connections_tx,
            outbound_peers: IndexSet::new(),
            ready_peers: IndexMap::new(),
            pending_peers: FuturesUnordered::new(),
        }
    }

    fn remove_peer(&mut self, id: &InternalPeerID<N::Addr>) {
        tracing::debug!("Removing Peer: {} from PeerSet", id);

        self.outbound_peers.swap_remove(id);

        self.ready_peers.swap_remove(id);
    }

    fn poll_pending_peers(&mut self, cx: &mut Context<'_>) {
        while let Poll::Ready(Some(res)) = self.pending_peers.poll_next_unpin(cx) {
            if let Ok(client) = res {
                tracing::trace!("Client {} is ready again for requests", client.info.id);

                if client.info.direction == ConnectionDirection::OutBound {
                    self.outbound_peers.insert(client.info.id);
                }

                self.ready_peers.insert(client.info.id, client);
            }
        }
    }

    fn poll_new_peers(&mut self, cx: &mut Context<'_>) {
        tracing::trace!("Polling new peer channel.");
        while let Poll::Ready(Some(client)) = self.new_peers_rx.poll_recv(cx) {
            tracing::trace!("Received new peer to add to peer set: {}", client.info.id);

            let mut client = PeakEwmaClient::new(client);

            if client.poll_ready(cx).is_ready() {
                if client.info.direction == ConnectionDirection::OutBound {
                    self.outbound_peers.insert(client.info.id);
                }

                self.ready_peers.insert(client.info.id, client);
            } else {
                self.pending_peers.push(PendingService::new(client));
            }
        }
    }

    fn p2c_peer(
        &mut self,
        p1: InternalPeerID<N::Addr>,
        p2: InternalPeerID<N::Addr>,
    ) -> Option<InternalPeerID<N::Addr>> {
        // Get the first random peer.
        let peer_1_client = self.ready_peers.get_mut(&p1)?;

        // Check the peer is ready and has not had an error.
        if !check_client_ok(peer_1_client) {
            tracing::debug!("Peer {} had an error or was not ready.", p1);
            self.remove_peer(&p1);
            return None;
        }
        // Get peer1's load.
        let peer_1_load = peer_1_client.load();

        // Get the second random peer.
        let peer_2_client = self.ready_peers.get_mut(&p2)?;
        // Check the peer is ready and has not had an error.
        if !check_client_ok(peer_2_client) {
            tracing::debug!("Peer {} had an error or was not ready.", p2);
            self.remove_peer(&p2);
            return None;
        }
        // Get peer2's load.
        let peer_2_load = peer_2_client.load();

        tracing::trace!(
            "Selecting less loaded peer between p1: {} l: {:?}, p2: {}, l: {:?}",
            p1,
            peer_1_load,
            p2,
            peer_2_load
        );

        let peer = match peer_1_load.partial_cmp(&peer_2_load) {
            None | Some(Ordering::Less) | Some(Ordering::Equal) => p1,
            Some(Ordering::Greater) => p2,
        };

        tracing::debug!("Selected P2C peer: {}", peer);
        Some(peer)
    }

    /// Returns a peer address selected from a load balancing algorithm.
    fn load_balanced_peer(&mut self) -> Option<InternalPeerID<N::Addr>> {
        loop {
            match self.ready_peers.len() {
                0 => return None,
                1 => {
                    let (&addr, client) = self.ready_peers.get_index_mut(0).unwrap();

                    if !check_client_ok(client) {
                        tracing::debug!("Peer {} had an error or was not ready.", addr);
                        self.remove_peer(&addr);
                        continue;
                    }

                    return Some(addr);
                }
                _ => {
                    let indexes =
                        rand::seq::index::sample(&mut thread_rng(), self.ready_peers.len(), 2);

                    let Some(peer) = self.p2c_peer(
                        *self.ready_peers.get_index(indexes.index(0)).unwrap().0,
                        *self.ready_peers.get_index(indexes.index(1)).unwrap().0,
                    ) else {
                        continue;
                    };

                    return Some(peer);
                }
            }
        }
    }

    fn load_balanced_sub_set(
        &mut self,
        mut sub_set: Vec<InternalPeerID<N::Addr>>,
    ) -> Option<InternalPeerID<N::Addr>> {
        loop {
            match sub_set.len() {
                0 => return None,
                1 => {
                    let addr = sub_set.pop().unwrap();

                    let client = self.ready_peers.get_mut(&addr)?;

                    if !check_client_ok(client) {
                        tracing::debug!("Peer {} had an error or was not ready.", addr);
                        self.remove_peer(&addr);
                        continue;
                    }

                    return Some(addr);
                }
                _ => {
                    let indexes = rand::seq::index::sample(&mut thread_rng(), sub_set.len(), 2);

                    let p1 = sub_set[indexes.index(0)];
                    let p2 = sub_set[indexes.index(1)];

                    if self.ready_peers.get(&p1).is_none() {
                        sub_set.swap_remove(indexes.index(0));
                        continue;
                    }

                    if self.ready_peers.get(&p2).is_none() {
                        sub_set.swap_remove(indexes.index(1));
                        continue;
                    }

                    let Some(peer) = self.p2c_peer(p1, p2) else {
                        continue;
                    };

                    return Some(peer);
                }
            }
        }
    }

    pub fn get_peer(&mut self, addr: InternalPeerID<N::Addr>) -> Option<&mut PeakEwmaClient<N>> {
        self.ready_peers.get_mut(&addr)
    }

    pub fn take_peer(&mut self, addr: &InternalPeerID<N::Addr>) -> Option<PeakEwmaClient<N>> {
        let peer = self.ready_peers.swap_remove(addr)?;

        match peer.info.direction {
            ConnectionDirection::InBound => (),
            ConnectionDirection::OutBound => {
                self.outbound_peers.swap_remove(addr);
            }
        };

        Some(peer)
    }

    pub fn number_ready(&self) -> usize {
        self.ready_peers.len()
    }
}

fn check_client_ok<N: NetworkZone>(client: &mut PeakEwmaClient<N>) -> bool {
    let Some(res) = client.ready().now_or_never() else {
        // TODO: This should not happen but for now make this a warning just in case it does.
        tracing::warn!("Peer was not ready in peer set when it should be.");
        return false;
    };

    res.is_ok()
}

impl<N: NetworkZone> Service<PeerSetRequest<N>> for PeerSet<N> {
    type Response = PeerSetResponse<N>;
    type Error = tower::BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_new_peers(cx);
        self.poll_pending_peers(cx);

        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: PeerSetRequest<N>) -> Self::Future {
        match req {
            PeerSetRequest::LoadBalancedPeerSubSetRequest { peers, req } => {
                let Some(peer) = self.load_balanced_sub_set(peers) else {
                    return ready(Err("No peers to connect to".into())).boxed();
                };

                let mut client = self.take_peer(&peer).unwrap();

                let fut = client.call(req);

                let handle = client.info.handle.clone();
                let id = client.info.id;

                let fut = fut
                    .map_ok(move |res| PeerSetResponse::PeerResponse(res, id, handle))
                    .boxed();

                self.pending_peers.push(PendingService::new(client));

                return fut;
            }
            PeerSetRequest::RequestToSpecificPeer { peer, req } => {
                let Some(mut client) = self.take_peer(&peer) else {
                    return ready(Err("Peer not connected or ready".into())).boxed();
                };

                let fut = client.call(req);

                let handle = client.info.handle.clone();
                let id = client.info.id;

                let fut = fut
                    .map_ok(move |res| PeerSetResponse::PeerResponse(res, id, handle))
                    .boxed();

                self.pending_peers.push(PendingService::new(client));

                return fut;
            }
        }
    }
}
