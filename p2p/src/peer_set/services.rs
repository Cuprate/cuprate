use monero_wire::NetworkAddress;
use tower::BoxError;

pub mod block_broadcaster;
pub mod block_download;

pub(crate) type DiscoveredPeer = Result<(NetworkAddress, crate::peer::Client), BoxError>;

pub use block_download::{BlockGetterRequest, BlockGetterResponse, BlockGetterService};
