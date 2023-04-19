use cuprate_address_book::{start_address_book, AddressBookStore, AddressBookConfig};
use monero_wire::{NetworkAddress, messages::PeerListEntryBase, network_address::NetZone};
use tower::{Service, ServiceExt};
use tracing::Instrument;

#[derive(Debug, Clone)]
struct PeerStore;

#[async_trait::async_trait]
impl AddressBookStore for PeerStore {
    type Error = cuprate_address_book::AddressBookError;


    async fn load_peers(&mut self, zone: NetZone) 
    -> Result<(
        Vec<PeerListEntryBase>, // white list
        Vec<PeerListEntryBase>, // gray list
        Vec<NetworkAddress>, // anchor list
        Vec<(NetworkAddress, chrono::NaiveDateTime)> // ban list
    ), Self::Error> {
        Ok(
            (vec![],
            vec![],
            vec![],
            vec![])
        )
    }

    async fn save_peers(
        &mut self, 
        zone: NetZone,  
        white: Vec<PeerListEntryBase>, 
        gray: Vec<PeerListEntryBase>, 
        anchor: Vec<NetworkAddress>,
        bans: Vec<(NetworkAddress, chrono::NaiveDateTime)> // ban lists
    ) -> Result<(), Self::Error> {
        todo!()
    }
    
}

#[tracing::instrument]
async fn test() {
    let config = AddressBookConfig::default();
    let mut book = start_address_book(PeerStore, config).await.unwrap().boxed_clone();

    //tracing::info!(parent: &n, "calling address book");

    let r_book = book.ready().await.unwrap();
    let res = r_book.call(cuprate_address_book::AddressBookRequest::GetRandomWhitePeer(NetZone::Public)).await;
    println!("{:?}", res);
}

#[tokio::test]
async fn t_start_address_book() {
	let subscriber = tracing_subscriber::fmt().with_max_level(tracing::Level::TRACE).finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    test().await;
}