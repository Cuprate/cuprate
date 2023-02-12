pub mod state_machine;


// this is all test code and will be removed
use std::{net, io::{Read, Write}, time::Duration};

use cuprate_net::messages::PeerID;
use state_machine::peer_list_mgr::PeerStoreConfig;
use cuprate_net::levin::{InternalStateMachine, Direction};

struct Store;

impl state_machine::peer_list_mgr::PeerStore for Store {
    fn read_white_peers(&self) -> Vec<cuprate_net::messages::PeerListEntryBase> {
        vec![cuprate_net::messages::PeerListEntryBase {
            adr: cuprate_net::NetworkAddress::IPv4(cuprate_net::network_address::IPv4Address{m_ip: 1432545, m_port: 0}),
            id: cuprate_net::messages::common::PeerID(0),
            last_seen: 0,
            pruning_seed: 0,
            rpc_port: 0,
            rpc_credits_per_hash: 0,
        }]
    }
    fn read_anchor_peers(&self) -> Vec<cuprate_net::messages::PeerListEntryBase> {
        vec![]
    }
    fn read_grey_peers(&self) -> Vec<cuprate_net::messages::PeerListEntryBase> {
        vec![]
    }
    fn write_anchor_peers(&mut self, peers: Vec<cuprate_net::messages::PeerListEntryBase>) {
        
    }
    fn write_white_peers(&mut self, peers: Vec<cuprate_net::messages::PeerListEntryBase>) {
        
    }
    fn write_grey_peers(&mut self, peers: Vec<cuprate_net::messages::PeerListEntryBase>) {
        
    }
    fn flush(&mut self) {
        
    }
}

#[test]
fn test() {
    let listener = net::TcpListener::bind("127.0.0.1:18088").unwrap();

    let info = state_machine::NodeInfo { 
        my_port: 18088, 
        network: state_machine::Network::MainNet, 
        peer_id: PeerID(765566565656757657), 
        support_flags: 1, 
        rpc_port: 0, 
        rpc_credits_per_hash: 0 };

    let rng = fastrand::Rng::new();

    let store = Store;

    let peer_store_cfg = PeerStoreConfig::default();

    let mut sm= state_machine::StateMachine::new(info, rng, store, peer_store_cfg);

    let mut protocol = cuprate_net::levin::protocol_machine::Levin::new(sm);

    let mut stream = listener.incoming().next().unwrap().unwrap();
    let addr = stream.peer_addr().unwrap();
    protocol.connected(addr.into(), Direction::Inbound);
    loop {
        std::thread::sleep(Duration::from_secs(2));
        println!("new conn: {}", addr);
        let mut buf = vec![0;1024];
        let len = stream.read(&mut buf).unwrap();
        buf = buf[0..len].to_vec();
        if !buf.is_empty() {
            println!("recived message");
            protocol.received_bytes(&addr.into(), &buf);
        }

        let output = protocol.next();
        if output.is_some() {
            println!("{:?}", output);
            match output.unwrap() {
                cuprate_net::levin::protocol_machine::Output::Write(_, bytes) => {
                    println!("{}, {:?}", bytes.len(), bytes);
                    stream.write_all(&bytes).unwrap()},
                _ => todo!("reactor I/O")
            };
        }


    }

}
