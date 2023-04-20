use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use futures::channel::mpsc;
use futures::StreamExt;
use monero::Hash;
use thiserror::Error;
use tower::{Service, ServiceExt};

use cuprate_common::{hardforks, HardForks};
use cuprate_peer::connection::PeerSyncChange;
use cuprate_protocol::temp_database::{
    BlockKnown, DataBaseRequest, DataBaseResponse, DatabaseError,
};
use cuprate_protocol::{InternalMessageRequest, InternalMessageResponse};
use monero_wire::messages::protocol::ChainResponse;
use monero_wire::messages::{ChainRequest, CoreSyncData};
use monero_wire::{Message, NetworkAddress};

// TODO: Move this!!!!!!!
// ********************************

pub enum PeerSetRequest {
    DisconnectPeer(NetworkAddress),
    BanPeer(NetworkAddress),
    SendRequest(InternalMessageRequest, Option<NetworkAddress>),
}

pub struct PeerSetResponse {
    peer: NetworkAddress,
    response: Option<InternalMessageResponse>,
}

// *******************************
#[derive(Debug, Default)]
pub struct IndividualPeerSync {
    height: u64,
    // no grantee this is the same block as height
    top_id: Hash,
    top_version: u8,
    cumulative_difficulty: u128,
    /// the height the list of needed blocks starts at
    start_height: u64,
    /// list of block hashes our node does not have.
    needed_blocks: Vec<(Hash, Option<u64>)>,
}

#[derive(Debug, Default)]
pub struct PeersSyncData {
    peers: HashMap<NetworkAddress, IndividualPeerSync>,
}

impl PeersSyncData {
    pub fn new_core_sync_data(
        &mut self,
        id: &NetworkAddress,
        core_sync: CoreSyncData,
    ) -> Result<(), SyncStatesError> {
        let peer_data = self.peers.get_mut(&id);
        if peer_data.is_none() {
            let ips = IndividualPeerSync {
                height: core_sync.current_height,
                top_id: core_sync.top_id,
                top_version: core_sync.top_version,
                cumulative_difficulty: core_sync.cumulative_difficulty(),
                start_height: 0,
                needed_blocks: vec![],
            };
            self.peers.insert(*id, ips);
        } else {
            let peer_data = peer_data.unwrap();
            if peer_data.height > core_sync.current_height {
                return Err(SyncStatesError::PeersHeightHasDropped);
            }
            if peer_data.cumulative_difficulty > core_sync.cumulative_difficulty() {
                return Err(SyncStatesError::PeersCumulativeDifficultyDropped);
            }
            peer_data.height = core_sync.current_height;
            peer_data.cumulative_difficulty = core_sync.cumulative_difficulty();
            peer_data.top_id = core_sync.top_id;
            peer_data.top_version = core_sync.top_version;
        }
        Ok(())
    }

    pub fn new_chain_response(
        &mut self,
        id: &NetworkAddress,
        chain_response: ChainResponse,
        needed_blocks: Vec<(Hash, Option<u64>)>,
    ) -> Result<(), SyncStatesError> {
        let peer_data = self
            .peers
            .get_mut(&id)
            .expect("Peers must give use their core sync before chain response");

        // it's sad we have to do this so late in the response validation process
        if peer_data.height > chain_response.total_height {
            return Err(SyncStatesError::PeersHeightHasDropped);
        }
        if peer_data.cumulative_difficulty > chain_response.cumulative_difficulty() {
            return Err(SyncStatesError::PeersCumulativeDifficultyDropped);
        }

        peer_data.cumulative_difficulty = chain_response.cumulative_difficulty();
        peer_data.height = chain_response.total_height;
        peer_data.start_height = chain_response.start_height
            + chain_response.m_block_ids.len() as u64
            - needed_blocks.len() as u64;
        peer_data.needed_blocks = needed_blocks;
        Ok(())
    }
    // returns true if we have ran out of known blocks for that peer
    pub fn new_objects_response(
        &mut self,
        id: &NetworkAddress,
        mut block_ids: HashSet<Hash>,
    ) -> Result<bool, SyncStatesError> {
        let peer_data = self
            .peers
            .get_mut(id)
            .expect("Peers must give use their core sync before objects response");
        let mut i = 0;
        if peer_data.needed_blocks.is_empty() {
            return Ok(true);
        }
        while !block_ids.contains(&peer_data.needed_blocks[i].0) {
            i += 1;
            if i == peer_data.needed_blocks.len() {
                peer_data.needed_blocks = vec![];
                peer_data.start_height = 0;
                return Ok(true);
            }
        }
        for _ in 0..block_ids.len() {
            if !block_ids.remove(&peer_data.needed_blocks[i].0) {
                return Err(SyncStatesError::PeerSentAnUnexpectedBlockId);
            }
            i += 1;
            if i == peer_data.needed_blocks.len() {
                peer_data.needed_blocks = vec![];
                peer_data.start_height = 0;
                return Ok(true);
            }
        }
        peer_data.needed_blocks = peer_data.needed_blocks[i..].to_vec();
        peer_data.start_height = peer_data.start_height + i as u64;
        return Ok(false);
    }

    pub fn peer_disconnected(&mut self, id: &NetworkAddress) {
        let _ = self.peers.remove(id);
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SyncStatesError {
    #[error("Peer sent a block id we know is bad")]
    PeerSentKnownBadBlock,
    #[error("Peer sent a block id we weren't expecting")]
    PeerSentAnUnexpectedBlockId,
    #[error("Peer sent a chain entry where we don't know the start")]
    PeerSentNoneOverlappingFirstBlock,
    #[error("We have the peers block just at a different height")]
    WeHaveBlockAtDifferentHeight,
    #[error("The peer sent a top version we weren't expecting")]
    PeerSentBadTopVersion,
    #[error("The peer sent a weird pruning seed")]
    PeerSentBadPruningSeed,
    #[error("The peer height has dropped")]
    PeersHeightHasDropped,
    #[error("The peers cumulative difficulty has dropped")]
    PeersCumulativeDifficultyDropped,
    #[error("Our database returned an error: {0}")]
    DataBaseError(#[from] DatabaseError),
}

pub struct SyncStates<Db> {
    peer_sync_rx: mpsc::Receiver<PeerSyncChange>,
    hardforks: HardForks,
    peer_sync_states: Arc<Mutex<PeersSyncData>>,
    blockchain: Db,
}

impl<Db> SyncStates<Db>
where
    Db: Service<DataBaseRequest, Response = DataBaseResponse, Error = DatabaseError>,
{
    pub fn new(
        peer_sync_rx: mpsc::Receiver<PeerSyncChange>,
        hardforks: HardForks,
        peer_sync_states: Arc<Mutex<PeersSyncData>>,
        blockchain: Db,
    ) -> Self {
        SyncStates {
            peer_sync_rx,
            hardforks,
            peer_sync_states,
            blockchain,
        }
    }
    async fn send_database_request(
        &mut self,
        req: DataBaseRequest,
    ) -> Result<DataBaseResponse, DatabaseError> {
        let ready_blockchain = self.blockchain.ready().await?;
        ready_blockchain.call(req).await
    }

    async fn handle_core_sync_change(
        &mut self,
        id: &NetworkAddress,
        core_sync: CoreSyncData,
    ) -> Result<bool, SyncStatesError> {
        if core_sync.current_height > 0 {
            let version = self
                .hardforks
                .get_ideal_version_from_height(core_sync.current_height - 1);
            if version >= 6 && version != core_sync.top_version {
                return Err(SyncStatesError::PeerSentBadTopVersion);
            }
        }
        if core_sync.pruning_seed != 0 {
            let log_stripes =
                monero::database::pruning::get_pruning_log_stripes(core_sync.pruning_seed);
            let stripe =
                monero::database::pruning::get_pruning_stripe_for_seed(core_sync.pruning_seed);
            if stripe != monero::database::pruning::CRYPTONOTE_PRUNING_LOG_STRIPES
                || stripe > (1 << log_stripes)
            {
                return Err(SyncStatesError::PeerSentBadPruningSeed);
            }
        }
        //if core_sync.current_height > max block numb
        let DataBaseResponse::BlockHeight(height) = self.send_database_request(DataBaseRequest::BlockHeight(core_sync.top_id)).await? else {
            unreachable!("the blockchain won't send the wrong response");
        };

        let behind: bool;

        if let Some(height) = height {
            if height != core_sync.current_height {
                return Err(SyncStatesError::WeHaveBlockAtDifferentHeight);
            }
            behind = false;
        } else {
            let DataBaseResponse::CumulativeDifficulty(cumulative_diff) = self.send_database_request(DataBaseRequest::CumulativeDifficulty).await? else {
                unreachable!("the blockchain won't send the wrong response");
            };
            // if their chain has more POW we want it
            if cumulative_diff < core_sync.cumulative_difficulty() {
                behind = true;
            } else {
                behind = false;
            }
        }

        let mut sync_states = self.peer_sync_states.lock().unwrap();
        sync_states.new_core_sync_data(id, core_sync)?;

        Ok(behind)
    }

    async fn handle_chain_entry_response(
        &mut self,
        id: &NetworkAddress,
        chain_response: ChainResponse,
    ) -> Result<(), SyncStatesError> {
        let mut expect_unknown = false;
        let mut needed_blocks = Vec::with_capacity(chain_response.m_block_ids.len());

        for (index, block_id) in chain_response.m_block_ids.iter().enumerate() {
            let DataBaseResponse::BlockKnown(known) = self.send_database_request(DataBaseRequest::BlockKnown(*block_id)).await? else {
                unreachable!("the blockchain won't send the wrong response");
            };
            if index == 0 {
                if !known.is_known() {
                    return Err(SyncStatesError::PeerSentNoneOverlappingFirstBlock);
                }
            } else {
                match known {
                    BlockKnown::No => expect_unknown = true,
                    BlockKnown::OnMainChain => {
                        if expect_unknown {
                            return Err(SyncStatesError::PeerSentAnUnexpectedBlockId);
                        } else {
                            let DataBaseResponse::BlockHeight(height) = self.send_database_request(DataBaseRequest::BlockHeight(*block_id)).await? else {
                                unreachable!("the blockchain won't send the wrong response");
                            };
                            if chain_response.start_height + index as u64
                                != height.expect("We already know this block is in our main chain.")
                            {
                                return Err(SyncStatesError::WeHaveBlockAtDifferentHeight);
                            }
                        }
                    }
                    BlockKnown::OnSideChain => {
                        if expect_unknown {
                            return Err(SyncStatesError::PeerSentAnUnexpectedBlockId);
                        }
                    }
                    BlockKnown::KnownBad => return Err(SyncStatesError::PeerSentKnownBadBlock),
                }
            }
            let block_weight = chain_response.m_block_weights.get(index).map(|f| f.clone());
            needed_blocks.push((*block_id, block_weight));
        }
        let mut sync_states = self.peer_sync_states.lock().unwrap();
        sync_states.new_chain_response(id, chain_response, needed_blocks)?;
        Ok(())
    }

    async fn build_chain_request(&mut self) -> Result<ChainRequest, DatabaseError> {
        let DataBaseResponse::Chain(ids) = self.send_database_request(DataBaseRequest::Chain).await? else {
            unreachable!("the blockchain won't send the wrong response");
        };

        Ok(ChainRequest {
            block_ids: ids,
            prune: false,
        })
    }

    async fn get_peers_chain_entry<Svc>(
        &mut self,
        peer_set: &mut Svc,
        id: &NetworkAddress,
    ) -> Result<ChainResponse, DatabaseError>
    where
        Svc: Service<PeerSetRequest, Response = PeerSetResponse, Error = DatabaseError>,
    {
        let chain_req = self.build_chain_request().await?;
        let ready_set = peer_set.ready().await.unwrap();
        let response: PeerSetResponse = ready_set
            .call(PeerSetRequest::SendRequest(
                Message::Notification(chain_req.into())
                    .try_into()
                    .expect("Chain request can always be converted to IMR"),
                Some(*id),
            ))
            .await?;
        let InternalMessageResponse::ChainResponse(response) = response.response.expect("peer set will return a result for a chain request") else {
            unreachable!("peer set will return correct response");
        };

        Ok(response)
    }

    async fn get_and_handle_chain_entry<Svc>(
        &mut self,
        peer_set: &mut Svc,
        id: NetworkAddress,
    ) -> Result<(), SyncStatesError>
    where
        Svc: Service<PeerSetRequest, Response = PeerSetResponse, Error = DatabaseError>,
    {
        let chain_response = self.get_peers_chain_entry(peer_set, &id).await?;
        self.handle_chain_entry_response(&id, chain_response).await
    }

    async fn handle_objects_response(
        &mut self,
        id: NetworkAddress,
        block_ids: Vec<Hash>,
        peers_height: u64,
    ) -> Result<bool, SyncStatesError> {
        let mut sync_states = self.peer_sync_states.lock().unwrap();
        let ran_out_of_blocks =
            sync_states.new_objects_response(&id, HashSet::from_iter(block_ids))?;
        drop(sync_states);
        if ran_out_of_blocks {
            let DataBaseResponse::CurrentHeight(our_height) = self.send_database_request(DataBaseRequest::CurrentHeight).await? else {
                unreachable!("the blockchain won't send the wrong response");
            };
            if our_height < peers_height {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn handle_peer_disconnect(&mut self, id: NetworkAddress) {
        let mut sync_states = self.peer_sync_states.lock().unwrap();
        sync_states.peer_disconnected(&id);
    }

    pub async fn run<Svc>(mut self, mut peer_set: Svc)
    where
        Svc: Service<PeerSetRequest, Response = PeerSetResponse, Error = DatabaseError>,
    {
        loop {
            let Some(change) = self.peer_sync_rx.next().await else {
                // is this best?
                return;
            };

            match change {
                PeerSyncChange::CoreSyncData(id, csd) => {
                    match self.handle_core_sync_change(&id, csd).await {
                        Err(_) => {
                            // TODO: check if error needs ban or forget
                            let ready_set = peer_set.ready().await.unwrap();
                            let res = ready_set.call(PeerSetRequest::BanPeer(id)).await;
                        }
                        Ok(request_chain) => {
                            if request_chain {
                                self.get_and_handle_chain_entry(&mut peer_set, id).await;
                            }
                        }
                    }
                }
                PeerSyncChange::ObjectsResponse(id, block_ids, height) => {
                    match self.handle_objects_response(id, block_ids, height).await {
                        Err(_) => {
                            // TODO: check if error needs ban or forget
                            let ready_set = peer_set.ready().await.unwrap();
                            let res = ready_set.call(PeerSetRequest::BanPeer(id)).await;
                        }
                        Ok(res) => {
                            if res {
                                self.get_and_handle_chain_entry(&mut peer_set, id).await;
                            }
                        }
                    }
                }
                PeerSyncChange::PeerDisconnected(id) => {
                    self.handle_peer_disconnect(id);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use monero::Hash;
    use monero_wire::messages::{ChainResponse, CoreSyncData};

    use crate::{PeersSyncData, SyncStatesError};

    #[test]
    fn peer_sync_data_good_core_sync() {
        let mut peer_sync_states = PeersSyncData::default();
        let core_sync = CoreSyncData::new(65346753, 1232, 389, Hash::null(), 1);

        peer_sync_states
            .new_core_sync_data(&monero_wire::NetworkAddress::default(), core_sync)
            .unwrap();

        let new_core_sync = CoreSyncData::new(65346754, 1233, 389, Hash::null(), 1);

        peer_sync_states
            .new_core_sync_data(&monero_wire::NetworkAddress::default(), new_core_sync)
            .unwrap();

        let peer = peer_sync_states
            .peers
            .get(&monero_wire::NetworkAddress::default())
            .unwrap();
        assert_eq!(peer.height, 1233);
        assert_eq!(peer.cumulative_difficulty, 65346754);
    }

    #[test]
    fn peer_sync_data_peer_height_dropped() {
        let mut peer_sync_states = PeersSyncData::default();
        let core_sync = CoreSyncData::new(65346753, 1232, 389, Hash::null(), 1);

        peer_sync_states
            .new_core_sync_data(&monero_wire::NetworkAddress::default(), core_sync)
            .unwrap();

        let new_core_sync = CoreSyncData::new(65346754, 1231, 389, Hash::null(), 1);

        let res = peer_sync_states
            .new_core_sync_data(&monero_wire::NetworkAddress::default(), new_core_sync)
            .unwrap_err();

        assert_eq!(res, SyncStatesError::PeersHeightHasDropped);
    }

    #[test]
    fn peer_sync_data_peer_cumulative_difficulty_dropped() {
        let mut peer_sync_states = PeersSyncData::default();
        let core_sync = CoreSyncData::new(65346753, 1232, 389, Hash::null(), 1);

        peer_sync_states
            .new_core_sync_data(&monero_wire::NetworkAddress::default(), core_sync)
            .unwrap();

        let new_core_sync = CoreSyncData::new(65346752, 1233, 389, Hash::null(), 1);

        let res = peer_sync_states
            .new_core_sync_data(&monero_wire::NetworkAddress::default(), new_core_sync)
            .unwrap_err();

        assert_eq!(res, SyncStatesError::PeersCumulativeDifficultyDropped);
    }

    #[test]
    fn peer_sync_new_chain_response() {
        let mut peer_sync_states = PeersSyncData::default();
        let core_sync = CoreSyncData::new(65346753, 1232, 389, Hash::null(), 1);

        peer_sync_states
            .new_core_sync_data(&monero_wire::NetworkAddress::default(), core_sync)
            .unwrap();

        let chain_response = ChainResponse::new(
            10,
            1233,
            65346754,
            vec![Hash::new(&[1]), Hash::new(&[2])],
            vec![],
            vec![],
        );

        let needed_blocks = vec![(Hash::new(&[2]), None)];

        peer_sync_states
            .new_chain_response(
                &monero_wire::NetworkAddress::default(),
                chain_response,
                needed_blocks,
            )
            .unwrap();

        let peer = peer_sync_states
            .peers
            .get(&monero_wire::NetworkAddress::default())
            .unwrap();

        assert_eq!(peer.start_height, 11);
        assert_eq!(peer.height, 1233);
        assert_eq!(peer.cumulative_difficulty, 65346754);
        assert_eq!(peer.needed_blocks, vec![(Hash::new(&[2]), None)]);
    }
}
