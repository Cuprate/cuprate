use tokio::sync::watch;

use monero_p2p::{AddressBook, NetworkZone};

use crate::{broadcast::BroadcastSvc, peer_set::PeerSet, peer_sync_state::NewSyncInfo};

pub struct P2PNetwork<N: NetworkZone, AdrBk> {
    peer_set: PeerSet<N>,
    broadcast_svc: BroadcastSvc<N>,
    address_book: AdrBk,

    pub top_sync_data_watch: watch::Receiver<NewSyncInfo>,
}

impl<N: NetworkZone, AdrBk> P2PNetwork<N, AdrBk>
where
    AdrBk: AddressBook<N>,
{
    pub fn new(
        peer_set: PeerSet<N>,
        broadcast_svc: BroadcastSvc<N>,
        address_book: AdrBk,
        top_sync_data_watch: watch::Receiver<NewSyncInfo>,
    ) -> Self {
        Self {
            peer_set,
            broadcast_svc,
            address_book,
            top_sync_data_watch,
        }
    }
}
