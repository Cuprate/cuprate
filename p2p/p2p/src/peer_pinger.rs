use crate::TransportConfig;
use cuprate_p2p_core::client::{Client, ConnectRequest, HandshakeError};
use cuprate_p2p_core::services::{
    AddressBookRequest, AddressBookResponse, ZoneSpecificPeerListEntryBase,
};
use cuprate_p2p_core::{AddressBook, NetworkZone, Transport};
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tower::{Service, ServiceExt};
use tracing::instrument;

const PEER_PING_INTERVAL: Duration = Duration::from_secs(60);

const BATCH_PING_COUNT: usize = 3;

pub(crate) struct PeerPinger<Z: NetworkZone, T: Transport<Z>, A> {
    /// The address book service
    pub address_book_svc: A,
    pub transport_config: Arc<TransportConfig<Z, T>>,
}

impl<Z, T, A> PeerPinger<Z, T, A>
where
    Z: NetworkZone,
    T: Transport<Z>,
    A: AddressBook<Z> + Clone,
{
    #[instrument(level = "debug", skip_all)]
    async fn ping_random_peers(&mut self) {
        tracing::debug!("pinning random peers in address book.");
        for _ in 0..(BATCH_PING_COUNT - 2) {
            let Ok(AddressBookResponse::Peer(peer)) = self
                .address_book_svc
                .ready()
                .await
                .expect("Error in address book!")
                .call(AddressBookRequest::TakeRandomWhitePeer { height: None })
                .await
            else {
                tracing::debug!("No peers to ping");
                break;
            };
            
            tokio::spawn(do_ping(peer, self.address_book_svc.clone(), self.transport_config.clone()));
        }

        for _ in 0..(BATCH_PING_COUNT + 2) {
            let Ok(AddressBookResponse::Peer(peer)) = self
                .address_book_svc
                .ready()
                .await
                .expect("Error in address book!")
                .call(AddressBookRequest::TakeRandomGrayPeer { height: None })
                .await
            else {
                tracing::debug!("No peers to ping");
                break;
            };

            tokio::spawn(do_ping(peer, self.address_book_svc.clone(), self.transport_config.clone()));
        }
    }

    pub(crate) async fn run(mut self) {
        let mut interval = tokio::time::interval(PEER_PING_INTERVAL);

        loop {
            interval.tick().await;
            self.ping_random_peers().await;
        }
    }
}

#[instrument(level = "debug", skip_all, fields(peer = %peer.adr))]
async fn do_ping<Z, T, A>(
    peer: ZoneSpecificPeerListEntryBase<Z::Addr>,
    mut address_book_svc: A,
    transport_config: Arc<TransportConfig<Z, T>>,
) where
    Z: NetworkZone,
    T: Transport<Z>,
    A: AddressBook<Z>,

{
    const PING_TIMEOUT: Duration = Duration::from_secs(10);

    let Ok(Ok(_)) = timeout(
        PING_TIMEOUT,
        cuprate_p2p_core::client::handshaker::ping::<Z, T>(peer.adr, &transport_config.client_config),
    )
    .await
    else {
        return;
    };

    address_book_svc
        .ready()
        .await
        .expect("Error in address book!")
        .call(AddressBookRequest::PeerReachable(peer))
        .await;
}
