//! Outbound Connection Maker

use std::{sync::Arc, time::Duration};

use tokio::{
    sync::{mpsc, OwnedSemaphorePermit, Semaphore},
    time::timeout,
};
use tower::{Service, ServiceExt};
use tracing::instrument;

use monero_p2p::{
    client::{Client, ConnectRequest, HandshakeError},
    services::{AddressBookRequest, AddressBookResponse},
    AddressBook, NetworkZone,
};

use crate::config::P2PConfig;

const CONNECTION_TIMEOUT: Duration = Duration::from_secs(120);

/// A request from the peer set to make an outbound connection.
///
/// This will only be sent when the peer set is under load from the rest of Cuprate or the peer
/// set needs specific data that none of the currently connected peers have.
pub struct MakeConnectionRequest {
    /// The block needed that no connected peers have due to pruning.
    block_needed: Option<u64>,
}

pub struct OutboundConnectionMaker<N: NetworkZone, A, C> {
    /// The channel to send new connections down.
    new_connection_tx: mpsc::Sender<Client<N>>,
    /// The channel that tells us to make new outbound connections
    make_connection_rx: mpsc::Receiver<MakeConnectionRequest>,
    /// The address book service
    address_book_svc: A,
    /// The service to connect to a specific peer.
    connector_svc: C,
    /// A semaphore to keep the amount of outbound peers constant.
    outbound_semaphore: Arc<Semaphore>,
    /// The amount of peers we connected to because we needed more peers. If the `outbound_semaphore`
    /// is full, and we need to connect to more peers for blocks ro becuase not enough peers are ready
    /// we add a permit to the semaphore and keep track here, upto a value in config.
    extra_peers: usize,

    config: P2PConfig,
}

impl<N: NetworkZone, A, C> OutboundConnectionMaker<N, A, C>
where
    A: AddressBook<N>,
    C: Service<ConnectRequest<N>, Response = Client<N>, Error = HandshakeError>,
    C::Future: Send + 'static,
{
    #[instrument(level = "debug", skip(self, permit), fields(%addr))]
    async fn connect_to_outbound_peer(
        &mut self,
        permit: OwnedSemaphorePermit,
        addr: N::Addr,
    ) -> Result<(), HandshakeError> {
        let new_connection_tx = self.new_connection_tx.clone();
        let connection_fut = self
            .connector_svc
            .ready()
            .await?
            .call(ConnectRequest { addr, permit });

        tokio::spawn(async move {
            if let Ok(Ok(peer)) = timeout(CONNECTION_TIMEOUT, connection_fut).await {
                let _ = new_connection_tx.send(peer).await;
            }
        });

        Ok(())
    }

    async fn handle_peer_request(&mut self, req: MakeConnectionRequest) {
        let peer = self
            .address_book_svc
            .ready()
            .await
            .expect("Error in address book!")
            .call(AddressBookRequest::GetRandomPeer {
                height: req.block_needed,
            })
            .await;
    }

    async fn run(self) {
        tokio::select! {
            biased;
            Some(peer_req) = self.make_connection_rx.next() =>

        }
    }
}
