use std::{
    collections::HashMap,
    future::Future,
    hash::Hash,
    marker::PhantomData,
    pin::Pin,
    task::{ready, Context, Poll},
    time::Instant,
};

use futures::TryFutureExt;
use rand::{distributions::Bernoulli, prelude::*, thread_rng};
use tower::{
    discover::{Change, Discover},
    Service,
};

use crate::{
    traits::{DiffuseRequest, StemRequest},
    DandelionConfig,
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
pub enum State {
    /// Fluff state, in this state we are diffusing stem transactions to all peers.
    Fluff,
    /// Stem state, in this state we are stemming stem transactions to a single outbound peer.
    Stem,
}

/// The routing state of a transaction.
#[derive(Debug, Clone, Eq, PartialEq)]
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
    pub tx: Tx,
    /// The transaction state.
    pub state: TxState<ID>,
}

/// The dandelion router service.
pub struct DandelionRouter<P, B, ID, S, Tx> {
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

    /// The routers tracing span.
    span: tracing::Span,

    _tx: PhantomData<Tx>,
}

impl<Tx, ID, P, B, S> DandelionRouter<P, B, ID, S, Tx>
where
    ID: Hash + Eq + Clone,
    P: Discover<Key = ID, Service = S, Error = tower::BoxError>,
    B: Service<DiffuseRequest<Tx>, Error = tower::BoxError>,
    S: Service<StemRequest<Tx>, Error = tower::BoxError>,
{
    /// This function gets the number of outbound peers from the [`Discover`] required for the selected [`Graph`](crate::Graph).
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

    fn fluff_tx(&mut self, tx: Tx) -> B::Future {
        self.broadcast_svc.call(DiffuseRequest(tx))
    }

    fn stem_tx(&mut self, tx: Tx, from: ID) -> S::Future {
        loop {
            let stem_route = self.stem_routes.entry(from.clone()).or_insert_with(|| {
                self.stem_peers
                    .iter()
                    .choose(&mut thread_rng())
                    .expect("No peers in `stem_peers` was poll_ready called?")
                    .0
                    .clone()
            });

            let Some(peer) = self.stem_peers.get_mut(stem_route) else {
                self.stem_routes.remove(&from);
                continue;
            };

            return peer.call(StemRequest(tx));
        }
    }

    fn stem_local_tx(&mut self, tx: Tx) -> S::Future {
        loop {
            let stem_route = self.local_route.get_or_insert_with(|| {
                self.stem_peers
                    .iter()
                    .choose(&mut thread_rng())
                    .expect("No peers in `stem_peers` was poll_ready called?")
                    .0
                    .clone()
            });

            let Some(peer) = self.stem_peers.get_mut(stem_route) else {
                self.local_route.take();
                continue;
            };

            return peer.call(StemRequest(tx));
        }
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
impl<Tx, ID, P, B, S> Service<DandelionRouteReq<Tx, ID>> for DandelionRouter<P, B, ID, S, Tx>
where
    ID: Hash + Eq + Clone,
    P: Discover<Key = ID, Service = S, Error = tower::BoxError>,
    B: Service<DiffuseRequest<Tx>, Error = tower::BoxError>,
    B::Future: Send + 'static,
    S: Service<StemRequest<Tx>, Error = tower::BoxError>,
    S::Future: Send + 'static,
{
    type Response = State;
    type Error = DandelionRouterError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.epoch_start.elapsed() > self.config.epoch_duration {
            // clear all the stem routing data.
            self.stem_peers.clear();
            self.stem_routes.clear();
            self.local_route.take();

            self.current_state = if self.state_dist.sample(&mut thread_rng()) {
                State::Fluff
            } else {
                State::Stem
            };

            self.span
                .record("state", format!("{:?}", self.current_state));
            tracing::debug!(parent: &self.span, "Starting new d++ epoch",);

            self.epoch_start = Instant::now();
        }

        let mut failed_peers_ids = Vec::new();

        for (peer_id, peer_svc) in &mut self.stem_peers {
            if ready!(peer_svc.poll_ready(cx)).is_err() {
                failed_peers_ids.push(peer_id.clone());
            }
        }

        for failed_peer in failed_peers_ids {
            tracing::debug!(parent: &self.span, "Peer returned an error on `poll_ready`, removing from router.");
            self.stem_peers.remove(&failed_peer);
        }

        // now we have removed the failed peers check if we still have enough for the graph chosen.
        ready!(self.poll_prepare_graph(cx)?);

        ready!(self
            .broadcast_svc
            .poll_ready(cx)
            .map_err(DandelionRouterError::BroadcastError)?);

        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: DandelionRouteReq<Tx, ID>) -> Self::Future {
        tracing::trace!(parent: &self.span,  "Handling route request.");

        match req.state {
            TxState::Fluff => Box::pin(
                self.fluff_tx(req.tx)
                    .map_ok(|_| State::Fluff)
                    .map_err(DandelionRouterError::BroadcastError),
            ),
            TxState::Stem { from } => match self.current_state {
                State::Fluff => {
                    tracing::debug!(parent: &self.span, "Fluffing stem tx.");

                    Box::pin(
                        self.fluff_tx(req.tx)
                            .map_ok(|_| State::Fluff)
                            .map_err(DandelionRouterError::BroadcastError),
                    )
                }
                State::Stem => {
                    tracing::trace!(parent: &self.span, "Steming transaction");

                    Box::pin(
                        self.stem_tx(req.tx, from)
                            .map_ok(|_| State::Stem)
                            .map_err(DandelionRouterError::PeerError),
                    )
                }
            },
            TxState::Local => {
                tracing::debug!(parent: &self.span, "Steming local tx.");

                Box::pin(
                    self.stem_local_tx(req.tx)
                        .map_ok(|_| State::Stem)
                        .map_err(DandelionRouterError::PeerError),
                )
            }
        }
    }
}

/// A [`DandelionRouter`] builder.
///
/// You _must_ call all functions, no defaults are set.
#[derive(Debug)]
pub struct DandelionRouterBuilder<P, B, Tx> {
    config: Option<DandelionConfig>,
    /// A [`Discover`] where we can get outbound peers from.
    outbound_peer_discover: Option<P>,
    /// A [`Service`] which handle broadcasting (diffusing) transactions.
    broadcast_svc: Option<B>,

    _tx: PhantomData<Tx>,
}

impl<P, B, Tx> Default for DandelionRouterBuilder<P, B, Tx> {
    fn default() -> Self {
        Self {
            config: None,
            outbound_peer_discover: None,
            broadcast_svc: None,

            _tx: PhantomData,
        }
    }
}

impl<ID, S, P, B, Tx> DandelionRouterBuilder<P, B, Tx>
where
    ID: Hash + Eq + Clone,
    P: Discover<Key = ID, Service = S, Error = tower::BoxError>,
    B: Service<DiffuseRequest<Tx>, Error = tower::BoxError>,
    B::Future: Send + 'static,
    S: Service<StemRequest<Tx>, Error = tower::BoxError>,
    S::Future: Send + 'static,
{
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_config(self, config: DandelionConfig) -> Self {
        Self {
            config: Some(config),
            ..self
        }
    }

    pub fn with_outbound_peer_discover(self, outbound_peer_discover: P) -> Self {
        Self {
            outbound_peer_discover: Some(outbound_peer_discover),
            ..self
        }
    }

    pub fn with_broadcast_svc(self, broadcast_svc: B) -> Self {
        Self {
            broadcast_svc: Some(broadcast_svc),
            ..self
        }
    }

    pub fn build(self) -> DandelionRouter<P, B, ID, S, Tx> {
        let config = self.config.unwrap();

        // get the current state
        let state_dist = Bernoulli::new(config.fluff_probability)
            .expect("Fluff probability was not between 0 and 1");

        let current_state = if state_dist.sample(&mut thread_rng()) {
            State::Fluff
        } else {
            State::Stem
        };

        DandelionRouter {
            outbound_peer_discover: Box::pin(self.outbound_peer_discover.unwrap()),
            broadcast_svc: self.broadcast_svc.unwrap(),
            current_state,
            epoch_start: Instant::now(),
            local_route: None,
            stem_routes: HashMap::new(),
            stem_peers: HashMap::new(),
            state_dist,
            config,
            span: tracing::debug_span!("dandelion_router", state = ?current_state),
            _tx: PhantomData,
        }
    }
}
