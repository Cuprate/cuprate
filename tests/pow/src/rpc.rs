use std::{
    collections::BTreeMap,
    ops::Range,
    sync::{atomic::Ordering, Mutex},
};

use function_name::named;
use hex::serde::deserialize;
use monero_serai::block::Block;
use randomx_rs::{RandomXCache, RandomXFlag, RandomXVM};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, ClientBuilder,
};
use serde::Deserialize;
use serde_json::{json, Value};
use thread_local::ThreadLocal;

use crate::TESTED_BLOCK_COUNT;

#[derive(Debug, Clone, Deserialize)]
struct JsonRpcResponse {
    result: GetBlockResponse,
}

#[derive(Debug, Clone, Deserialize)]
struct GetBlockResponse {
    #[serde(deserialize_with = "deserialize")]
    pub blob: Vec<u8>,
    pub block_header: BlockHeader,
}

#[derive(Debug, Clone, Deserialize)]
struct BlockHeader {
    #[serde(deserialize_with = "deserialize")]
    pub pow_hash: Vec<u8>,
    #[serde(deserialize_with = "deserialize")]
    pub hash: Vec<u8>,
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
            .unwrap()
            .try_into()
            .unwrap();

        assert!(top_height > 3301441, "node is behind");

        Self {
            client,
            rpc_url,
            top_height,
        }
    }

    async fn get_block(&self, height: usize) -> GetBlockResponse {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "get_block",
            "params": {"height": height, "fill_pow_hash": true}
        });

        let rpc_url = format!("{}/json_rpc", self.rpc_url);

        tokio::task::spawn(self.client.get(rpc_url).json(&request).send())
            .await
            .unwrap()
            .unwrap()
            .json::<JsonRpcResponse>()
            .await
            .unwrap()
            .result
    }

    async fn test<const RANDOMX: bool>(
        &self,
        range: Range<usize>,
        hash: impl Fn(Vec<u8>, u64, u64, [u8; 32]) -> [u8; 32] + Send + Sync + 'static + Copy,
        name: &'static str,
    ) {
        let tasks = range.map(|height| {
            let task = self.get_block(height);
            (height, task)
        });

        for (height, task) in tasks {
            let result = task.await;

            let (seed_height, seed_hash) = if RANDOMX {
                let seed_height = cuprate_consensus_rules::blocks::randomx_seed_height(height);

                let seed_hash: [u8; 32] = self
                    .get_block(seed_height)
                    .await
                    .block_header
                    .hash
                    .try_into()
                    .unwrap();

                (seed_height, seed_hash)
            } else {
                (0, [0; 32])
            };

            let top_height = self.top_height;

            #[expect(clippy::cast_precision_loss)]
            rayon::spawn(move || {
                let GetBlockResponse { blob, block_header } = result;
                let header = block_header;

                let block = match Block::read(&mut blob.as_slice()) {
                    Ok(b) => b,
                    Err(e) => panic!("{e:?}\nblob: {blob:?}, header: {header:?}"),
                };

                let pow_hash = hash(
                    block.serialize_pow_hash(),
                    height.try_into().unwrap(),
                    seed_height.try_into().unwrap(),
                    seed_hash,
                );

                assert_eq!(
                    header.pow_hash, pow_hash,
                    "\nheight: {height}\nheader: {header:#?}\nblock: {block:#?}"
                );

                let count = TESTED_BLOCK_COUNT.fetch_add(1, Ordering::Release) + 1;

                if std::env::var("VERBOSE").is_err() && count % 500 != 0 {
                    return;
                }

                let hash = hex::encode(pow_hash);
                let percent = (count as f64 / top_height as f64) * 100.0;

                println!(
                    "progress | {count}/{top_height} ({percent:.2}%)
height   | {height}
algo     | {name}
hash     | {hash}\n"
                );
            });
        }
    }

    #[named]
    pub(crate) async fn cryptonight_v0(&self) {
        self.test::<false>(
            0..1546000,
            |b, _, _, _| cuprate_cryptonight::cryptonight_hash_v0(&b),
            function_name!(),
        )
        .await;
    }

    #[named]
    pub(crate) async fn cryptonight_v1(&self) {
        self.test::<false>(
            1546000..1685555,
            |b, _, _, _| cuprate_cryptonight::cryptonight_hash_v1(&b).unwrap(),
            function_name!(),
        )
        .await;
    }

    #[named]
    pub(crate) async fn cryptonight_v2(&self) {
        self.test::<false>(
            1685555..1788000,
            |b, _, _, _| cuprate_cryptonight::cryptonight_hash_v2(&b),
            function_name!(),
        )
        .await;
    }

    #[named]
    pub(crate) async fn cryptonight_r(&self) {
        self.test::<false>(
            1788000..1978433,
            |b, h, _, _| cuprate_cryptonight::cryptonight_hash_r(&b, h),
            function_name!(),
        )
        .await;
    }

    #[named]
    pub(crate) async fn randomx(&self) {
        #[expect(clippy::significant_drop_tightening)]
        let function = move |bytes: Vec<u8>, _, seed_height, seed_hash: [u8; 32]| {
            static RANDOMX_VM: ThreadLocal<Mutex<BTreeMap<u64, RandomXVM>>> = ThreadLocal::new();

            let mut thread_local = RANDOMX_VM
                .get_or(|| Mutex::new(BTreeMap::new()))
                .lock()
                .unwrap();

            let randomx_vm = thread_local.entry(seed_height).or_insert_with(|| {
                let flag = RandomXFlag::get_recommended_flags();
                let cache = RandomXCache::new(flag, &seed_hash).unwrap();
                RandomXVM::new(flag, Some(cache), None).unwrap()
            });

            randomx_vm
                .calculate_hash(&bytes)
                .unwrap()
                .try_into()
                .unwrap()
        };

        self.test::<true>(1978433..self.top_height, function, function_name!())
            .await;
    }
}
