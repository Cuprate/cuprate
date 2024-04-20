/// A request to diffuse a transaction to all connected peers.
///
/// This crate does not handle diffusion it is left to implementers.
pub struct DiffuseRequest<Tx>(pub Tx);

/// A request sent to a single peer to stem this transaction.
pub struct StemRequest<Tx>(pub Tx);
