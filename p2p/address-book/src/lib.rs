pub(crate) mod address_book;
mod addr_book_client;

pub use addr_book_client::AddressBookClient;


const MAX_WHITE_LIST_PEERS: usize = 1000;
const MAX_GRAY_LIST_PEERS: usize = 5000;



#[derive(Debug, thiserror::Error)]
pub enum AddressBookError {
    #[error("Peer was not found in book")]
    PeerNotFound,
    #[error("Peer sent an address out of it's net-zone")]
    PeerSentAnAddressOutOfZone,
    #[error("The address books channel has closed.")]
    AddressBooksChannelClosed,
}
