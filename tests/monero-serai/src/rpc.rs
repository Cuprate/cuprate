use std::{collections::BTreeSet, sync::atomic::Ordering};

use hex::serde::deserialize;
use monero_serai::{block::Block, transaction::Transaction};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, ClientBuilder,
};
use serde::Deserialize;
use serde_json::json;

use crate::{TESTED_BLOCK_COUNT, TESTED_TX_COUNT};

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct BlockHeader {
    #[serde(deserialize_with = "deserialize")]
    pub hash: Vec<u8>,
    #[serde(deserialize_with = "deserialize")]
    pub miner_tx_hash: Vec<u8>,
    #[serde(deserialize_with = "deserialize")]
    pub prev_hash: Vec<u8>,

    pub block_weight: usize,
    pub height: usize,
    pub major_version: u8,
    pub minor_version: u8,
    pub nonce: u32,
    pub num_txes: usize,
    pub reward: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct RpcClient {
    client: Client,
    rpc_url: String,
    pub top_height: usize,
}

impl RpcClient {
    pub(crate) async fn new(rpc_url: String) -> Self {
        let headers = {
            let mut h = HeaderMap::new();
            h.insert("Content-Type", HeaderValue::from_static("application/json"));
            h
        };

        let client = ClientBuilder::new()
            .default_headers(headers)
            .build()
            .unwrap();

        #[derive(Debug, Clone, Deserialize)]
        struct JsonRpcResponse {
            result: GetLastBlockHeaderResponse,
        }

        #[derive(Debug, Clone, Deserialize)]
        pub(crate) struct GetLastBlockHeaderResponse {
            pub block_header: BlockHeader,
        }

        let request = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "get_last_block_header",
            "params": {}
        });

        let top_height = client
            .get(format!("{rpc_url}/json_rpc"))
            .json(&request)
            .send()
            .await
            .unwrap()
            .json::<JsonRpcResponse>()
            .await
            .unwrap()
            .result
            .block_header
            .height;

        println!("top_height: {top_height}");
        assert!(top_height > 3301441, "node is behind");

        Self {
            client,
            rpc_url,
            top_height,
        }
    }

    async fn get_transactions(&self, tx_hashes: Vec<[u8; 32]>) -> Vec<(Transaction, Vec<u8>)> {
        assert!(!tx_hashes.is_empty());

        #[derive(Debug, Clone, Deserialize)]
        pub(crate) struct GetTransactionsResponse {
            pub txs: Vec<Tx>,
        }

        #[derive(Debug, Clone, Deserialize)]
        pub(crate) struct Tx {
            pub as_hex: String,
            pub pruned_as_hex: String,
        }

        let url = format!("{}/get_transactions", self.rpc_url);

        let txs_hashes = tx_hashes
            .into_iter()
            .map(hex::encode)
            .collect::<Vec<String>>();

        let request = json!({
            "txs_hashes": txs_hashes,
        });

        let txs = self
            .client
            .get(&url)
            .json(&request)
            .send()
            .await
            .unwrap()
            .json::<GetTransactionsResponse>()
            .await
            .unwrap()
            .txs;

        txs.into_par_iter()
            .map(|r| {
                let blob = hex::decode(if r.as_hex.is_empty() {
                    r.pruned_as_hex
                } else {
                    r.as_hex
                })
                .unwrap();

                (Transaction::read(&mut blob.as_slice()).unwrap(), blob)
            })
            .collect()
    }

    pub(crate) async fn get_block_test_batch(&self, heights: BTreeSet<usize>) {
        #[derive(Debug, Clone, Deserialize)]
        struct JsonRpcResponse {
            result: GetBlockResponse,
        }

        #[derive(Debug, Clone, Deserialize)]
        pub(crate) struct GetBlockResponse {
            #[serde(deserialize_with = "deserialize")]
            pub blob: Vec<u8>,
            pub block_header: BlockHeader,
        }

        let tasks = heights.into_iter().map(|height| {
            let json_rpc_url = format!("{}/json_rpc", self.rpc_url);

            let request = json!({
                "jsonrpc": "2.0",
                "id": 0,
                "method": "get_block",
                "params": {"height": height}
            });

            let task = tokio::task::spawn(self.client.get(&json_rpc_url).json(&request).send());

            (height, task)
        });

        for (height, task) in tasks {
            let resp = task
                .await
                .unwrap()
                .unwrap()
                .json::<JsonRpcResponse>()
                .await
                .unwrap()
                .result;

            let info = format!("\nheight: {height}\nresponse: {resp:#?}");

            // Test block deserialization.
            let block = match Block::read(&mut resp.blob.as_slice()) {
                Ok(b) => b,
                Err(e) => panic!("{e:?}\n{info}"),
            };

            // Fetch all transactions.
            let mut tx_hashes = vec![block.miner_transaction.hash()];
            tx_hashes.extend(block.transactions.iter());
            let txs = self.get_transactions(tx_hashes.clone()).await;
            assert_eq!(tx_hashes.len(), txs.len());

            let top_height = self.top_height;

            #[expect(clippy::cast_precision_loss)]
            rayon::spawn(move || {
                // Test block properties.
                assert_eq!(resp.blob, block.serialize(), "{info}");

                assert!(
                    !block.miner_transaction.prefix().outputs.is_empty(),
                    "miner_tx has no outputs\n{info}"
                );

                let block_reward = block
                    .miner_transaction
                    .prefix()
                    .outputs
                    .iter()
                    .map(|o| o.amount.unwrap())
                    .sum::<u64>();
                assert_ne!(block_reward, 0, "block reward is 0\n{info}");

                // Test fields are correct.
                let BlockHeader {
                    block_weight,
                    hash,
                    height,
                    major_version,
                    minor_version,
                    miner_tx_hash,
                    nonce,
                    num_txes,
                    prev_hash,
                    reward,
                    timestamp,
                } = resp.block_header;

                let total_block_weight = txs.iter().map(|(tx, _)| tx.weight()).sum::<usize>();

                // Test transaction properties.
                txs.into_par_iter()
                    .zip(tx_hashes)
                    .for_each(|((tx, blob), hash)| {
                        assert_eq!(hash, tx.hash(), "{info}, tx: {tx:#?}");
                        assert_ne!(tx.weight(), 0, "{info}, tx: {tx:#?}");
                        assert!(!tx.prefix().inputs.is_empty(), "{info}, tx: {tx:#?}");
                        assert_eq!(blob, tx.serialize(), "{info}, tx: {tx:#?}");
                        assert!(matches!(tx.version(), 1 | 2), "{info}, tx: {tx:#?}");
                    });

                assert_eq!(block_weight, total_block_weight, "{info}");
                assert_ne!(block.miner_transaction.weight(), 0, "{info}");
                assert_eq!(hash, block.hash(), "{info}");
                assert_eq!(height, block.number().unwrap(), "{info}");
                assert_eq!(major_version, block.header.hardfork_version, "{info}");
                assert_eq!(minor_version, block.header.hardfork_signal, "{info}");
                assert_eq!(miner_tx_hash, block.miner_transaction.hash(), "{info}");
                assert_eq!(nonce, block.header.nonce, "{info}");
                assert_eq!(num_txes, block.transactions.len(), "{info}");
                assert_eq!(prev_hash, block.header.previous, "{info}");
                assert_eq!(reward, block_reward, "{info}");
                assert_eq!(timestamp, block.header.timestamp, "{info}");

                let progress = TESTED_BLOCK_COUNT.fetch_add(1, Ordering::Release) + 1;
                let tx_count = TESTED_TX_COUNT.fetch_add(num_txes, Ordering::Release) + 1;
                let percent = (progress as f64 / top_height as f64) * 100.0;

                println!(
                    "progress        | {progress}/{top_height} ({percent:.2}%)
tx_count        | {tx_count}
hash            | {}
miner_tx_hash   | {}
prev_hash       | {}
reward          | {}
timestamp       | {}
nonce           | {}
height          | {}
block_weight    | {}
miner_tx_weight | {}
major_version   | {}
minor_version   | {}
num_txes        | {}\n",
                    hex::encode(hash),
                    hex::encode(miner_tx_hash),
                    hex::encode(prev_hash),
                    reward,
                    timestamp,
                    nonce,
                    height,
                    block_weight,
                    block.miner_transaction.weight(),
                    major_version,
                    minor_version,
                    num_txes,
                );
            });
        }
    }
}
