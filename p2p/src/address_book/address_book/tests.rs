use super::*;
use crate::NetZoneBasicNodeData;
use monero_wire::network_address::IPv4Address;
use rand::Rng;

fn create_random_net_address<R: Rng>(r: &mut R) -> NetworkAddress {
    NetworkAddress::IPv4(IPv4Address {
        m_ip: r.gen(),
        m_port: r.gen(),
    })
}

fn create_random_net_addr_vec<R: Rng>(r: &mut R, len: usize) -> Vec<NetworkAddress> {
    let mut ret = Vec::with_capacity(len);
    for i in 0..len {
        ret.push(create_random_net_address(r));
    }
    ret
}

fn create_random_peer<R: Rng>(r: &mut R) -> PeerListEntryBase {
    PeerListEntryBase {
        adr: create_random_net_address(r),
        pruning_seed: r.gen_range(384..=391),
        id: PeerID(r.gen()),
        last_seen: r.gen(),
        rpc_port: r.gen(),
        rpc_credits_per_hash: r.gen(),
    }
}

fn create_random_peer_vec<R: Rng>(r: &mut R, len: usize) -> Vec<PeerListEntryBase> {
    let mut ret = Vec::with_capacity(len);
    for i in 0..len {
        ret.push(create_random_peer(r));
    }
    ret
}

#[derive(Clone)]
pub struct MockPeerStore;

#[async_trait::async_trait]
impl P2PStore for MockPeerStore {
    async fn basic_node_data(&mut self) -> Result<Option<NetZoneBasicNodeData>, &'static str> {
        unimplemented!()
    }
    async fn save_basic_node_data(
        &mut self,
        node_id: &NetZoneBasicNodeData,
    ) -> Result<(), &'static str> {
        unimplemented!()
    }
    async fn load_peers(
        &mut self,
        zone: NetZone,
    ) -> Result<
        (
            Vec<PeerListEntryBase>,
            Vec<PeerListEntryBase>,
            Vec<NetworkAddress>,
        ),
        &'static str,
    > {
        let mut r = rand::thread_rng();
        Ok((
            create_random_peer_vec(&mut r, 300),
            create_random_peer_vec(&mut r, 1500),
            create_random_net_addr_vec(&mut r, 50),
        ))
    }
    async fn save_peers(
        &mut self,
        zone: NetZone,
        white: Vec<&PeerListEntryBase>,
        gray: Vec<&PeerListEntryBase>,
        anchor: Vec<&NetworkAddress>,
    ) -> Result<(), &'static str> {
        todo!()
    }
}
