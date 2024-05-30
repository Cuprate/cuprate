//! # Inbound Server
//!
//! This module contains the inbound connection server, which listens for inbound connections, gives
//! them to the handshaker service and then adds them to the client pool.
use std::{pin::pin, sync::Arc};

use futures::StreamExt;
use tokio::{
    sync::Semaphore,
    time::{sleep, timeout},
};
use tower::{Service, ServiceExt};
use tracing::{instrument, Instrument, Span};

use monero_p2p::{
    client::{Client, DoHandshakeRequest, HandshakeError, InternalPeerID},
    services::{AddressBookRequest, AddressBookResponse},
    AddressBook, ConnectionDirection, NetworkZone,
};

use crate::{
    client_pool::ClientPool,
    constants::{HANDSHAKE_TIMEOUT, INBOUND_CONNECTION_COOL_DOWN},
    P2PConfig,
};

/// The inbound server.
#[instrument(level = "warn", skip_all)]
pub async fn inbound_server<N, HS, A>(
    client_pool: Arc<ClientPool<N>>,
    mut handshaker: HS,
    mut address_book: A,
    config: P2PConfig<N>,
) -> Result<(), tower::BoxError>
where
    N: NetworkZone,
    HS: Service<DoHandshakeRequest<N>, Response = Client<N>, Error = HandshakeError>
        + Send
        + 'static,
    HS::Future: Send + 'static,
    A: AddressBook<N>,
{
    let Some(server_config) = config.server_config else {
        tracing::warn!("No inbound server config provided, not listening for inbound connections.");
        return Ok(());
    };

    tracing::info!("Starting inbound connection server");

    let listener = N::incoming_connection_listener(server_config, config.p2p_port)
        .await
        .inspect_err(|e| tracing::warn!("Failed to start inbound server: {e}"))?;

    let mut listener = pin!(listener);

    let semaphore = Arc::new(Semaphore::new(config.max_inbound_connections));

    while let Some(connection) = listener.next().await {
        let Ok((addr, peer_stream, peer_sink)) = connection else {
            continue;
        };

        if let Some(addr) = &addr {
            let AddressBookResponse::IsPeerBanned(banned) = address_book
                .ready()
                .await?
                .call(AddressBookRequest::IsPeerBanned(*addr))
                .await?
            else {
                panic!("Address book returned incorrect response!");
            };

            if banned {
                continue;
            }
        }

        let addr = match addr {
            Some(addr) => InternalPeerID::KnownAddr(addr),
            None => InternalPeerID::Unknown(rand::random()),
        };

        if let Ok(permit) = semaphore.clone().try_acquire_owned() {
            tracing::debug!("Permit free for incoming connection, attempting handshake.");

            let fut = handshaker.ready().await?.call(DoHandshakeRequest {
                addr,
                peer_stream,
                peer_sink,
                direction: ConnectionDirection::InBound,
                permit,
            });

            let cloned_pool = client_pool.clone();

            tokio::spawn(
                async move {
                    if let Ok(Ok(peer)) = timeout(HANDSHAKE_TIMEOUT, fut).await {
                        cloned_pool.add_new_client(peer);
                    }
                }
                .instrument(Span::current()),
            );
        } else {
            tracing::debug!("No permit free for incoming connection.");
            // TODO: listen for if the peer is just trying to ping us to see if we are reachable.
        }

        sleep(INBOUND_CONNECTION_COOL_DOWN).await;
    }

    Ok(())
}
