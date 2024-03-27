use tokio::sync::watch;

use monero_p2p::{AddressBook, NetworkZone, PeerSyncSvc};

use crate::{broadcast::BroadcastSvc, peer_set::PeerSet, peer_sync_state::NewSyncInfo};

pub struct P2PNetwork<N: NetworkZone, AdrBk, PSync> {
    pub peer_set: PeerSet<N>,
    pub peer_sync_svc: PSync,
    pub broadcast_svc: BroadcastSvc<N>,
    pub address_book: AdrBk,

    pub top_sync_data_watch: watch::Receiver<NewSyncInfo>,
}

impl<N: NetworkZone, AdrBk, PSync> P2PNetwork<N, AdrBk, PSync>
where
    AdrBk: AddressBook<N>,
    PSync: PeerSyncSvc<N>,
{
    pub fn new(
        peer_set: PeerSet<N>,
        peer_sync_svc: PSync,
        broadcast_svc: BroadcastSvc<N>,
        address_book: AdrBk,
        top_sync_data_watch: watch::Receiver<NewSyncInfo>,
    ) -> Self {
        Self {
            peer_set,
            peer_sync_svc,
            broadcast_svc,
            address_book,
            top_sync_data_watch,
        }
    }
}
