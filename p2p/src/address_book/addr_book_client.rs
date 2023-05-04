use std::future::Future;
use std::pin::Pin;

use futures::channel::{mpsc, oneshot};
use futures::FutureExt;
use tokio::task::spawn;
use tower::steer::Steer;

use monero_wire::network_address::NetZone;

use crate::Config;

use super::address_book::{AddressBook, AddressBookClientRequest};
use super::{
    AddressBookError, AddressBookRequest, AddressBookResponse, AddressBookStore,
};

pub async fn start_address_book<S>(
    peer_store: S,
    config: Config,
) -> Result<
    impl tower::Service<
            AddressBookRequest,
            Response = AddressBookResponse,
            Error = AddressBookError,
            Future = Pin<
                Box<
                    dyn Future<Output = Result<AddressBookResponse, AddressBookError>>
                        + Send
                        + 'static,
                >,
            >,
        > + Clone,
    AddressBookError,
>
where
    S: AddressBookStore,
{
    let mut builder = AddressBookBuilder::new(peer_store, config);

    let public = builder.build(NetZone::Public).await?;
    let tor = builder.build(NetZone::Tor).await?;
    let i2p = builder.build(NetZone::I2p).await?;

    let books = vec![public, tor, i2p];

    Ok(Steer::new(
        books,
        |req: &AddressBookRequest, _: &[_]| match req.get_zone() {
            NetZone::Public => 0,
            NetZone::Tor => 1,
            NetZone::I2p => 2,
        },
    ))
}

pub struct AddressBookBuilder<S> {
    peer_store: S,
    config: Config,
}

impl<S> AddressBookBuilder<S>
where
    S: AddressBookStore,
{
    fn new(peer_store: S, config: Config) -> Self {
        AddressBookBuilder { peer_store, config }
    }

    async fn build(&mut self, zone: NetZone) -> Result<AddressBookClient, AddressBookError> {
        let (white, gray, anchor, bans) =
            self.peer_store.load_peers(zone).await.map_err(Into::into)?;

        let book = AddressBook::new(self.config.clone(), zone, white, gray, anchor, bans);

        let (tx, rx) = mpsc::channel(5);

        spawn(book.run(rx));

        Ok(AddressBookClient { book: tx })
    }
}

#[derive(Debug, Clone)]
struct AddressBookClient {
    book: mpsc::Sender<AddressBookClientRequest>,
}

impl tower::Service<AddressBookRequest> for AddressBookClient {
    type Error = AddressBookError;
    type Response = AddressBookResponse;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.book
            .poll_ready(cx)
            .map_err(|_| AddressBookError::AddressBooksChannelClosed)
    }

    fn call(&mut self, req: AddressBookRequest) -> Self::Future {
        let (tx, rx) = oneshot::channel();
        // get the callers span
        let span = tracing::span::Span::current();

        let req = AddressBookClientRequest { req, tx, span };

        match self.book.try_send(req) {
            Err(_e) => {
                // I'm assuming all callers will call `poll_ready` first (which they are supposed to)
                futures::future::ready(Err(AddressBookError::AddressBooksChannelClosed)).boxed()
            }
            Ok(()) => async move {
                rx.await
                    .expect("Address Book will not drop requests until completed")
            }
            .boxed(),
        }
    }
}
