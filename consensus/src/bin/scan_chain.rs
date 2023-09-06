#![cfg(feature = "binaries")]

use futures::Stream;
use monero_serai::rpc::HttpRpc;
use std::pin::Pin;

use std::task::{Context, Poll};
use tower::balance::p2c::Balance;
use tower::discover::Change;
use tower::util::BoxService;
use tower::{Service, ServiceExt};

use monero_consensus::DatabaseRequest;
use tracing::level_filters::LevelFilter;

use monero_consensus::hardforks::HardFork;
use monero_consensus::pow::difficulty::DifficultyCalculator;
use monero_consensus::rpc::Rpc;

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
            Some(futures::future::ready(Attempts(self.0 - 1)))
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

    let rpc_discoverer = tower::discover::ServiceList::new(
        urls.into_iter()
            .map(|url| tower::load::Constant::new(Rpc::new_http(url), 0)),
    );
    let rpc_balance = Balance::new(rpc_discoverer);
    let rpc_buffer = tower::buffer::Buffer::new(BoxService::new(rpc_balance), 3);
    let mut rpc = tower::retry::Retry::new(Attempts(3), rpc_buffer);

    let pow_info = rpc
        .ready()
        .await
        .unwrap()
        .call(DatabaseRequest::BlockPOWInfo(64.into()))
        .await
        .unwrap();

    println!("{pow_info:?}");

    let difficulty = DifficultyCalculator::init_from_chain_height(2968227, rpc.clone())
        .await
        .unwrap();

    println!("{:?}", difficulty.next_difficulty(&HardFork::V16)); //257344482654

    //let _hfs = HardForks::init_at_chain_height(HardForkConfig::default(), 1009827, rpc.clone())
    //    .await
    //    .unwrap();
}
