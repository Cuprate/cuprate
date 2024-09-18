use tower::{Service, ServiceExt};

use cuprate_helper::network::Network;
use cuprate_test_utils::monerod::monerod;
use cuprate_wire::{common::PeerSupportFlags, protocol::GetObjectsRequest, BasicNodeData};

use crate::{
    client::{handshaker::HandshakerBuilder, ConnectRequest, Connector},
    protocol::{PeerRequest, PeerResponse},
    ClearNet, ProtocolRequest, ProtocolResponse,
};

#[tokio::test]
async fn get_single_block_from_monerod() {
    let monerod = monerod(["--out-peers=0"]).await;

    let our_basic_node_data = BasicNodeData {
        my_port: 0,
        network_id: Network::Mainnet.network_id(),
        peer_id: 87980,
        support_flags: PeerSupportFlags::FLUFFY_BLOCKS,
        rpc_port: 0,
        rpc_credits_per_hash: 0,
    };

    let handshaker = HandshakerBuilder::<ClearNet>::new(our_basic_node_data).build();

    let mut connector = Connector::new(handshaker);

    let mut connected_peer = connector
        .ready()
        .await
        .unwrap()
        .call(ConnectRequest {
            addr: monerod.p2p_addr(),
            permit: None,
        })
        .await
        .unwrap();

    let PeerResponse::Protocol(ProtocolResponse::GetObjects(obj)) = connected_peer
        .ready()
        .await
        .unwrap()
        .call(PeerRequest::Protocol(ProtocolRequest::GetObjects(
            GetObjectsRequest {
                blocks: hex::decode(
                    "418015bb9ae982a1975da7d79277c2705727a56894ba0fb246adaabb1f4632e3",
                )
                .unwrap()
                .try_into()
                .unwrap(),
                pruned: false,
            },
        )))
        .await
        .unwrap()
    else {
        panic!("Client returned wrong response");
    };

    assert_eq!(obj.blocks.len(), 1);
    assert_eq!(obj.missed_ids.len(), 0);
    assert_eq!(obj.current_blockchain_height, 1);
}
