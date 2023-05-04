use tower::buffer::Buffer;
use tower::util::BoxService;
use tower::{Service, BoxError};

use crate::constants::ADDRESS_BOOK_BUFFER_SIZE;
use crate::{Config, P2PStore};
use crate::address_book::{start_address_book, AddressBookRequest, AddressBookResponse};
use crate::protocol::{InternalMessageRequest, InternalMessageResponse};

pub async fn init<Svc, P2PS>(config: Config, inbound_service: Svc, p2p_store: P2PS) 
-> Result<
    Buffer<BoxService<AddressBookRequest, AddressBookResponse, BoxError>, AddressBookRequest>, BoxError>
where
    Svc: Service<InternalMessageRequest, Response = InternalMessageResponse, Error = BoxError>
        + Clone
        + Send
        + 'static,
    Svc::Future: Send,
    P2PS: P2PStore
{
    let book =  Buffer::new(BoxService::new(start_address_book(p2p_store, config).await?), ADDRESS_BOOK_BUFFER_SIZE);
    
    Ok(book)

}
