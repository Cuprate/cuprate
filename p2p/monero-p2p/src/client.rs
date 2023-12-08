mod conector;
mod connection;
pub mod handshaker;

pub use conector::{ConnectRequest, Connector};
pub use handshaker::{DoHandshakeRequest, HandShaker, HandshakeError};

/// An internal identifier for a given peer, will be their address if known
/// or a random u64 if not.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum InternalPeerID<A> {
    KnownAddr(A),
    Unknown(u64),
}
