use futures::{FutureExt, StreamExt};
use indexmap::IndexMap;
use monero_serai::{
    block::{Block, BlockHeader},
    ringct::{RctBase, RctPrunable, RctSignatures},
    transaction::{Input, Timelock, Transaction, TransactionPrefix},
};
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use crate::block_downloader::{
    download_blocks, BlockDownloaderConfig, ChainSvcRequest, ChainSvcResponse,
};
use crate::client_pool::ClientPool;
use fixed_bytes::ByteArrayVec;
use monero_p2p::client::{mock_client, Client, InternalPeerID, PeerInformation};
use monero_p2p::network_zones::ClearNet;
use monero_p2p::services::{PeerSyncRequest, PeerSyncResponse};
use monero_p2p::{ConnectionDirection, NetworkZone, PeerRequest, PeerResponse};
use monero_pruning::PruningSeed;
use monero_wire::common::{BlockCompleteEntry, TransactionBlobs};
use monero_wire::protocol::{ChainResponse, GetObjectsResponse};
use proptest::{collection::vec, prelude::*};
use tokio::sync::Semaphore;
use tower::{service_fn, Service};

prop_compose! {
    fn dummy_transaction_stragtegy(height: u64)
        (
            extra in vec(any::<u8>(), 0..1_000),
            timelock in any::<usize>(),
        )
    -> Transaction {
        Transaction {
            prefix: TransactionPrefix {
                version: 1,
                timelock: Timelock::Block(timelock),
                inputs: vec![Input::Gen(height)],
                outputs: vec![],
                extra,
            },
            signatures: vec![],
            rct_signatures: RctSignatures {
                base: RctBase {
                    fee: 0,
                    pseudo_outs: vec![],
                    encrypted_amounts: vec![],
                    commitments: vec![],
                },
                prunable: RctPrunable::Null
            },
        }
    }
}

prop_compose! {
    fn dummy_block_stragtegy(
            height: u64,
            previous: [u8; 32],
        )
        (
            miner_tx in dummy_transaction_stragtegy(height),
            txs in vec(dummy_transaction_stragtegy(height), 0..25)
        )
    -> (Block, Vec<Transaction>) {
       (
           Block {
                header: BlockHeader {
                    major_version: 0,
                    minor_version: 0,
                    timestamp: 0,
                    previous,
                    nonce: 0,
                },
                miner_tx,
                txs: txs.iter().map(Transaction::hash).collect(),
           },
           txs
       )
    }
}

struct MockBlockchain {
    blocks: IndexMap<[u8; 32], (Block, Vec<Transaction>)>,
}

impl Debug for MockBlockchain {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("MockBlockchain")
    }
}

prop_compose! {
    fn dummy_blockchain_stragtegy()(
        blocks in vec(dummy_block_stragtegy(0, [0; 32]), 1..50_000),
    ) -> MockBlockchain {
        let mut blockchain = IndexMap::new();

        for (height, mut block) in  blocks.into_iter().enumerate() {
            if let Some(last) = blockchain.last() {
                block.0.header.previous = *last.0;
                block.0.miner_tx.prefix.inputs = vec![Input::Gen(height as u64)]
            }

            blockchain.insert(block.0.hash(), block);
        }

        MockBlockchain {
            blocks: blockchain
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 4,
        max_shrink_iters: 10,
        timeout: 600 * 1_000,
        .. ProptestConfig::default()
    })]

    #[test]
    fn test_block_downloader(blockchain in dummy_blockchain_stragtegy(), peers in 1_usize..128) {
        let blockchain = Arc::new(blockchain);

        let tokio_pool = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();

        tokio_pool.block_on(async move {
            let client_pool = ClientPool::new();

            let mut peer_ids = Vec::with_capacity(peers);

            for _ in 0..peers {
                let client = mock_block_downloader_client(blockchain.clone());

                peer_ids.push(client.info.id);

                client_pool.add_new_client(client);
            }

            let stream = download_blocks(
                client_pool,
                SyncStateSvc(peer_ids) ,
                OurChainSvc {
                    genesis: *blockchain.blocks.first().unwrap().0
                },
                BlockDownloaderConfig {
                    buffer_size: 100_000,
                    in_progress_queue_size: 100_000,
                    check_client_pool_interval: Duration::from_secs(5),
                    target_batch_size: 50_000,
                    initial_batch_size: 1,
            });

            let blocks = stream.map(|blocks| blocks.blocks).concat().await;

            assert_eq!(blocks.len() + 1, blockchain.blocks.len());
        });
    }
}

fn mock_block_downloader_client(blockchain: Arc<MockBlockchain>) -> Client<ClearNet> {
    let semaphore = Arc::new(Semaphore::new(1));

    let (connection_guard, connection_handle) = monero_p2p::handles::HandleBuilder::new()
        .with_permit(semaphore.try_acquire_owned().unwrap())
        .build();

    let request_handler = service_fn(move |req: PeerRequest| {
        let bc = blockchain.clone();

        async move {
            match req {
                PeerRequest::GetChain(chain_req) => {
                    let mut i = 0;
                    while !bc.blocks.contains_key(&chain_req.block_ids[i]) {
                        i += 1;

                        if i == chain_req.block_ids.len() {
                            i -= 1;
                            break;
                        }
                    }

                    let block_index = bc.blocks.get_index_of(&chain_req.block_ids[i]).unwrap();

                    let block_ids = bc
                        .blocks
                        .get_range(block_index..)
                        .unwrap()
                        .iter()
                        .map(|(id, _)| *id)
                        .take(200)
                        .collect::<Vec<_>>();

                    Ok(PeerResponse::GetChain(ChainResponse {
                        start_height: 0,
                        total_height: 0,
                        cumulative_difficulty_low64: 1,
                        cumulative_difficulty_top64: 0,
                        m_block_ids: block_ids.into(),
                        m_block_weights: vec![],
                        first_block: Default::default(),
                    }))
                }

                PeerRequest::GetObjects(obj) => {
                    let mut res = Vec::with_capacity(obj.blocks.len());

                    for i in 0..obj.blocks.len() {
                        let block = bc.blocks.get(&obj.blocks[i]).unwrap();

                        let block_entry = BlockCompleteEntry {
                            pruned: false,
                            block: block.0.serialize().into(),
                            txs: TransactionBlobs::Normal(
                                block
                                    .1
                                    .iter()
                                    .map(Transaction::serialize)
                                    .map(Into::into)
                                    .collect(),
                            ),
                            block_weight: 0,
                        };

                        res.push(block_entry);
                    }

                    Ok(PeerResponse::GetObjects(GetObjectsResponse {
                        blocks: res,
                        missed_ids: ByteArrayVec::from([]),
                        current_blockchain_height: 0,
                    }))
                }
                _ => panic!(),
            }
        }
        .boxed()
    });

    let info = PeerInformation {
        id: InternalPeerID::Unknown(rand::random()),
        handle: connection_handle,
        direction: ConnectionDirection::InBound,
        pruning_seed: PruningSeed::NotPruned,
    };

    mock_client(info, connection_guard, request_handler)
}

#[derive(Clone)]
struct SyncStateSvc<Z: NetworkZone>(Vec<InternalPeerID<Z::Addr>>);

impl Service<PeerSyncRequest<ClearNet>> for SyncStateSvc<ClearNet> {
    type Response = PeerSyncResponse<ClearNet>;
    type Error = tower::BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: PeerSyncRequest<ClearNet>) -> Self::Future {
        let peers = self.0.clone();

        async move { Ok(PeerSyncResponse::PeersToSyncFrom(peers)) }.boxed()
    }
}

struct OurChainSvc {
    genesis: [u8; 32],
}

impl Service<ChainSvcRequest> for OurChainSvc {
    type Response = ChainSvcResponse;
    type Error = tower::BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: ChainSvcRequest) -> Self::Future {
        let genesis = self.genesis;

        async move {
            Ok(match req {
                ChainSvcRequest::CompactHistory => ChainSvcResponse::CompactHistory {
                    block_ids: vec![genesis],
                    cumulative_difficulty: 1,
                },
                ChainSvcRequest::FindFirstUnknown(_) => ChainSvcResponse::FindFirstUnknown(1, 1),
                ChainSvcRequest::CumulativeDifficulty => ChainSvcResponse::CumulativeDifficulty(1),
            })
        }
        .boxed()
    }
}
