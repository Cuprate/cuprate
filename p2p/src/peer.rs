pub mod client;
pub mod connection;
pub mod handshaker;

use monero_wire::levin::BucketError;
use thiserror::Error;

const BLOCKS_IDS_SYNCHRONIZING_DEFAULT_COUNT: usize = 10000;
const BLOCKS_IDS_SYNCHRONIZING_MAX_COUNT: usize = 25000;
const P2P_MAX_PEERS_IN_HANDSHAKE: usize = 250;

pub enum Direction {
    Inbound,
    Outbound,
}

#[derive(Debug, Error, Clone, Copy)]
pub enum PeerResponseError {
    #[error("The peers sent a wrong response to out request")]
    PeerSentWrongResponse,
    #[error("The peers sent a list of peers with length above or max amount")]
    PeerSentTooManyPeers,
    #[error("The peers sent a handshake after connection already established")]
    PeerSentHandshakeAgain,
    #[error("The peers sent an item without enough information")]
    PeerSentItemWithoutEnoughInformation,
    #[error("The peers sent a GetObjectsResponse with a non-requested block")]
    PeerSentBlockWeDidNotRequest,
    #[error("The peer sent a response with a height lower than they told us before")]
    PeersHeightHasDropped,
    #[error("The peer sent a response with a cumulative difficulty lower than they told us before")]
    PeersCumulativeDifficultyHasDropped,
    #[error("The peers pruning seed has changed")]
    PeersPruningSeedHasChanged,
    #[error("The peer sent a response which contains an item with a differing pruning state than requested")]
    PeerSentItemWithAPruningStateWeDoNotWant,
    #[error("The peer sent a block which includes unrelated transactions/ not enough transactions")]
    PeerSentBlockWithIncorrectTransactions,
    #[error("The peer sent a blocks response with a different amount to what was expected")]
    PeerSentIncorrectAmountOfBlocks,
    #[error("The peer sent a chain response with no block ids")]
    PeerSentNoBlockIds,
    #[error("The peer sent a chain response with an invalid start/nblocks/height")]
    PeerSentBadStartOrNBlocksOrheight,
    #[error("The peer sent a response with an invalid block weights")]
    PeerSentInvalidBlockWeights,
    #[error("The peer sent a response which contains too much information")]
    PeerSentTooMuchInformation,
}

#[derive(Debug, Error, Clone, Copy)]
pub enum RequestServiceError {}

#[derive(Debug, Error, Clone, Copy)]
pub enum PeerError {
    #[error("Peer is on a different network")]
    PeerIsOnAnotherNetwork,
    #[error("Peer sent an unexpected response")]
    PeerSentUnSolicitedResponse,
    #[error("Internal service did not respond when required")]
    InternalServiceDidNotRespond,
    #[error("Connection to peer has been terminated")]
    PeerConnectionClosed,
    #[error("The Client `internal` channel was closed")]
    ClientChannelClosed,
    #[error("The Peer sent an unexpected response")]
    PeerSentUnexpectedResponse,
    #[error("The peer sent a bad response: {0}")]
    ResponseError(#[from] PeerResponseError),
    #[error("Internal service error: {0}")]
    InternalService(#[from] RequestServiceError),
    #[error("Levin Error")]
    LevinError, // remove me, this is just temporary
}

impl From<BucketError> for PeerError {
    fn from(_: BucketError) -> Self {
        PeerError::LevinError
    }
}
