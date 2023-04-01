use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use cuprate_protocol::{InternalMessageRequest, InternalMessageResponse};
use tower::{Service, ServiceExt};
use futures::StreamExt;
use futures::channel::mpsc;
use monero::Hash;

use monero_wire::{NetworkAddress, Message};
use monero_wire::messages::{CoreSyncData, ChainRequest};
use monero_wire::messages::protocol::ChainResponse;

use cuprate_peer::connection::PeerSyncChange;
use cuprate_common::HardForks;

// TODO: Move this!!!!!!!
// ********************************
pub enum DataBaseRequest {
    CurrentHeight,
    GetBlockHeight(Hash),
    Chain,
}

pub enum DataBaseResponse {
    CurrentHeight(u64),
    GetBlockHeight(u64),
    Chain(Vec<Hash>)
}

#[derive(Debug)]
pub enum DatabaseError {

}

pub enum PeerSetRequest {
    DisconnectPeer(NetworkAddress),
    BanPeer(NetworkAddress),
    SendRequest(InternalMessageRequest, Option<NetworkAddress>)
}

pub struct PeerSetResponse {
    peer: NetworkAddress,
    response: Option<InternalMessageResponse>,

}

// *******************************


pub struct IndividualPeerSync {
    height: u64,
    top_id: Hash,
    top_version: u8,
    cumulative_difficulty: u128,
    /// the height the list of unknown blocks starts at
    start_height: u64,
    /// list of block hashes our node does not have, the first one we will have so we know where this chain starts.
    unknown_block_hashes: Vec<Hash>,
    /// list of weights of the unknown blocks
    block_weights: Vec<u64>,

}

pub struct PeersSyncData {
   peers: HashMap<NetworkAddress, IndividualPeerSync>, 
   // top here means most work done on the chain
   top_peer: NetworkAddress,
   top_max_height: u64,
   top_start_height: u64,
   /// list of block hashes our node does not have, the first one we will have so we know where this chain starts.
   top_unknown_block_hashes: Vec<Hash>,
   /// list of weights of the unknown blocks
   top_block_weights: Vec<u64>,
   top_cumulative_difficulty: u128,
}

impl PeersSyncData {
    pub fn new_core_sync_data(&mut self, id: &NetworkAddress, core_sync: CoreSyncData) -> Result<(), SyncStatesError> {
        let peer_data = self.peers.get_mut(&id);
        if peer_data.is_none() {
            let ips = IndividualPeerSync { height: core_sync.current_height, top_id: core_sync.top_id, top_version: core_sync.top_version, cumulative_difficulty: core_sync.cumulative_difficulty(), start_height: 0, unknown_block_hashes: vec![], block_weights: vec![] };
            self.peers.insert(*id, ips);
        } else{
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

        if self.top_cumulative_difficulty < core_sync.cumulative_difficulty() {
            self.top_cumulative_difficulty = core_sync.cumulative_difficulty();
            self.top_max_height = core_sync.current_height;
            self.top_peer= *id;
        }
        Ok(())
    }
}

pub enum SyncStatesError {
    PeerSentBadTopVersion,
    PeerSentBadPruningSeed,
    PeersHeightHasDropped,
    PeersCumulativeDifficultyDropped,
}

pub enum OkCoreSync {
    Ok,
    RequestChainEntry,
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
    async fn handle_core_sync_change(&mut self, id: &NetworkAddress, core_sync: CoreSyncData) -> Result<OkCoreSync, SyncStatesError> {
        if core_sync.current_height > 0 {
            let version = self.hardforks.get_ideal_version_from_height(core_sync.current_height - 1);
            if version >= 6 && version != core_sync.top_version {
                return Err(SyncStatesError::PeerSentBadTopVersion);
            }
        }
        if core_sync.pruning_seed != 0 {
            let log_stripes = monero::database::pruning::get_pruning_log_stripes(core_sync.pruning_seed);
            let stripe = monero::database::pruning::get_pruning_stripe_for_seed(core_sync.pruning_seed);
            if stripe != monero::database::pruning::CRYPTONOTE_PRUNING_LOG_STRIPES || stripe > (1 << log_stripes) {
                return Err(SyncStatesError::PeerSentBadPruningSeed);
            }
        }
        //if core_sync.current_height > max block numb
        let ready_blockchain = self.blockchain.ready().await.unwrap();
        let DataBaseResponse::CurrentHeight(current_height) = ready_blockchain.call(DataBaseRequest::CurrentHeight).await.unwrap() else {
            unreachable!("the blockchain won't send the wrong response");
        };

        let peers_height = core_sync.current_height.clone();

        let mut sync_states = self.peer_sync_states.lock().unwrap();
        sync_states.new_core_sync_data(id, core_sync)?;

        if current_height < peers_height {
            Ok(OkCoreSync::RequestChainEntry)
        } else {
            Ok(OkCoreSync::Ok)
        }

    }

    async fn handle_chain_entry_response(&mut self, id: NetworkAddress, chain_response: ChainResponse) {

    }

    async fn build_chain_request(&mut self) -> ChainRequest {
        let ready_blockchain = self.blockchain.ready().await.unwrap();
        let DataBaseResponse::Chain(ids) = ready_blockchain.call(DataBaseRequest::Chain).await.unwrap() else {
            unreachable!("the blockchain won't send the wrong response");
        };

        ChainRequest {
            block_ids: ids,
            prune: false
        }
    }

    async fn get_peers_chain_entry<Svc>(&mut self, peer_set: &mut Svc, id: NetworkAddress)
    where
        Svc: Service<PeerSetRequest, Response = PeerSetResponse, Error = DatabaseError>
    {
        let chain_req = self.build_chain_request().await;
        let ready_set = peer_set.ready().await.unwrap();
        let response: PeerSetResponse = ready_set.call(PeerSetRequest::SendRequest(Message::Notification(chain_req.into()).try_into().expect("Chain request can always be converted to IMR"), Some(id))).await.unwrap();
        let InternalMessageResponse::ChainResponse(response) = response.response.expect("peer set will return a result for a request") else {
            unreachable!("peer set will return correct response");
        };

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
                            let ready_set = peer_set.ready().await.unwrap();
                            let res = ready_set.call(PeerSetRequest::BanPeer(id)).await;

                        }
                        Ok(res) => match res {
                            OkCoreSync::Ok => (),
                            OkCoreSync::RequestChainEntry => {
                                self.get_peers_chain_entry(&mut peer_set, id).await
                            }
                        }
                    }
                }
            }
        }
    }
}