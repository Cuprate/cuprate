/// A request for a certain number of outbound peers.
pub struct OutboundPeersRequest(pub usize);

/// A response for [`OutboundPeersRequest`], the amount of peers should be less than or equal
/// to the amount request (not more!).
///
/// You probably will want to wrap these in a drop guard that returns them to the peer set as the dandelion
/// router will just drop them when finished.
pub struct OutboundPeers<S>(pub Vec<S>);

/// A request to diffuse a transaction to all connected peers.
///
/// This crate does not handle diffusion it is left to implementers.
pub struct DiffuseRequest<Tx>(pub Tx);

/// A request sent to a single peer to stem this transaction.
pub struct StemRequest<Tx>(pub Tx);
