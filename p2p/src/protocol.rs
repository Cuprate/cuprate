/// This module defines InternalRequests and InternalResponses. Cuprate's P2P works by translating network messages into an internal 
/// request/ response, this is easy for levin "requests" and "responses" (admin messages) but takes a bit more work with "notifications" 
/// (protocol messages).
/// 
/// Some notifications are easy to translate, like `GetObjectsRequest` is obviously a request but others like `NewFluffyBlock` are a 
/// bit tricker. To translate a `NewFluffyBlock` into a request/ response we will have to look to see if we asked for `FluffyMissingTransactionsRequest`
/// if we have we interpret `NewFluffyBlock` as a response if not its a request that doesn't require a response.
/// 
/// Here is every P2P request/ response. *note admin messages are already request/ response so "Handshake" is actually made of a HandshakeRequest & HandshakeResponse
/// 
/// Admin:
///     Handshake,
///     TimedSync,
///     Ping,
///     SupportFlags
/// Protocol:
///     Request: GetObjectsRequest,                 Response: GetObjectsResponse,
///     Request: ChainRequest,                      Response: ChainResponse,
///     Request: FluffyMissingTransactionsRequest,  Response: NewFluffyBlock,  <- these 2 could be requests or responses
///     Request: GetTxPoolCompliment,               Response: NewTransactions, <-
///     Request: NewBlock,                          Response: None,
///     Request: NewFluffyBlock,                    Response: None,
///     Request: NewTransactions,                   Response: None
/// 

use monero_wire::messages::{
    AdminMessage, ProtocolMessage, Handshake, TimedSync, Ping, SupportFlags, GetObjectsRequest, GetObjectsResponse,
    ChainRequest, ChainResponse, FluffyMissingTransactionsRequest, NewFluffyBlock, GetTxPoolCompliment,
    NewTransactions, NewBlock, Message, MessageResponse, MessageNotification, MessageRequest
};
 
macro_rules! client_request_peer_response {
    (
    Admin:
        $($admin_mes:ident),+
    Protocol:
        $(Request: $protocol_req:ident, Response: $(SOME: $protocol_res:ident)? $(NULL: $none:expr)?  ),+
    ) => {

        pub enum InternalMessageRequest {
            $($admin_mes(<$admin_mes as AdminMessage>::Request),)+
            $($protocol_req(<$protocol_req as ProtocolMessage>::Notification),)+
        }

        impl InternalMessageRequest {
            pub fn get_str_name(&self) -> &'static str {
                match self {
                    $(InternalMessageRequest::$admin_mes(_) => $admin_mes::NAME,)+
                    $(InternalMessageRequest::$protocol_req(_) => $protocol_req::NAME,)+
                }
            }
            pub fn id(&self) -> u32 {
                match self {
                    $(InternalMessageRequest::$admin_mes(_) => $admin_mes::ID,)+
                    $(InternalMessageRequest::$protocol_req(_) => $protocol_req::ID,)+
                }
            }
            pub fn expected_id(&self) -> Option<u32> {
                match self {
                    $(InternalMessageRequest::$admin_mes(_) => Some($admin_mes::ID),)+
                    $(InternalMessageRequest::$protocol_req(_) => $(Some($protocol_res::ID))? $($none)?,)+
                }
            }
            pub fn is_levin_request(&self) -> bool {
                match self {
                    $(InternalMessageRequest::$admin_mes(_) => true,)+
                    $(InternalMessageRequest::$protocol_req(_) => false,)+
                }
            }
        }

        impl From<MessageRequest> for InternalMessageRequest {
            fn from(value: MessageRequest) -> Self {
                match value {
                    $(MessageRequest::$admin_mes(mes) => InternalMessageRequest::$admin_mes(mes),)+
                }
            }
        }

        #[derive(Debug)]
        pub struct NotAnInternalRequest;

        impl TryFrom<Message> for InternalMessageRequest {
            type Error = NotAnInternalRequest;
            fn try_from(value: Message) -> Result<Self, Self::Error> {
                match value {
                    Message::Response(_) => Err(NotAnInternalRequest),
                    Message::Request(req) => Ok(req.into()),
                    Message::Notification(noti) => {
                        match noti {
                            $(MessageNotification::$protocol_req(noti) => Ok(InternalMessageRequest::$protocol_req(noti)),)+
                            _ => Err(NotAnInternalRequest),
                        }
                    }
                }
            }
        }

        pub enum InternalMessageResponse {
            $($admin_mes(<$admin_mes as AdminMessage>::Response),)+
            $($($protocol_res(<$protocol_res as ProtocolMessage>::Notification),)?)+
        }

        impl InternalMessageResponse {
            pub fn get_str_name(&self) -> &'static str {
                match self {
                    $(InternalMessageResponse::$admin_mes(_) => $admin_mes::NAME,)+
                    $($(InternalMessageResponse::$protocol_res(_) => $protocol_res::NAME,)?)+
                }
            }
            pub fn id(&self) -> u32 {
                match self{
                    $(InternalMessageResponse::$admin_mes(_) => $admin_mes::ID,)+
                    $($(InternalMessageResponse::$protocol_res(_) => $protocol_res::ID,)?)+
                }
            }
        }

        impl From<MessageResponse> for InternalMessageResponse {
            fn from(value: MessageResponse) -> Self {
                match value {
                    $(MessageResponse::$admin_mes(mes) => InternalMessageResponse::$admin_mes(mes),)+
                }
            }
        }
        
        #[derive(Debug)]
        pub struct NotAnInternalResponse;

        impl TryFrom<Message> for InternalMessageResponse {
            type Error = NotAnInternalResponse;
            fn try_from(value: Message) -> Result<Self, Self::Error> {
                match value {
                    Message::Response(res) => Ok(res.into()),
                    Message::Request(_) => Err(NotAnInternalResponse),
                    Message::Notification(noti) => {
                        match noti {
                            $($(MessageNotification::$protocol_res(noti) => Ok(InternalMessageResponse::$protocol_res(noti)),)?)+
                            _ => Err(NotAnInternalResponse),
                        }
                    }
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

