use bytes::Bytes;
use dandelion_tower::TxState;
use futures::Stream;

/// A trait representing the whole P2P network, including all network zones.
///
/// [`cuprate_p2p`] provides a per [`NetworkZone`](monero_p2p::NetworkZone) abstraction in [`TODO`], this trait
/// provides a full abstraction, just exposing a minimal interface for Cuprate to interact with.
///
/// It's methods will handle routing to the different [`NetworkZone`](monero_p2p::NetworkZone)s when required.
///
/// This is kept generic for testing purposes.
trait P2PNetwork: Clone {
    /// An identifier for a node on any [`NetworkZone`](monero_p2p::NetworkZone)
    type PeerID;
    /// The block downloader stream.
    type BlockDownloader: Stream<Item = ()>;

    /// Broadcasts a block to the network.
    fn broadcast_block(&mut self, block_bytes: Bytes, chain_height: u64);

    /// Broadcasts a transaction to the network.
    fn broadcast_transaction(&mut self, tx_bytes: Bytes, state: TxState<Self::PeerID>);

    fn block_downloader(&mut self) -> Self::BlockDownloader;
}
