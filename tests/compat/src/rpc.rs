use crossbeam::channel::Sender;
use monero_serai::{block::Block, transaction::Transaction};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, ClientBuilder,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    constants::RANDOMX_START_HEIGHT,
    types::{GetBlockResponse, JsonRpcResponse, RpcBlockData, RpcTxData},
};

#[derive(Debug, Clone)]
pub struct RpcClient {
    client: Client,
    json_rpc_url: String,
    get_transactions_url: String,
    pub top_height: u64,
}

impl RpcClient {
    pub async fn new(rpc_url: String) -> Self {
        let headers = {
            let mut h = HeaderMap::new();
            h.insert("Content-Type", HeaderValue::from_static("application/json"));
            h
        };

        let client = ClientBuilder::new()
            .default_headers(headers)
            .build()
            .unwrap();

        let request = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "get_last_block_header",
            "params": {}
        });

        let json_rpc_url = format!("{rpc_url}/json_rpc");
        let get_transactions_url = format!("{rpc_url}/get_transactions");

        let top_height = client
            .get(&json_rpc_url)
            .json(&request)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap()
            .get("result")
            .unwrap()
            .get("block_header")
            .unwrap()
            .get("height")
            .unwrap()
            .as_u64()
            .unwrap();

        assert!(top_height > 3301441, "node is behind");

        Self {
            client,
            json_rpc_url,
            get_transactions_url,
            top_height,
        }
    }

    async fn get_block(&self, height: u64) -> GetBlockResponse {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "get_block",
            "params": {"height": height, "fill_pow_hash": true}
        });

        self.client
            .get(&self.json_rpc_url)
            .json(&request)
            .send()
            .await
            .unwrap()
            .json::<JsonRpcResponse>()
            .await
            .unwrap()
            .result
    }

    async fn get_transactions(&self, tx_hashes: Vec<[u8; 32]>) -> Vec<RpcTxData> {
        assert!(!tx_hashes.is_empty());

        #[derive(Debug, Clone, Deserialize)]
        struct GetTransactionsResponse {
            txs: Vec<Tx>,
        }

        #[derive(Debug, Clone, Deserialize)]
        struct Tx {
            as_hex: String,
            pruned_as_hex: String,
        }

        let txs_hashes = tx_hashes.iter().map(hex::encode).collect::<Vec<String>>();
        let request = json!({"txs_hashes":txs_hashes});

        let txs = self
            .client
            .get(&self.get_transactions_url)
            .json(&request)
            .send()
            .await
            .unwrap()
            .json::<GetTransactionsResponse>()
            .await
            .unwrap()
            .txs;

        assert_eq!(txs.len(), tx_hashes.len());

        txs.into_par_iter()
            .zip(tx_hashes)
            .map(|(r, tx_hash)| {
                let tx_blob = hex::decode(if r.as_hex.is_empty() {
                    r.pruned_as_hex
                } else {
                    r.as_hex
                })
                .unwrap();

                let tx = Transaction::read(&mut tx_blob.as_slice()).unwrap();

                RpcTxData {
                    tx,
                    tx_blob,
                    tx_hash,
                }
            })
            .collect()
    }

    pub async fn test(self, top_height: u64, tx: Sender<RpcBlockData>) {
        use futures::StreamExt;

        let iter = (0..top_height).map(|height| {
            let this = self.clone();
            let tx = tx.clone();

            async move {
                let get_block_response = this.get_block(height).await;

                let (this, get_block_response, block, txs) =
                    tokio::task::spawn_blocking(move || async move {
                        // Deserialize the block.
                        let block = Block::read(&mut get_block_response.blob.as_slice()).unwrap();

                        // Fetch and deserialize all transactions.
                        let mut tx_hashes = Vec::with_capacity(block.transactions.len() + 1);
                        tx_hashes.push(block.miner_transaction.hash());
                        tx_hashes.extend(block.transactions.iter());
                        let txs = this.get_transactions(tx_hashes).await;

                        (this, get_block_response, block, txs)
                    })
                    .await
                    .unwrap()
                    .await;

                let (seed_height, seed_hash) = if height < RANDOMX_START_HEIGHT {
                    (0, [0; 32])
                } else {
                    let seed_height = cuprate_consensus_rules::blocks::randomx_seed_height(
                        height.try_into().unwrap(),
                    )
                    .try_into()
                    .unwrap();

                    let seed_hash = this
                        .get_block(seed_height)
                        .await
                        .block_header
                        .hash
                        .try_into()
                        .unwrap();

                    (seed_height, seed_hash)
                };

                let data = RpcBlockData {
                    get_block_response,
                    block,
                    seed_height,
                    seed_hash,
                    txs,
                };

                tx.send(data).unwrap();
            }
        });

        futures::stream::iter(iter)
            .buffer_unordered(4) // This can't be too high or else we get bottlenecked by `monerod`
            .for_each(|()| async {})
            .await;
    }
}
