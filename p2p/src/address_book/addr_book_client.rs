//! This module holds the address books client and [`tower::Service`].
//!
//! To start the address book use [`start_address_book`].
// TODO: Store banned peers persistently.
use std::future::Future;
use std::pin::Pin;
use std::task::Poll;

use futures::channel::{mpsc, oneshot};
use futures::FutureExt;
use tokio::task::{spawn, JoinHandle};
use tower::steer::Steer;
use tower::BoxError;
use tracing::Instrument;

use monero_wire::network_address::NetZone;

use crate::{Config, P2PStore};

use super::address_book::{AddressBook, AddressBookClientRequest};
use super::{AddressBookError, AddressBookRequest, AddressBookResponse};

/// Start the address book.
/// Under the hood this function spawns 3 address books
/// for the 3 [`NetZone`] and combines them into a [`tower::Steer`](Steer).
pub async fn start_address_book<S>(
    peer_store: S,
    config: Config,
) -> Result<
    impl tower::Service<
        AddressBookRequest,
        Response = AddressBookResponse,
        Error = BoxError,
        Future = Pin<
            Box<dyn Future<Output = Result<AddressBookResponse, BoxError>> + Send + 'static>,
        >,
    >,
    BoxError,
>
where
    S: P2PStore,
{
    let mut builder = AddressBookBuilder::new(peer_store, config);

    let public = builder.build(NetZone::Public).await?;
    let tor = builder.build(NetZone::Tor).await?;
    let i2p = builder.build(NetZone::I2p).await?;

    // This list MUST be in the same order as closuer in the `Steer` func
    let books = vec![public, tor, i2p];

    Ok(Steer::new(
        books,
        |req: &AddressBookRequest, _: &[_]| match req.get_zone() {
            // This:
            NetZone::Public => 0,
            NetZone::Tor => 1,
            NetZone::I2p => 2,
        },
    ))
}

/// An address book builder.
/// This:
/// - starts the address book
/// - creates and returns the `AddressBookClient`
struct AddressBookBuilder<S> {
    peer_store: S,
    config: Config,
}

impl<S> AddressBookBuilder<S>
where
    S: P2PStore,
{
    fn new(peer_store: S, config: Config) -> Self {
        AddressBookBuilder { peer_store, config }
    }

    /// Builds the address book for a specific [`NetZone`]
    async fn build(&mut self, zone: NetZone) -> Result<AddressBookClient, AddressBookError> {
        let (white, gray, anchor) = self
            .peer_store
            .load_peers(zone)
            .await
            .map_err(|e| AddressBookError::PeerStoreError(e))?;

        let book = AddressBook::new(
            self.config.clone(),
            zone,
            white,
            gray,
            anchor,
            vec![],
            self.peer_store.clone(),
        );

        let (tx, rx) = mpsc::channel(0);

        let book_span = tracing::info_span!("AddressBook", book = book.book_name());

        let book_handle = spawn(book.run(rx).instrument(book_span));

        Ok(AddressBookClient {
            book: tx,
            book_handle,
        })
    }
}

/// The Client for an individual address book.
#[derive(Debug)]
struct AddressBookClient {
    /// The channel to pass requests to the address book.
    book: mpsc::Sender<AddressBookClientRequest>,
    /// The address book task handle.
    book_handle: JoinHandle<()>,
}

impl tower::Service<AddressBookRequest> for AddressBookClient {
    type Response = AddressBookResponse;
    type Error = BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Check the channel
        match self.book.poll_ready(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Ok(())) => (),
            Poll::Ready(Err(_)) => {
                return Poll::Ready(Err(AddressBookError::AddressBooksChannelClosed.into()))
            }
        }

        // Check the address book task is still running
        match self.book_handle.poll_unpin(cx) {
            // The address book is still running
            Poll::Pending => Poll::Ready(Ok(())),
            // The address book task has exited
            Poll::Ready(_) => Err(AddressBookError::AddressBookTaskExited)?,
        }
    }

    fn call(&mut self, req: AddressBookRequest) -> Self::Future {
        let (tx, rx) = oneshot::channel();
        // get the callers span
        let span = tracing::debug_span!(parent: &tracing::span::Span::current(), "AddressBook");

        let req = AddressBookClientRequest { req, tx, span };

        match self.book.try_send(req) {
            Err(_e) => {
                // I'm assuming all callers will call `poll_ready` first (which they are supposed to)
                futures::future::ready(Err(AddressBookError::AddressBooksChannelClosed.into()))
                    .boxed()
            }
            Ok(()) => async move {
                rx.await
                    .expect("Address Book will not drop requests until completed")
                    .map_err(Into::into)
            }
            .boxed(),
        }
    }
}
