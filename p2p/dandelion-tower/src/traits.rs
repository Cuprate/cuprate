/// A request to diffuse a transaction to all connected peers.
///
/// This crate does not handle diffusion it is left to implementers.
pub struct DiffuseRequest<Tx>(pub Tx);

/// A request sent to a single peer to stem this transaction.
pub struct StemRequest<Tx>(pub Tx);

#[cfg(feature = "txpool")]
/// A request sent to the backing transaction pool storage.
pub enum TxStoreRequest<TxId> {
    /// A request to retrieve a `Tx` with the given Id from the pool, should not remove that tx from the pool.
    ///
    /// Must return [`TxStoreResponse::Transaction`]
    Get(TxId),
    /// Promote a transaction from the stem pool to the public pool.
    ///
    /// If the tx is already in the fluff pool do nothing.
    ///
    /// This should not error if the tx isn't in the pool at all.
    Promote(TxId),
}

#[cfg(feature = "txpool")]
/// A response sent back from the backing transaction pool.
pub enum TxStoreResponse<Tx> {
    /// A generic ok response.
    Ok,
    /// A response containing a requested transaction.
    Transaction(Option<(Tx, crate::State)>),
}
