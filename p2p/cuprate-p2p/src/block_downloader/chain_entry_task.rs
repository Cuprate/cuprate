use crate::peer_set::ClientPoolGuard;
use monero_p2p::client::InternalPeerID;
use monero_p2p::handles::ConnectionHandle;
use monero_p2p::{NetworkZone, PeerRequest, PeerResponse};
use monero_wire::protocol::{ChainRequest, ChainResponse};
use tower::{Service, ServiceExt};
use tracing::instrument;

pub struct ChainEntryTaskOk<N: NetworkZone> {
    pub client: ClientPoolGuard<N>,
    pub chain_entry: ChainResponse,
}

#[derive(Debug, thiserror::Error)]
pub enum ChainEntryTaskErr {
    #[error("{0}")]
    PeerClientError(#[from] tower::BoxError),
}

#[instrument(level="info", skip_all, fields(%client.info.id))]
pub async fn get_next_chain_entry<N: NetworkZone>(
    mut client: ClientPoolGuard<N>,
    history: Vec<[u8; 32]>,
) -> Result<ChainEntryTaskOk<N>, ChainEntryTaskErr> {
    tracing::debug!("Requesting next chain entry.");

    let PeerResponse::GetChain(chain) = client
        .ready()
        .await?
        .call(PeerRequest::GetChain(ChainRequest {
            block_ids: history.into(),
            prune: false,
        }))
        .await?
    else {
        panic!("Connection task returned wrong response for request.");
    };

    tracing::warn!(
        "Got next chain entry, amount of blocks: {}",
        chain.m_block_ids.len()
    );

    Ok(ChainEntryTaskOk {
        chain_entry: chain,
        client,
    })
}
