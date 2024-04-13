use std::sync::Arc;
use tokio::sync::watch;

use monero_p2p::{AddressBook, NetworkZone, PeerSyncSvc};

use crate::{broadcast::BroadcastSvc, peer_set::ClientPool, peer_sync_state::NewSyncInfo};

pub struct P2PNetwork<N: NetworkZone, AdrBk, PSync> {
    pub client_pool: Arc<ClientPool<N>>,
    pub peer_sync_svc: PSync,
    pub broadcast_svc: BroadcastSvc<N>,
    pub address_book: AdrBk,

    pub top_sync_data_watch: watch::Receiver<NewSyncInfo>,
}

impl<N: NetworkZone, AdrBk, PSync> P2PNetwork<N, AdrBk, PSync>
where
    AdrBk: AddressBook<N>,
    PSync: PeerSyncSvc<N> + Clone,
{
    pub fn new(
        client_pool: Arc<ClientPool<N>>,
        peer_sync_svc: PSync,
        broadcast_svc: BroadcastSvc<N>,
        address_book: AdrBk,
        top_sync_data_watch: watch::Receiver<NewSyncInfo>,
    ) -> Self {
        Self {
            client_pool,
            peer_sync_svc,
            broadcast_svc,
            address_book,
            top_sync_data_watch,
        }
    }
}
