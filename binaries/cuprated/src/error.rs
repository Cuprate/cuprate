/// An unrecoverable error in a cuprated subsystem.
#[derive(Debug, thiserror::Error)]
pub enum CupratedError {
    /// The blockchain manager encountered an unrecoverable error.
    #[error("blockchain: {0:#}")]
    Blockchain(anyhow::Error),

    /// The transaction pool subsystem encountered an unrecoverable error.
    #[error("txpool: {0:#}")]
    Txpool(anyhow::Error),

    /// The block syncer encountered an error.
    #[error("syncer: {0:#}")]
    Syncer(anyhow::Error),

    /// The RPC server encountered an error.
    #[error("rpc: {0:#}")]
    Rpc(anyhow::Error),
}
