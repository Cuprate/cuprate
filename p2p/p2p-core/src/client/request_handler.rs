use futures::TryFutureExt;
use rand::{thread_rng, Rng};
use tower::ServiceExt;

use cuprate_pruning::PruningSeed;
use cuprate_wire::{
    admin::{
        PingResponse, SupportFlagsResponse, TimedSyncRequest, TimedSyncResponse,
        PING_OK_RESPONSE_STATUS_TEXT,
    },
    AdminRequestMessage, AdminResponseMessage, BasicNodeData,
};

use crate::{
    client::{PeerInformation, PeerSyncCallback},
    constants::MAX_PEERS_IN_PEER_LIST_MESSAGE,
    services::{
        AddressBookRequest, AddressBookResponse, CoreSyncDataRequest, CoreSyncDataResponse,
        ZoneSpecificPeerListEntryBase,
    },
    AddressBook, CoreSyncSvc, NetworkZone, PeerRequest, PeerResponse, ProtocolRequestHandler,
};

#[derive(thiserror::Error, Debug, Copy, Clone, Eq, PartialEq)]
enum PeerRequestHandlerError {
    #[error("Received a handshake request during a connection.")]
    ReceivedHandshakeDuringConnection,
}

/// The peer request handler, handles incoming [`PeerRequest`]s to our node.
#[derive(Debug, Clone)]
pub(crate) struct PeerRequestHandler<Z: NetworkZone, A, CS, PR> {
    /// The address book service.
    pub address_book_svc: A,
    /// Our core sync service.
    pub our_sync_svc: CS,

    /// The handler for [`ProtocolRequest`](crate::ProtocolRequest)s to our node.
    pub protocol_request_handler: PR,

    /// The basic node data of our node.
    pub our_basic_node_data: BasicNodeData,

    /// The information on the connected peer.
    pub peer_info: PeerInformation<Z::Addr>,

    /// Called with the peer's cumulative difficulty.
    pub on_peer_sync: Option<PeerSyncCallback>,
}

impl<Z, A, CS, PR> PeerRequestHandler<Z, A, CS, PR>
where
    Z: NetworkZone,
    A: AddressBook<Z>,
    CS: CoreSyncSvc,
    PR: ProtocolRequestHandler,
{
    /// Handles an incoming [`PeerRequest`] to our node.
    pub(crate) async fn handle_peer_request(
        &mut self,
        req: PeerRequest,
    ) -> Result<PeerResponse, tower::BoxError> {
        match req {
            PeerRequest::Admin(admin_req) => match admin_req {
                AdminRequestMessage::Handshake(_) => {
                    Err(PeerRequestHandlerError::ReceivedHandshakeDuringConnection.into())
                }
                AdminRequestMessage::SupportFlags => {
                    let support_flags = self.our_basic_node_data.support_flags;

                    Ok(PeerResponse::Admin(AdminResponseMessage::SupportFlags(
                        SupportFlagsResponse { support_flags },
                    )))
                }
                AdminRequestMessage::Ping => Ok(PeerResponse::Admin(AdminResponseMessage::Ping(
                    PingResponse {
                        peer_id: self.our_basic_node_data.peer_id,
                        status: PING_OK_RESPONSE_STATUS_TEXT,
                    },
                ))),
                AdminRequestMessage::TimedSync(timed_sync_req) => {
                    let res = self.handle_timed_sync_request(timed_sync_req).await?;

                    Ok(PeerResponse::Admin(AdminResponseMessage::TimedSync(res)))
                }
            },

            PeerRequest::Protocol(protocol_req) => {
                // TODO: add limits here

                self.protocol_request_handler
                    .ready()
                    .await?
                    .call(protocol_req)
                    .map_ok(PeerResponse::Protocol)
                    .await
            }
        }
    }

    /// Handles a [`TimedSyncRequest`] to our node.
    async fn handle_timed_sync_request(
        &mut self,
        req: TimedSyncRequest,
    ) -> Result<TimedSyncResponse, tower::BoxError> {
        // TODO: add a limit on the amount of these requests in a certain time period.

        let new_cd = req.payload_data.cumulative_difficulty();
        *self.peer_info.core_sync_data.lock().unwrap() = req.payload_data;

        if let Some(on_peer_sync) = &self.on_peer_sync {
            on_peer_sync.call(new_cd);
        }

        // Fetch core sync data.
        let CoreSyncDataResponse(core_sync_data) = self
            .our_sync_svc
            .ready()
            .await?
            .call(CoreSyncDataRequest)
            .await?;

        // Attempt to fetch our own address if supported by this network zone.
        let own_addr = if Z::BROADCAST_OWN_ADDR {
            let AddressBookResponse::OwnAddress(own_addr) = self
                .address_book_svc
                .ready()
                .await?
                .call(AddressBookRequest::OwnAddress)
                .await?
            else {
                panic!("Address book sent incorrect response!");
            };

            own_addr
        } else {
            None
        };

        let mut peer_list_req_size = MAX_PEERS_IN_PEER_LIST_MESSAGE;
        if own_addr.is_some() {
            peer_list_req_size -= 1;
        }

        // Fetch a peerlist to send
        let AddressBookResponse::Peers(mut peers) = self
            .address_book_svc
            .ready()
            .await?
            .call(AddressBookRequest::GetWhitePeers(peer_list_req_size))
            .await?
        else {
            panic!("Address book sent incorrect response!");
        };

        if let Some(own_addr) = own_addr {
            // Append our address to the final peer list
            peers.insert(
                thread_rng().gen_range(0..=peers.len()),
                ZoneSpecificPeerListEntryBase {
                    adr: own_addr,
                    id: self.our_basic_node_data.peer_id,
                    last_seen: 0,
                    pruning_seed: PruningSeed::NotPruned,
                    rpc_port: self.our_basic_node_data.rpc_port,
                    rpc_credits_per_hash: self.our_basic_node_data.rpc_credits_per_hash,
                },
            );
        }

        Ok(TimedSyncResponse {
            payload_data: core_sync_data,
            local_peerlist_new: peers.into_iter().map(Into::into).collect(),
        })
    }
}
