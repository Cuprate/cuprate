/// This module defines network Requests and Responses 

use monero_wire::messages::{
    AdminMessage, ProtocolMessage, Handshake, TimedSync, Ping, SupportFlags, GetObjectsRequest, GetObjectsResponse,
    ChainRequest, ChainResponse, FluffyMissingTransactionsRequest, NewFluffyBlock, GetTxPoolCompliment,
    NewTransactions, NewBlock, Message,
};
use levin::{
    connection::{ClientResponseChan, ClientRequest, BucketBuilder, PeerResponse},
    bucket::header::Flags,
};
use levin::bucket::BucketError;
 
macro_rules! client_request_peer_response {
    (
    Admin:
        $($admin_mes:ident),+
    Protocol:
        $(Request: $protocol_req:ident, Response: $(SOME: $protocol_res:ident)? $(NULL: $none:expr)?  ),+
    ) => {

        pub enum MessageRequest {
            $($admin_mes(<$admin_mes as AdminMessage>::Request),)+
            $($protocol_req(<$protocol_req as ProtocolMessage>::Notification),)+
        }

        impl MessageRequest {
            pub fn id(&self) -> u32 {
                match self {
                    $(MessageRequest::$admin_mes(_) => $admin_mes::ID,)+
                    $(MessageRequest::$protocol_req(_) => $protocol_req::ID,)+
                }
            }
            pub fn expected_id(&self) -> Option<u32> {
                match self {
                    $(MessageRequest::$admin_mes(_) => Some($admin_mes::ID),)+
                    $(MessageRequest::$protocol_req(_) => $(Some($protocol_res::ID))? $($none)?,)+
                }
            }
            pub fn encode(&self) -> Option<Vec<u8>> {// change me to result
                match self {
                    $(MessageRequest::$admin_mes(mes) => mes.encode().ok(),)+
                    $(MessageRequest::$protocol_req(mes) => mes.encode().ok(),)+
                }
            }
            pub fn is_levin_request(&self) -> bool {
                match self {
                    $(MessageRequest::$admin_mes(_) => true,)+
                    $(MessageRequest::$protocol_req(_) => false,)+
                }
            }
        }

        pub enum MessageResponse {
            $($admin_mes(<$admin_mes as AdminMessage>::Response),)+
            $($($protocol_res(<$protocol_res as ProtocolMessage>::Notification),)?)+
        }

        impl MessageResponse {
            pub fn id(&self) -> u32 {
                match self{
                    $(MessageResponse::$admin_mes(_) => $admin_mes::ID,)+
                    $($(MessageResponse::$protocol_res(_) => $protocol_res::ID,)?)+
                }
            }
        }

        impl PeerResponse for MessageResponse {
            fn decode(command: u32, body: bytes::Bytes) -> Result<Self, BucketError> {
                match command {
                    $($admin_mes::ID => 
                        Ok(
                            MessageResponse::$admin_mes(
                                <$admin_mes as AdminMessage>::Response::decode(&body)
                                    .map_err(|e| BucketError::FailedToDecodeBucketBody(e.to_string()))?
                                )
                        ),
                    )+
                    $($($protocol_res::ID => 
                        Ok(
                            MessageResponse::$protocol_res(
                                <$protocol_res as ProtocolMessage>::Notification::decode(&body)
                                    .map_err(|e| BucketError::FailedToDecodeBucketBody(e.to_string()))?
                                )
                            ),
                        )?
                    )+
                    _ => Err(BucketError::UnsupportedP2pCommand(command))
                }
            }
        }

    };
}

client_request_peer_response!(
    Admin:
        Handshake,
        TimedSync,
        Ping,
        SupportFlags
    Protocol:
        Request: GetObjectsRequest,                 Response: SOME: GetObjectsResponse,
        Request: ChainRequest,                      Response: SOME: ChainResponse,
        Request: FluffyMissingTransactionsRequest,  Response: SOME: NewFluffyBlock,  // these 2 could be requests or responses
        Request: GetTxPoolCompliment,               Response: SOME: NewTransactions, //
        // these don't need to be responded to
        Request: NewBlock,                          Response: NULL: None,
        Request: NewFluffyBlock,                    Response: NULL: None,
        Request: NewTransactions,                   Response: NULL: None
);

pub struct ClientReq {
    req: MessageRequest,
    tx: Option<ClientResponseChan<MessageResponse>>,
}

impl ClientReq {
    pub fn new(req: MessageRequest, tx: ClientResponseChan<MessageResponse>) -> Self {
        ClientReq{req, tx: Some(tx)}
    }
}

impl Into<BucketBuilder> for ClientReq {
    fn into(self) -> BucketBuilder {
        let mut bucket_builder = BucketBuilder::default();
        bucket_builder.set_return_code(0);
        bucket_builder.set_command(self.command());
        bucket_builder.set_have_to_return(self.is_levin_request());
        bucket_builder.set_flags(Flags::new_request());
        match self.req.encode() {
            Some(val) => bucket_builder.set_body(val.into()),
            None => (), // Levin will now error :)
        }
        bucket_builder
    }
}

impl ClientRequest<MessageResponse> for ClientReq {
    fn command(&self) -> u32 {
        self.req.id()
    }
    fn is_levin_request(&self) -> bool {
        self.req.is_levin_request()
    }
    fn tx(&mut self) -> Option<ClientResponseChan<MessageResponse>> {
        std::mem::replace(&mut self.tx, None)
    }
}
