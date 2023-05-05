use futures::TryStreamExt;
use futures::{future, StreamExt};
use tower::buffer::Buffer;
use tower::discover::Change;
use tower::util::BoxService;
use tower::{BoxError, Layer, Service};

use monero_wire::NetworkAddress;

use crate::address_book::{start_address_book, AddressBookRequest, AddressBookResponse};
use crate::constants;
use crate::protocol::{
    CoreSyncDataRequest, CoreSyncDataResponse, InternalMessageRequest, InternalMessageResponse,
};
use crate::{peer, Config, NetZoneBasicNodeData, P2PStore};

use super::set::{MorePeers, PeerSet};

type DiscoveredPeer = Result<(NetworkAddress, peer::Client), BoxError>;

pub async fn init<Svc, CoreSync, P2PS>(
    config: Config,
    inbound_service: Svc,
    core_sync_svc: CoreSync,
    mut p2p_store: P2PS,
) -> Result<
    Buffer<BoxService<AddressBookRequest, AddressBookResponse, BoxError>, AddressBookRequest>,
    BoxError,
>
where
    Svc: Service<InternalMessageRequest, Response = InternalMessageResponse, Error = BoxError>
        + Clone
        + Send
        + 'static,
    Svc::Future: Send,

    CoreSync: Service<CoreSyncDataRequest, Response = CoreSyncDataResponse, Error = BoxError>
        + Clone
        + Send
        + 'static,
    CoreSync::Future: Send,

    P2PS: P2PStore,
{
    let basic_node_data: NetZoneBasicNodeData = match p2p_store.basic_node_data().await? {
        Some(bnd) => bnd,
        None => {
            let node_id = crate::NodeID::generate();
            let bnd = NetZoneBasicNodeData::new(&config, &node_id);
            p2p_store.save_basic_node_data(&bnd).await?;
            bnd
        }
    };
    let address_book = Buffer::new(
        BoxService::new(start_address_book(p2p_store, config).await?),
        constants::ADDRESS_BOOK_BUFFER_SIZE,
    );

    let outbound_connector = {
        use tower::timeout::TimeoutLayer;
        let hs_timeout = TimeoutLayer::new(constants::HANDSHAKE_TIMEOUT);
        let hs = peer::Handshaker::new(
            basic_node_data,
            config.network(),
            address_book.clone(),
            core_sync_svc,
            inbound_service,
        );
        hs_timeout.layer(hs)
    };

    let (peerset_tx, peerset_rx) =
        futures::channel::mpsc::channel::<DiscoveredPeer>(config.peerset_total_connection_limit());

    let discovered_peers = peerset_rx
        // Discover interprets an error as stream termination,
        // so discard any errored connections...
        .filter(|result| future::ready(result.is_ok()))
        .map_ok(|(address, client)| Change::Insert(address, client.into()));

    // Create an mpsc channel for peerset demand signaling,
    // based on the maximum number of outbound peers.
    let (mut demand_tx, demand_rx) =
        futures::channel::mpsc::channel::<MorePeers>(config.peerset_total_connection_limit());

    // Create a oneshot to send background task JoinHandles to the peer set
    let (handle_tx, handle_rx) = tokio::sync::oneshot::channel();

    let peer_set = PeerSet::new(&config, discovered_peers, demand_tx, handle_rx);
    let peer_set = Buffer::new(BoxService::new(peer_set), constants::PEERSET_BUFFER_SIZE);

    Ok(address_book)
}
