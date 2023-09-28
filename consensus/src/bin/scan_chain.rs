#![cfg(feature = "binaries")]

use futures::Stream;
use monero_serai::rpc::HttpRpc;
use std::pin::Pin;

use std::task::{Context, Poll};
use tower::discover::Change;

use tracing::level_filters::LevelFilter;

use monero_consensus::block::weight::BlockWeightsCache;
use monero_consensus::hardforks::HardFork;
use monero_consensus::rpc::{init_rpc_load_balancer, Rpc};

struct RpcDiscoverer(Vec<String>, u64);

impl Stream for RpcDiscoverer {
    type Item = Result<Change<u64, Rpc<HttpRpc>>, tower::BoxError>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if let Some(url) = this.0.pop() {
            this.1 += 1;
            return Poll::Ready(Some(Ok(Change::Insert(this.1, Rpc::new_http(url)))));
        }
        Poll::Ready(None)
    }
}

#[derive(Clone)]
pub struct Attempts(u64);

impl<Req: Clone, Res, E> tower::retry::Policy<Req, Res, E> for Attempts {
    type Future = futures::future::Ready<Self>;
    fn retry(&self, _: &Req, result: Result<&Res, &E>) -> Option<Self::Future> {
        if result.is_err() {
            Some(futures::future::ready(Attempts(self.0)))
        } else {
            None
        }
    }

    fn clone_request(&self, req: &Req) -> Option<Req> {
        Some(req.clone())
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::DEBUG)
        .init();

    let urls = vec![
        "http://xmr-node.cakewallet.com:18081".to_string(),
        "http://node.sethforprivacy.com".to_string(),
        "http://nodex.monerujo.io:18081".to_string(),
        "http://node.community.rino.io:18081".to_string(),
        "http://nodes.hashvault.pro:18081".to_string(),
        "http://node.moneroworld.com:18089".to_string(),
        "http://node.c3pool.com:18081".to_string(),
        //
        "http://xmr-node.cakewallet.com:18081".to_string(),
        "http://node.sethforprivacy.com".to_string(),
        "http://nodex.monerujo.io:18081".to_string(),
        "http://node.community.rino.io:18081".to_string(),
        "http://nodes.hashvault.pro:18081".to_string(),
        "http://node.moneroworld.com:18089".to_string(),
        "http://node.c3pool.com:18081".to_string(),
    ];

    let rpc = init_rpc_load_balancer(urls);

    let difficulty = BlockWeightsCache::init_from_chain_height(2984089, rpc.clone())
        .await
        .unwrap();

    println!(
        "{:?}",
        difficulty.next_block_long_term_weight(&HardFork::V15, 175819)
    );

    //  println!("{:?}", difficulty.next_difficulty(&HardFork::V1)); //774466376

    //let _hfs = HardForks::init_at_chain_height(HardForkConfig::default(), 1009827, rpc.clone())
    //    .await
    //    .unwrap();
}
