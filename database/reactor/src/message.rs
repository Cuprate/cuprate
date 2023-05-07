//! ### Message module
//! This module contains all the request and response messages that the daemon use to interact with the database

use cuprate_database::error::DBException;
use futures::channel::oneshot;
use monero::Hash;
use tracing::Span;

/// `DatabaseClientRequest` is an inner struct used to share tracing::Span to the worker threads and a oneshot channel to send the response to the caller.
pub(crate) struct DatabaseClientRequest {
	pub req: DatabaseRequest,
	pub tx: oneshot::Sender<Result<DatabaseResponse, DBException>>,
	pub span: Span,
}

/// `DatabaseRequest` is an enum listing all possible request to the database reactor
pub enum DatabaseRequest {
    CurrentHeight,
    CumulativeDifficulty,
    CoreSyncData,
    Chain,
    BlockHeight(Hash),
    BlockKnown(Hash),
}

/// `DatabaseResponse` is an enum listing all the response sent from the reactor, to answer their corresponding Request.
pub enum DatabaseResponse {
    CurrentHeight(u64),
    CumulativeDifficulty(u128),
    //CoreSyncData(CoreSyncData),
    Chain(Vec<Hash>),
    BlockHeight(Option<u64>),
    BlockKnown(BlockKnown),
}

// Temporary
pub enum BlockKnown {
    No,
    OnMainChain,
    OnSideChain,
    KnownBad,
}