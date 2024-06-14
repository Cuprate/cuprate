use std::sync::Arc;

use tokio::sync::Semaphore;
use tower::{Service, ServiceExt};

use cuprate_helper::network::Network;
use cuprate_wire::{common::PeerSupportFlags, protocol::GetObjectsRequest, BasicNodeData};

use cuprate_p2p_core::{
    client::{ConnectRequest, Connector, HandShaker},
    network_zones::ClearNet,
    protocol::{PeerRequest, PeerResponse},
};

use cuprate_test_utils::monerod::monerod;

mod utils;
use utils::*;

#[tokio::test]
async fn get_single_block_from_monerod() {
    let semaphore = Arc::new(Semaphore::new(10));
    let permit = semaphore.acquire_owned().await.unwrap();

    let monerod = monerod(["--out-peers=0"]).await;

    let our_basic_node_data = BasicNodeData {
        my_port: 0,
        network_id: Network::Mainnet.network_id(),
        peer_id: 87980,
        support_flags: PeerSupportFlags::FLUFFY_BLOCKS,
        rpc_port: 0,
        rpc_credits_per_hash: 0,
    };

    let handshaker = HandShaker::<ClearNet, _, _, _, _, _>::new(
        DummyAddressBook,
        DummyPeerSyncSvc,
        DummyCoreSyncSvc,
        DummyPeerRequestHandlerSvc,
        |_| futures::stream::pending(),
        our_basic_node_data,
    );

    let mut connector = Connector::new(handshaker);

    let mut connected_peer = connector
        .ready()
        .await
        .unwrap()
        .call(ConnectRequest {
            addr: monerod.p2p_addr(),
            permit,
        })
        .await
        .unwrap();

    let PeerResponse::GetObjects(obj) = connected_peer
        .ready()
        .await
        .unwrap()
        .call(PeerRequest::GetObjects(GetObjectsRequest {
            blocks: hex::decode("418015bb9ae982a1975da7d79277c2705727a56894ba0fb246adaabb1f4632e3")
                .unwrap()
                .try_into()
                .unwrap(),
            pruned: false,
        }))
        .await
        .unwrap()
    else {
        panic!("Client returned wrong response");
    };

    assert_eq!(obj.blocks.len(), 1);
    assert_eq!(obj.missed_ids.len(), 0);
    assert_eq!(obj.current_blockchain_height, 1);
}
