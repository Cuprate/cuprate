//! # Inbound Server
//!
//! This module contains the inbound connection server, which listens for inbound connections, gives
//! them to the handshaker service and then adds them to the client pool.
use std::{pin::pin, sync::Arc};

use futures::{SinkExt, StreamExt};
use tokio::{
    sync::{mpsc, Semaphore},
    task::JoinSet,
    time::{sleep, timeout},
};
use tower::{Service, ServiceExt};
use tracing::{instrument, Instrument, Span};

use cuprate_p2p_core::{
    client::{Client, DoHandshakeRequest, HandshakeError, InternalPeerID},
    services::{AddressBookRequest, AddressBookResponse},
    AddressBook, ConnectionDirection, NetworkZone, Transport,
};
use cuprate_wire::{
    admin::{PingResponse, PING_OK_RESPONSE_STATUS_TEXT},
    AdminRequestMessage, AdminResponseMessage, Message,
};

use crate::{
    constants::{
        HANDSHAKE_TIMEOUT, INBOUND_CONNECTION_COOL_DOWN, PING_REQUEST_CONCURRENCY,
        PING_REQUEST_TIMEOUT,
    },
    P2PConfig,
};

/// Starts the inbound server. This function will listen to all incoming connections
/// and initiate handshake if needed, after verifying the address isn't banned.
#[instrument(level = "warn", skip_all)]
pub async fn inbound_server<Z, T, HS, A>(
    new_connection_tx: mpsc::Sender<Client<Z>>,
    mut handshaker: HS,
    mut address_book: A,
    config: P2PConfig<Z>,
    transport_config: Option<T::ServerConfig>,
    inbound_semaphore: Arc<Semaphore>,
) -> Result<(), tower::BoxError>
where
    Z: NetworkZone,
    T: Transport<Z>,
    HS: Service<DoHandshakeRequest<Z, T>, Response = Client<Z>, Error = HandshakeError>
        + Send
        + 'static,
    HS::Future: Send + 'static,
    A: AddressBook<Z>,
{
    // Copying the peer_id before borrowing for ping responses (Make us avoid a `clone()`).
    let our_peer_id = config.basic_node_data().peer_id;

    // Mandatory. Extract server config from P2PConfig
    let Some(server_config) = transport_config else {
        tracing::warn!("No inbound server config provided, not listening for inbound connections.");
        return Ok(());
    };

    tracing::info!("Starting inbound connection server");

    let listener = T::incoming_connection_listener(server_config)
        .await
        .inspect_err(|e| tracing::warn!("Failed to start inbound server: {e}"))?;

    let mut listener = pin!(listener);

    // Use the provided semaphore for limiting to maximum inbound connections.
    let semaphore = inbound_semaphore;
    // Create ping request handling JoinSet
    let mut ping_join_set = JoinSet::new();

    // Listen to incoming connections and extract necessary information.
    while let Some(connection) = listener.next().await {
        let Ok((addr, mut peer_stream, mut peer_sink)) = connection else {
            continue;
        };

        // If peer is banned, drop connection
        if let Some(addr) = &addr {
            let AddressBookResponse::GetBan { unban_instant } = address_book
                .ready()
                .await?
                .call(AddressBookRequest::GetBan(*addr))
                .await?
            else {
                panic!("Address book returned incorrect response!");
            };

            if unban_instant.is_some() {
                continue;
            }
        }

        // Create a new internal id for new peers
        let addr = match addr {
            Some(addr) => InternalPeerID::KnownAddr(addr),
            None => InternalPeerID::Unknown(rand::random()),
        };

        // If we're still behind our maximum limit, Initiate handshake.
        if let Ok(permit) = Arc::clone(&semaphore).try_acquire_owned() {
            tracing::debug!("Permit free for incoming connection, attempting handshake.");

            let fut = handshaker.ready().await?.call(DoHandshakeRequest {
                addr,
                peer_stream,
                peer_sink,
                direction: ConnectionDirection::Inbound,
                permit: Some(permit),
            });

            let new_connection_tx = new_connection_tx.clone();

            tokio::spawn(
                async move {
                    let client = timeout(HANDSHAKE_TIMEOUT, fut).await;

                    match client {
                        Ok(Ok(peer)) => drop(new_connection_tx.send(peer).await),
                        Err(_) => tracing::debug!("Timed out"),
                        Ok(Err(e)) => tracing::debug!("error: {e:?}"),
                    }
                }
                .instrument(Span::current()),
            );
        } else {
            // Otherwise check if the node is simply pinging us.
            tracing::debug!("No permit free for incoming connection.");

            // We only handle 2 ping request conccurently. Otherwise we drop the connection immediately.
            if ping_join_set.len() < PING_REQUEST_CONCURRENCY {
                ping_join_set.spawn(
                    async move {
                        // Await first message from node. If it is a ping request we respond back, otherwise we drop the connection.
                        let fut = timeout(PING_REQUEST_TIMEOUT, peer_stream.next());

                        // Ok if timeout did not elapsed -> Some if there is a message -> Ok if it has been decoded
                        if matches!(
                            fut.await,
                            Ok(Some(Ok(Message::Request(AdminRequestMessage::Ping))))
                        ) {
                            let response = peer_sink
                                .send(
                                    Message::Response(AdminResponseMessage::Ping(PingResponse {
                                        status: PING_OK_RESPONSE_STATUS_TEXT,
                                        peer_id: our_peer_id,
                                    }))
                                    .into(),
                                )
                                .await;

                            if let Err(err) = response {
                                tracing::debug!(
                                    "Unable to respond to ping request from peer ({addr}): {err}"
                                );
                            }
                        }
                    }
                    .instrument(Span::current()),
                );
            }
        }

        sleep(INBOUND_CONNECTION_COOL_DOWN).await;
    }

    Ok(())
}
