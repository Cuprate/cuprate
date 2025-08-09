//! Outbound Connection Maintainer.
//!
//! This module handles maintaining the number of outbound connections defined in the [`P2PConfig`].
//! It also handles making extra connections when the peer set is under load or when we need data that
//! no connected peer has.

use futures::future::BoxFuture;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use rand::{distributions::Bernoulli, prelude::*};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::{
    sync::{mpsc, OwnedSemaphorePermit, Semaphore},
    task::JoinSet,
    time::{sleep, timeout},
};
use tower::{Service, ServiceExt};
use tracing::{instrument, Instrument, Span};

use crate::constants::OUTBOUND_ANCHOR_CONNECTION_ATTEMPT_TIMEOUT;
use crate::{
    config::P2PConfig,
    constants::{HANDSHAKE_TIMEOUT, MAX_SEED_CONNECTIONS, OUTBOUND_CONNECTION_ATTEMPT_TIMEOUT},
};
use cuprate_p2p_core::services::ZoneSpecificPeerListEntryBase;
use cuprate_p2p_core::{
    client::{Client, ConnectRequest, HandshakeError},
    services::{AddressBookRequest, AddressBookResponse},
    AddressBook, NetworkZone,
};

enum OutboundConnectorError {
    MaxConnections,
    FailedToConnectToSeeds,
    NoAvailablePeers,
}

/// A request from the peer set to make an outbound connection.
///
/// This will only be sent when the peer set is under load from the rest of Cuprate or the peer
/// set needs specific data that none of the currently connected peers have.
pub struct MakeConnectionRequest {
    /// The block needed that no connected peers have due to pruning.
    block_needed: Option<usize>,
}

/// The outbound connection count keeper.
///
/// This handles maintaining a minimum number of connections and making extra connections when needed, upto a maximum.
pub struct OutboundConnectionKeeper<Z: NetworkZone, A, C> {
    /// The channel to send new outbound connections down.
    pub new_peers_tx: mpsc::Sender<Client<Z>>,
    /// The channel that tells us to make new _extra_ outbound connections.
    pub make_connection_rx: mpsc::Receiver<MakeConnectionRequest>,
    /// The address book service
    pub address_book_svc: A,
    /// The service to connect to a specific peer.
    pub connector_svc: C,
    /// A semaphore to keep the amount of outbound peers constant.
    pub outbound_semaphore: Arc<Semaphore>,
    /// The amount of peers we connected to because we needed more peers. If the `outbound_semaphore`
    /// is full, and we need to connect to more peers for blocks or because not enough peers are ready
    /// we add a permit to the semaphore and keep track here, upto a value in config.
    pub extra_peers: usize,
    /// The p2p config.
    pub config: P2PConfig<Z>,
    /// The [`Bernoulli`] distribution, when sampled will return true if we should connect to a gray peer or
    /// false if we should connect to a white peer.
    ///
    /// This is weighted to the percentage given in `config`.
    pub peer_type_gen: Bernoulli,

    pub anchor_connection_keeper: Option<AnchorConnectionKeeper<Z, A, C>>,
}

impl<Z, A, C> OutboundConnectionKeeper<Z, A, C>
where
    Z: NetworkZone,
    A: AddressBook<Z> + Clone,
    C: Service<ConnectRequest<Z>, Response = Client<Z>, Error = HandshakeError>
        + Clone
        + Send
        + 'static,
    C::Future: Send + 'static,
{
    pub fn new(
        config: P2PConfig<Z>,
        new_peers_tx: mpsc::Sender<Client<Z>>,
        make_connection_rx: mpsc::Receiver<MakeConnectionRequest>,
        address_book_svc: A,
        connector_svc: C,
    ) -> Self {
        let peer_type_gen = Bernoulli::new(config.gray_peers_percent)
            .expect("Gray peer percent is incorrect should be 0..=1");

        let outbound_connections = config
            .outbound_connections
            .checked_sub(config.numb_anchors)
            .expect("Outbound connections should be more than or equal to anchor connections");

        Self {
            new_peers_tx: new_peers_tx.clone(),
            make_connection_rx,
            outbound_semaphore: Arc::new(Semaphore::new(outbound_connections)),
            extra_peers: 0,
            peer_type_gen,
            anchor_connection_keeper: Some(AnchorConnectionKeeper {
                new_peers_tx,
                anchor_semaphore: Arc::new(Semaphore::new(config.numb_anchors)),
                address_book_svc: address_book_svc.clone(),
                connector_svc: connector_svc.clone(),
                config: config.clone(),
            }),
            address_book_svc,
            connector_svc,
            config,
        }
    }

    /// Connects to random seeds to get peers and immediately disconnects
    #[instrument(level = "info", skip(self))]
    #[expect(clippy::significant_drop_tightening)]
    async fn connect_to_random_seeds(&mut self) -> Result<(), OutboundConnectorError> {
        let seeds = self
            .config
            .seeds
            .choose_multiple(&mut thread_rng(), MAX_SEED_CONNECTIONS);

        assert_ne!(seeds.len(), 0, "No seed nodes available to get peers from");

        let mut allowed_errors = seeds.len();

        let mut handshake_futs = JoinSet::new();

        for seed in seeds {
            tracing::info!("Getting peers from seed node: {}", seed);

            let fut = timeout(
                HANDSHAKE_TIMEOUT,
                self.connector_svc
                    .ready()
                    .await
                    .expect("Connector had an error in `poll_ready`")
                    .call(ConnectRequest {
                        addr: *seed,
                        permit: None,
                    }),
            );
            // Spawn the handshake on a separate task with a timeout, so we don't get stuck connecting to a peer.
            handshake_futs.spawn(fut);
        }

        while let Some(res) = handshake_futs.join_next().await {
            if matches!(res, Err(_) | Ok(Err(_) | Ok(Err(_)))) {
                allowed_errors -= 1;
            }
        }

        if allowed_errors == 0 {
            Err(OutboundConnectorError::FailedToConnectToSeeds)
        } else {
            Ok(())
        }
    }

    /// Handles a request from the peer set for more peers.
    #[expect(
        clippy::significant_drop_tightening,
        reason = "we need to hold onto a permit"
    )]
    async fn handle_peer_request(
        &mut self,
        req: &MakeConnectionRequest,
    ) -> Result<(), OutboundConnectorError> {
        // try to get a permit.
        let permit = Arc::clone(&self.outbound_semaphore)
            .try_acquire_owned()
            .or_else(|_| {
                // if we can't get a permit add one if we are below the max number of connections.
                if self.extra_peers >= self.config.extra_outbound_connections {
                    // If we can't add a permit return an error.
                    Err(OutboundConnectorError::MaxConnections)
                } else {
                    self.outbound_semaphore.add_permits(1);
                    self.extra_peers += 1;
                    Ok(Arc::clone(&self.outbound_semaphore)
                        .try_acquire_owned()
                        .unwrap())
                }
            })?;

        // try to get a random peer on any network zone from the address book.
        let peer = self
            .address_book_svc
            .ready()
            .await
            .expect("Error in address book!")
            .call(AddressBookRequest::TakeRandomPeer {
                height: req.block_needed,
            })
            .await;

        match peer {
            Err(_) => {
                // TODO: We should probably send peer requests to our connected peers rather than go to seeds.
                tracing::warn!("No peers in address book which are available and have the data we need. Getting peers from seed nodes.");

                self.connect_to_random_seeds().await?;
                Err(OutboundConnectorError::NoAvailablePeers)
            }

            Ok(AddressBookResponse::Peer(peer)) => {
                connect_to_outbound_peer(
                    self.new_peers_tx.clone(),
                    &mut self.connector_svc,
                    permit,
                    peer.adr,
                )
                .await;
                Ok(())
            }
            Ok(_) => panic!("peer list sent incorrect response!"),
        }
    }

    /// Handles a free permit, by either connecting to a new peer or by removing a permit if we are above the
    /// minimum number of outbound connections.
    #[instrument(level = "debug", skip(self, permit))]
    async fn handle_free_permit(
        &mut self,
        permit: OwnedSemaphorePermit,
    ) -> Result<(), OutboundConnectorError> {
        if self.extra_peers > 0 {
            tracing::debug!(
                "Permit available but we are over the minimum number of peers, forgetting permit."
            );
            permit.forget();
            self.extra_peers -= 1;
            return Ok(());
        }

        tracing::debug!("Permit available, making outbound connection.");

        let req = if self.peer_type_gen.sample(&mut thread_rng()) {
            AddressBookRequest::TakeRandomGrayPeer { height: None }
        } else {
            // This will try white peers first then gray.
            AddressBookRequest::TakeRandomPeer { height: None }
        };

        let Ok(AddressBookResponse::Peer(peer)) = self
            .address_book_svc
            .ready()
            .await
            .expect("Error in address book!")
            .call(req)
            .await
        else {
            tracing::warn!("No peers in peer list to make connection to.");
            self.connect_to_random_seeds().await?;
            return Err(OutboundConnectorError::NoAvailablePeers);
        };

        connect_to_outbound_peer(
            self.new_peers_tx.clone(),
            &mut self.connector_svc,
            permit,
            peer.adr,
        )
        .await;
        Ok(())
    }

    /// Runs the outbound connection count keeper.
    pub async fn run(mut self) {
        tracing::info!(
            "Starting outbound connection maintainer, target outbound connections: {}",
            self.config.outbound_connections
        );

        tokio::spawn(self.anchor_connection_keeper.take().unwrap().run());

        loop {
            tokio::select! {
                biased;
                peer_req = self.make_connection_rx.recv() => {
                    let Some(peer_req) = peer_req else {
                        tracing::info!("Shutting down outbound connector, make connection channel closed.");
                        return;
                    };
                    #[expect(clippy::let_underscore_must_use, reason = "We can't really do much about errors in this function.")]
                    let _ = self.handle_peer_request(&peer_req).await;
                },
                // This future is not cancellation safe as you will lose your space in the queue but as we are the only place
                // that actually requires permits that should be ok.
                Ok(permit) = Arc::clone(&self.outbound_semaphore).acquire_owned() => {
                    if self.handle_free_permit(permit).await.is_err() {
                        // if we got an error then we still have a permit free so to prevent this from just looping
                        // uncontrollably add a timeout.
                        sleep(OUTBOUND_CONNECTION_ATTEMPT_TIMEOUT).await;
                    }
                }
            }
        }
    }
}

struct AnchorConnectionKeeper<Z: NetworkZone, A, C> {
    /// The channel to send new outbound connections down.
    pub new_peers_tx: mpsc::Sender<Client<Z>>,

    /// The address book service
    pub address_book_svc: A,
    /// The service to connect to a specific peer.
    pub connector_svc: C,

    /// A semaphore to keep the amount of outbound peers constant.
    pub anchor_semaphore: Arc<Semaphore>,

    /// The p2p config.
    pub config: P2PConfig<Z>,
}

impl<Z: NetworkZone, A, C> AnchorConnectionKeeper<Z, A, C>
where
    Z: NetworkZone,
    A: AddressBook<Z>,
    C: Service<ConnectRequest<Z>, Response = Client<Z>, Error = HandshakeError> + Send,
    C::Future: Send + 'static,
{
    #[instrument(level = "debug", skip_all)]
    async fn handle_free_anchor_permit(
        &mut self,
        free_anchors: &mut Vec<ZoneSpecificPeerListEntryBase<Z::Addr>>,
        anchor_handshakes: &mut FuturesUnordered<
            BoxFuture<'static, (Z::Addr, Result<(), HandshakeError>)>,
        >,
        permit: OwnedSemaphorePermit,
    ) -> Result<(), OutboundConnectorError> {
        tracing::debug!("Anchor permit available, making outbound connection.");

        if free_anchors.is_empty() {
            let Ok(AddressBookResponse::Peers(peers)) = self
                .address_book_svc
                .ready()
                .await
                .expect("Error in address book!")
                .call(AddressBookRequest::GetAnchorPeers {
                    include_connected: false,
                    len: self.config.numb_anchors,
                })
                .await
            else {
                tracing::warn!("No anchor peers in peer list to make connection to.");
                return Err(OutboundConnectorError::NoAvailablePeers);
            };

            *free_anchors = peers;
        }

        let Some(anchor) = free_anchors.pop() else {
            tracing::warn!("No peers in peer list to promote to anchors.");
            return Err(OutboundConnectorError::NoAvailablePeers);
        };

        let addr = anchor.adr;

        let handle = connect_to_outbound_peer(
            self.new_peers_tx.clone(),
            &mut self.connector_svc,
            permit,
            addr.clone(),
        )
        .await;

        anchor_handshakes.push(handle.map(move |r| (addr, r.unwrap())).boxed());

        Ok(())
    }

    async fn remove_anchor(&mut self, addr: Z::Addr) {
        self.address_book_svc
            .ready()
            .await
            .expect("Error in address book!")
            .call(AddressBookRequest::RemoveAnchorPeer(addr))
            .await
            .expect("Error in address book!");
    }

    async fn run(mut self) {
        let mut free_anchors = Vec::new();
        let mut anchor_handshakes = FuturesUnordered::new();
        let mut fails: HashMap<Z::Addr, u8> = HashMap::new();
        loop {
            tokio::select! {
                Some((addr, res)) = anchor_handshakes.next() => {
                    match res {
                       Ok(()) => {
                            fails.remove(&addr);
                        },
                       Err(_) => {
                            let numb_fails = fails.entry(addr).or_default();
                            *numb_fails += 1;
                            tracing::warn!("Failed to connect to anchor peer: {}, attempt: {}.", addr, numb_fails);
                            if *numb_fails >= 3 {
                                free_anchors.clear();
                                fails.remove(&addr);
                                self.remove_anchor(addr).await;
                            } else {
                                sleep(OUTBOUND_ANCHOR_CONNECTION_ATTEMPT_TIMEOUT).await;
                            }
                        }
                    }
                }
                Ok(permit) = Arc::clone(&self.anchor_semaphore).acquire_owned() => {
                    if self.handle_free_anchor_permit(&mut free_anchors, &mut anchor_handshakes, permit).await.is_err() {
                        // if we got an error then we still have a permit free so to prevent this from just looping
                        // uncontrollably add a timeout.
                        sleep(OUTBOUND_ANCHOR_CONNECTION_ATTEMPT_TIMEOUT).await;
                    }
                }
            }
        }
    }
}

/// Connects to a given outbound peer.
#[instrument(level = "info", skip_all)]
async fn connect_to_outbound_peer<Z: NetworkZone, C>(
    new_peers_tx: mpsc::Sender<Client<Z>>,
    connector_svc: &mut C,
    permit: OwnedSemaphorePermit,
    addr: Z::Addr,
) -> JoinHandle<Result<(), HandshakeError>>
where
    C: Service<ConnectRequest<Z>, Response = Client<Z>, Error = HandshakeError>,
    C::Future: Send + 'static,
{
    let connection_fut = connector_svc
        .ready()
        .await
        .expect("Connector had an error in `poll_ready`")
        .call(ConnectRequest {
            addr,
            permit: Some(permit),
        });

    tokio::spawn(
        async move {
            let peer = timeout(HANDSHAKE_TIMEOUT, connection_fut).await??;
            drop(new_peers_tx.send(peer).await);
            Ok(())
        }
        .instrument(Span::current()),
    )
}
