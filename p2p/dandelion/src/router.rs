use std::future::Future;
use std::pin::{pin, Pin};
use std::task::ready;
use std::{
    collections::HashMap,
    hash::Hash,
    task::{Context, Poll},
    time::Instant,
};

use rand::distributions::Bernoulli;
use rand::prelude::*;
use rand::thread_rng;
use tower::discover::Change;
use tower::{discover::Discover, Service};

use crate::{
    traits::{DiffuseRequest, OutboundPeers, OutboundPeersRequest, StemRequest},
    DandelionConfig, Graph,
};

#[derive(thiserror::Error, Debug)]
pub enum DandelionRouterError {
    /// This error is probably recoverable so the request should be retried.
    #[error("Peer chosen to route stem txs to had an err: {0}.")]
    PeerError(tower::BoxError),
    /// The broadcast service returned an error.
    #[error("Broadcast service returned an err: {0}.")]
    BroadcastError(tower::BoxError),
    /// The outbound peer discoverer returned an error, this is critical.
    #[error("The outbound peer discoverer returned an err: {0}.")]
    OutboundPeerDiscoverError(tower::BoxError),
    /// The outbound peer discoverer returned [`None`].
    #[error("The outbound peer discoverer exited.")]
    OutboundPeerDiscoverExited,
}

/// The current dandelion++ state.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum State {
    /// Fluff state, in this state we are diffusing stem transactions to all peers.
    Fluff,
    /// Stem state, in this state we are stemming stem transactions to a single outbound peer.
    Stem,
}

/// The routing state of a transaction.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TxState<ID> {
    /// Fluff state.
    Fluff,
    /// Stem state.
    Stem {
        /// The peer who sent us this transaction's ID.
        from: ID,
    },
    /// Local - the transaction originated from our node.
    Local,
}

/// A request to route a transaction.
pub struct DandelionRouteReq<Tx, ID> {
    /// The transaction.
    tx: Tx,
    /// The transaction state.
    state: TxState<ID>,
}

/// The dandelion router service.
pub struct DandelionRouter<P, B, ID, S> {
    /// A [`Discover`] where we can get outbound peers from.
    outbound_peer_discover: Pin<Box<P>>,
    /// A [`Service`] which handle broadcasting (diffusing) transactions.
    broadcast_svc: B,

    /// The current state.
    current_state: State,
    /// The time at which this epoch started.
    epoch_start: Instant,

    /// The stem our local transactions will be sent to.
    local_route: Option<ID>,
    /// A [`HashMap`] linking peer's IDs to IDs in `stem_peers`.
    stem_routes: HashMap<ID, ID>,
    /// Peers we are using for stemming.
    ///
    /// This will contain peers, even in [`State::Fluff`] to allow us to stem [`TxState::Local`]
    /// transactions.
    stem_peers: HashMap<ID, S>,

    /// The distribution to sample to get the [`State`], true is [`State::Fluff`].
    state_dist: Bernoulli,

    /// The config.
    config: DandelionConfig,
}

impl<P, B, ID, S> DandelionRouter<P, B, ID, S>
where
    ID: Hash + Eq,
    P: Discover<Key = ID, Service = S, Error = tower::BoxError>,
{
    /// This function gets the number of outbound peers from the [`Discover`] required for the selected [`Graph`].
    fn poll_prepare_graph(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), DandelionRouterError>> {
        let peers_needed = match self.current_state {
            State::Stem => self.config.number_of_stems(),
            // When in the fluff state we only need one peer, the one for our txs.
            State::Fluff => 1,
        };

        while self.stem_peers.len() < peers_needed {
            match ready!(self
                .outbound_peer_discover
                .as_mut()
                .poll_discover(cx)
                .map_err(DandelionRouterError::OutboundPeerDiscoverError))
            .ok_or(DandelionRouterError::OutboundPeerDiscoverExited)??
            {
                Change::Insert(key, svc) => {
                    self.stem_peers.insert(key, svc);
                }
                Change::Remove(key) => {
                    self.stem_peers.remove(&key);
                }
            }
        }

        Poll::Ready(Ok(()))
    }
}

/*
## Generics ##

Tx: The tx type
ID: Peer Id type - unique identifier for nodes.
P: Peer Set discover - where we can get outbound peers from
B: Broadcast service - where we send txs to get diffused.
S: The Peer service - handles routing messages to a single node.
 */
impl<Tx, ID, P, B, S> Service<DandelionRouteReq<Tx, ID>> for DandelionRouter<P, B, ID, S>
where
    ID: Hash + Eq + Clone,
    P: Discover<Key = ID, Service = S, Error = tower::BoxError>,
    B: Service<DiffuseRequest<Tx>, Error = tower::BoxError>,
    S: Service<StemRequest<Tx>, Error = tower::BoxError>,
{
    type Response = State;
    type Error = DandelionRouterError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.epoch_start.elapsed() > self.config.epoch_duration {
            self.stem_peers.clear();
            self.stem_routes.clear();
            self.local_route.take();

            self.current_state = if self.state_dist.sample(&mut thread_rng()) {
                State::Fluff
            } else {
                State::Stem
            };

            self.epoch_start = Instant::now();
        }

        let mut failed_peers_ids = Vec::new();

        for (peer_id, peer_svc) in &mut self.stem_peers {
            if ready!(peer_svc.poll_ready(cx)).is_err() {
                failed_peers_ids.push(peer_id.clone());
            }
        }

        for failed_peer in failed_peers_ids {
            self.stem_peers.remove(&failed_peer);
        }

        // now we have removed the failed peers check if we still have enough for the graph chosen.
        ready!(self.poll_prepare_graph(cx)?);
        ready!(self.broadcast_svc.poll_ready(cx)?);

        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: DandelionRouteReq<Tx, ID>) -> Self::Future {
        todo!();
        /*
        let fut = match req.state {
            TxState::Fluff => Box::pin( self.broadcast_svc.call(DiffuseRequest(req.tx))),
            TxState::Stem {from} => {
                let stem_route = self.stem_routes.entry(&from).or_insert_with(|| self.stem_peers.keys().choose(&mut thread_rng()).clone());

                Box::pin(self.stem_peers.get_mut(stem_route).unwrap().call(StemRequest(req.tx)))
            }
        }

         */
    }
}
