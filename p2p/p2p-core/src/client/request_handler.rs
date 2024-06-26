use futures::TryFutureExt;
use tower::ServiceExt;

use cuprate_wire::{
    admin::{
        PingResponse, SupportFlagsResponse, TimedSyncRequest, TimedSyncResponse,
        PING_OK_RESPONSE_STATUS_TEXT,
    },
    AdminRequestMessage, AdminResponseMessage, BasicNodeData,
};

use crate::{
    client::PeerInformation,
    services::{
        AddressBookRequest, AddressBookResponse, CoreSyncDataRequest, CoreSyncDataResponse,
        PeerSyncRequest,
    },
    AddressBook, CoreSyncSvc, NetworkZone, PeerRequest, PeerResponse, PeerSyncSvc,
    ProtocolRequestHandler,
};

#[derive(thiserror::Error, Debug, Copy, Clone, Eq, PartialEq)]
enum PeerRequestHandlerError {
    #[error("Received a handshake request during a connection.")]
    ReceivedHandshakeDuringConnection,
}

#[derive(Debug, Clone)]
pub(crate) struct RequestHandler<Z: NetworkZone, A, CS, PS, PR> {
    pub address_book_svc: A,
    pub our_sync_svc: CS,
    pub peer_sync_svc: PS,

    pub protocol_request_handler: PR,

    pub our_basic_node_data: BasicNodeData,
    pub peer_info: PeerInformation<Z::Addr>,
}

impl<Z: NetworkZone, A, CS, PS, PR> RequestHandler<Z, A, CS, PS, PR>
where
    Z: NetworkZone,
    A: AddressBook<Z>,
    CS: CoreSyncSvc,
    PS: PeerSyncSvc<Z>,
    PR: ProtocolRequestHandler,
{
    pub async fn handle_peer_request(
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

    async fn handle_timed_sync_request(
        &mut self,
        req: TimedSyncRequest,
    ) -> Result<TimedSyncResponse, tower::BoxError> {
        let peer_id = self.peer_info.id;
        let handle = self.peer_info.handle.clone();

        self.peer_sync_svc
            .ready()
            .await?
            .call(PeerSyncRequest::IncomingCoreSyncData(
                peer_id,
                handle,
                req.payload_data,
            ))
            .await?;

        let AddressBookResponse::Peers(peers) = self
            .address_book_svc
            .ready()
            .await?
            .call(AddressBookRequest::GetWhitePeers(20))
            .await?
        else {
            panic!("Address book sent incorrect response!");
        };

        let CoreSyncDataResponse(core_sync_data) = self
            .our_sync_svc
            .ready()
            .await?
            .call(CoreSyncDataRequest)
            .await?;

        Ok(TimedSyncResponse {
            payload_data: core_sync_data,
            local_peerlist_new: peers.into_iter().map(Into::into).collect(),
        })
    }
}
