use std::fs;

use borsh::{from_slice, to_vec, BorshDeserialize, BorshSerialize};
use tokio::task::{spawn_blocking, JoinHandle};

use monero_p2p::{services::ZoneSpecificPeerListEntryBase, NetZoneAddress, NetworkZone};

use crate::{peer_list::PeerList, AddressBookConfig};

// TODO: store anchor and ban list.

#[derive(BorshSerialize)]
struct SerPeerDataV1<'a, A: NetZoneAddress> {
    white_list: Vec<&'a ZoneSpecificPeerListEntryBase<A>>,
    gray_list: Vec<&'a ZoneSpecificPeerListEntryBase<A>>,
}

#[derive(BorshDeserialize)]
struct DeserPeerDataV1<A: NetZoneAddress> {
    white_list: Vec<ZoneSpecificPeerListEntryBase<A>>,
    gray_list: Vec<ZoneSpecificPeerListEntryBase<A>>,
}

pub fn save_peers_to_disk<Z: NetworkZone>(
    cfg: &AddressBookConfig,
    white_list: &PeerList<Z>,
    gray_list: &PeerList<Z>,
) -> JoinHandle<std::io::Result<()>> {
    // maybe move this to another thread but that would require cloning the data ... this
    // happens so infrequently that it's probably not worth it.
    let data = to_vec(&SerPeerDataV1 {
        white_list: white_list.peers.values().collect::<Vec<_>>(),
        gray_list: gray_list.peers.values().collect::<Vec<_>>(),
    })
    .unwrap();

    let file = cfg.peer_store_file.clone();
    spawn_blocking(move || fs::write(&file, &data))
}

pub async fn read_peers_from_disk<Z: NetworkZone>(
    cfg: &AddressBookConfig,
) -> Result<
    (
        Vec<ZoneSpecificPeerListEntryBase<Z::Addr>>,
        Vec<ZoneSpecificPeerListEntryBase<Z::Addr>>,
    ),
    std::io::Error,
> {
    let file = cfg.peer_store_file.clone();
    let data = spawn_blocking(move || fs::read(file)).await.unwrap()?;

    let de_ser: DeserPeerDataV1<Z::Addr> = from_slice(&data)?;
    Ok((de_ser.white_list, de_ser.gray_list))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer_list::{tests::make_fake_peer_list, PeerList};

    use cuprate_test_utils::test_netzone::{TestNetZone, TestNetZoneAddr};

    #[test]
    fn ser_deser_peer_list() {
        let white_list = make_fake_peer_list(0, 50);
        let gray_list = make_fake_peer_list(50, 100);

        let data = to_vec(&SerPeerDataV1 {
            white_list: white_list.peers.values().collect::<Vec<_>>(),
            gray_list: gray_list.peers.values().collect::<Vec<_>>(),
        })
        .unwrap();

        let de_ser: DeserPeerDataV1<TestNetZoneAddr> = from_slice(&data).unwrap();

        let white_list_2: PeerList<TestNetZone<true, true, true>> =
            PeerList::new(de_ser.white_list);
        let gray_list_2: PeerList<TestNetZone<true, true, true>> = PeerList::new(de_ser.gray_list);

        assert_eq!(white_list.peers.len(), white_list_2.peers.len());
        assert_eq!(gray_list.peers.len(), gray_list_2.peers.len());

        for addr in white_list.peers.keys() {
            assert!(white_list_2.contains_peer(addr));
        }

        for addr in gray_list.peers.keys() {
            assert!(gray_list_2.contains_peer(addr));
        }
    }
}
