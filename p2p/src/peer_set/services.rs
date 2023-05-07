use monero_wire::NetworkAddress;
use tower::BoxError;

pub mod block_download;
pub mod block_broadcaster;


pub(crate) type DiscoveredPeer = Result<(NetworkAddress, crate::peer::Client), BoxError>;


pub use block_download::{BlockGetterRequest, BlockGetterResponse, BlockGetterService};