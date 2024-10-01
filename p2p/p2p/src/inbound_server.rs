//! # Inbound Server
//!
//! This module contains the inbound connection server, which listens for inbound connections, gives
//! them to the handshaker service and then adds them to the client pool.
use std::{pin::pin, sync::Arc};

use futures::{SinkExt, StreamExt};
use tokio::{
    sync::Semaphore,
    task::JoinSet,
    time::{sleep, timeout},
};
use tower::{Service, ServiceExt};
use tracing::{instrument, Instrument, Span};

use cuprate_p2p_core::{
    client::{Client, DoHandshakeRequest, HandshakeError, InternalPeerID},
    services::{AddressBookRequest, AddressBookResponse},
    AddressBook, ConnectionDirection, NetworkZone,
};
use cuprate_wire::{
    admin::{PingResponse, PING_OK_RESPONSE_STATUS_TEXT},
    AdminRequestMessage, AdminResponseMessage, Message,
};

use crate::{
    client_pool::ClientPool,
    constants::{
        HANDSHAKE_TIMEOUT, INBOUND_CONNECTION_COOL_DOWN, PING_REQUEST_CONCURRENCY,
        PING_REQUEST_TIMEOUT,
    },
    P2PConfig,
};

/// Starts the inbound server. This function will listen to all incoming connections
/// and initiate handshake if needed, after verifying the address isn't banned.
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
    // Copying the peer_id before borrowing for ping responses (Make us avoid a `clone()`).
    let our_peer_id = config.basic_node_data().peer_id;

    // Mandatory. Extract server config from P2PConfig
    let Some(server_config) = config.server_config else {
        tracing::warn!("No inbound server config provided, not listening for inbound connections.");
        return Ok(());
    };

    tracing::info!("Starting inbound connection server");

    let listener = N::incoming_connection_listener(server_config, config.p2p_port)
        .await
        .inspect_err(|e| tracing::warn!("Failed to start inbound server: {e}"))?;

    let mut listener = pin!(listener);

    // Create semaphore for limiting to maximum inbound connections.
    let semaphore = Arc::new(Semaphore::new(config.max_inbound_connections));
    // Create ping request handling JoinSet
    let mut ping_join_set = JoinSet::new();

    // Listen to incoming connections and extract necessary information.
    while let Some(connection) = listener.next().await {
        let Ok((addr, mut peer_stream, mut peer_sink)) = connection else {
            continue;
        };

        // If peer is banned, drop connection
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

            let cloned_pool = Arc::clone(&client_pool);

            tokio::spawn(
                async move {
                    let client = timeout(HANDSHAKE_TIMEOUT, fut).await;
                    if let Ok(Ok(peer)) = client {
                        cloned_pool.add_new_client(peer);
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
