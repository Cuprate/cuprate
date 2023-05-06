use cuprate_database::error::DBException;
use futures::channel::oneshot;
use monero::Hash;
use tracing::Span;

pub(crate) struct DatabaseClientRequest {
	pub req: DatabaseRequest,
	pub tx: oneshot::Sender<Result<DatabaseResponse, DBException>>,
	pub span: Span,
}

pub enum DatabaseRequest {
    CurrentHeight,
    CumulativeDifficulty,
    CoreSyncData,
    Chain,
    BlockHeight(Hash),
    BlockKnown(Hash),
	Shutdown,
}

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