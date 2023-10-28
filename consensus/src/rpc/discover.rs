use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
    time::Duration,
};

use futures::{
    channel::mpsc::{self, SendError},
    stream::FuturesUnordered,
    SinkExt, StreamExt,
};
use monero_serai::rpc::HttpRpc;
use tokio::time::timeout;
use tower::{discover::Change, load::PeakEwma};
use tracing::instrument;

use super::{cache::ScanningCache, Rpc};

#[instrument(skip(cache))]
async fn check_rpc(addr: String, cache: Arc<RwLock<ScanningCache>>) -> Option<Rpc<HttpRpc>> {
    tracing::debug!("Sending request to node.");
    let rpc = HttpRpc::new(addr.clone()).ok()?;
    // make sure the RPC is actually reachable
    timeout(Duration::from_secs(2), rpc.get_height())
        .await
        .ok()?
        .ok()?;

    tracing::debug!("Node sent ok response.");

    Some(Rpc::new_http(addr, cache))
}

pub(crate) struct RPCDiscover {
    pub initial_list: Vec<String>,
    pub ok_channel: mpsc::Sender<Change<usize, PeakEwma<Rpc<HttpRpc>>>>,
    pub already_connected: HashSet<String>,
    pub cache: Arc<RwLock<ScanningCache>>,
}

impl RPCDiscover {
    async fn found_rpc(&mut self, rpc: Rpc<HttpRpc>) -> Result<(), SendError> {
        //if self.already_connected.contains(&rpc.addr) {
        //    return Ok(());
        //}

        tracing::info!("Connecting to node: {}", &rpc.addr);

        let addr = rpc.addr.clone();
        self.ok_channel
            .send(Change::Insert(
                self.already_connected.len(),
                PeakEwma::new(
                    rpc,
                    Duration::from_secs(5000),
                    300.0,
                    tower::load::CompleteOnResponse::default(),
                ),
            ))
            .await?;
        self.already_connected.insert(addr);

        Ok(())
    }

    pub async fn run(mut self) {
        if !self.initial_list.is_empty() {
            let mut fut = FuturesUnordered::from_iter(
                self.initial_list
                    .drain(..)
                    .map(|addr| check_rpc(addr, self.cache.clone())),
            );

            while let Some(res) = fut.next().await {
                if let Some(rpc) = res {
                    if self.found_rpc(rpc).await.is_err() {
                        tracing::info!("Stopping RPC discover channel closed!");
                        return;
                    }
                }
            }
        }
    }
}
