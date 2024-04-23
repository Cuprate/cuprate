/// A request to diffuse a transaction to all connected peers.
///
/// This crate does not handle diffusion it is left to implementers.
pub struct DiffuseRequest<Tx>(pub Tx);

/// A request sent to a single peer to stem this transaction.
pub struct StemRequest<Tx>(pub Tx);

#[cfg(feature = "txpool")]
/// A request sent to the backing transaction pool storage.
pub enum TxStoreRequest<Tx, TxID> {
    /// A request to store a transaction with the ID to store it under and the pool to store it in.
    ///
    /// If the tx is already in the pool then do nothing, unless the tx is in the stem pool then move it
    /// to the fluff pool _if this request state is fluff_.
    ///
    /// The ID is user defined and is provided to the [`DandelionPool`](crate::txpool::DandelionPool) in
    /// the request.
    Store(Tx, TxID, crate::State),
    /// A request to retrieve a [`Tx`] with the given ID from the pool, should not remove that tx from the pool.
    ///
    /// Must return [`TxStoreResponse::Transaction`]
    Get(TxID),
    /// Promote a transaction from the stem pool to the public pool.
    ///
    /// If the tx is already in the fluff pool do nothing.
    ///
    /// This should not error if the tx isn't in the pool at all.
    Promote(TxID),
    /// A request to check if a translation is in the pool.
    ///
    /// Must return [`TxStoreResponse::Contains`]
    Contains(TxID),
}

#[cfg(feature = "txpool")]
/// A response sent back from the backing transaction pool.
pub enum TxStoreResponse<Tx> {
    /// A generic ok response.
    Ok,
    /// A response containing a [`Option`] for if the transaction is in the pool (Some) or not (None) and in which pool
    /// the tx is in.
    Contains(Option<crate::State>),
    /// A response containing a requested transaction.
    Transaction(Option<(Tx, crate::State)>),
}
