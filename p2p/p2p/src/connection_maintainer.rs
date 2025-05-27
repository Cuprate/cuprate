//! Outbound Connection Maintainer.
//!
//! This module handles maintaining the number of outbound connections defined in the [`P2PConfig`].
//! It also handles making extra connections when the peer set is under load or when we need data that
//! no connected peer has.
use std::sync::Arc;

use rand::{distributions::Bernoulli, prelude::*};
use tokio::{
    sync::{mpsc, OwnedSemaphorePermit, Semaphore},
    task::JoinSet,
    time::{sleep, timeout},
};
use tower::{Service, ServiceExt};
use tracing::{instrument, Instrument, Span};

use cuprate_p2p_core::{
    client::{Client, ConnectRequest, HandshakeError},
    services::{AddressBookRequest, AddressBookResponse},
    AddressBook, NetworkZone,
};

use crate::{
    config::P2PConfig,
    constants::{HANDSHAKE_TIMEOUT, MAX_SEED_CONNECTIONS, OUTBOUND_CONNECTION_ATTEMPT_TIMEOUT},
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
    /// The pool of currently connected peers.
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
}

impl<Z, A, C> OutboundConnectionKeeper<Z, A, C>
where
    Z: NetworkZone,
    A: AddressBook<Z>,
    C: Service<ConnectRequest<Z>, Response = Client<Z>, Error = HandshakeError>,
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

        Self {
            new_peers_tx,
            make_connection_rx,
            address_book_svc,
            connector_svc,
            outbound_semaphore: Arc::new(Semaphore::new(config.outbound_connections)),
            extra_peers: 0,
            config,
            peer_type_gen,
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

    /// Connects to a given outbound peer.
    #[instrument(level = "info", skip_all)]
    async fn connect_to_outbound_peer(&mut self, permit: OwnedSemaphorePermit, addr: Z::Addr) {
        let new_peers_tx = self.new_peers_tx.clone();
        let connection_fut = self
            .connector_svc
            .ready()
            .await
            .expect("Connector had an error in `poll_ready`")
            .call(ConnectRequest {
                addr,
                permit: Some(permit),
            });

        tokio::spawn(
            async move {
                if let Ok(Ok(peer)) = timeout(HANDSHAKE_TIMEOUT, connection_fut).await {
                    drop(new_peers_tx.send(peer).await);
                }
            }
            .instrument(Span::current()),
        );
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
                self.connect_to_outbound_peer(permit, peer.adr).await;
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

        self.connect_to_outbound_peer(permit, peer.adr).await;
        Ok(())
    }

    /// Runs the outbound connection count keeper.
    pub async fn run(mut self) {
        tracing::info!(
            "Starting outbound connection maintainer, target outbound connections: {}",
            self.config.outbound_connections
        );

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
