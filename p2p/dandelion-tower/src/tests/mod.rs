mod pool;
mod router;

use std::{collections::HashMap, future::Future, hash::Hash, sync::Arc};

use futures::{Stream, StreamExt, TryStreamExt};
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tower::{util::service_fn, Service, ServiceExt};

use crate::{
    traits::{TxStoreRequest, TxStoreResponse},
    OutboundPeer, State,
};

pub(crate) fn mock_discover_svc<Req: Send + 'static>() -> (
    impl Stream<
        Item = Result<
            OutboundPeer<
                usize,
                impl Service<
                        Req,
                        Future = impl Future<Output = Result<(), tower::BoxError>> + Send + 'static,
                        Error = tower::BoxError,
                    > + Send
                    + 'static,
            >,
            tower::BoxError,
        >,
    >,
    UnboundedReceiver<(usize, Req)>,
) {
    let (tx, rx) = mpsc::unbounded_channel();

    let discover = futures::stream::iter(0_usize..1_000_000)
        .map(move |i| {
            let tx_2 = tx.clone();

            Ok::<_, tower::BoxError>(OutboundPeer::Peer(
                i,
                service_fn(move |req| {
                    tx_2.send((i, req)).unwrap();

                    async move { Ok::<(), tower::BoxError>(()) }
                }),
            ))
        })
        .map_err(Into::into);

    (discover, rx)
}

pub(crate) fn mock_broadcast_svc<Req: Send + 'static>() -> (
    impl Service<
            Req,
            Future = impl Future<Output = Result<(), tower::BoxError>> + Send + 'static,
            Error = tower::BoxError,
        > + Send
        + 'static,
    UnboundedReceiver<Req>,
) {
    let (tx, rx) = mpsc::unbounded_channel();

    (
        service_fn(move |req| {
            tx.send(req).unwrap();

            async move { Ok::<(), tower::BoxError>(()) }
        }),
        rx,
    )
}

#[allow(clippy::type_complexity)] // just test code.
pub(crate) fn mock_in_memory_backing_pool<
    Tx: Clone + Send + 'static,
    TxID: Clone + Hash + Eq + Send + 'static,
>() -> (
    impl Service<
            TxStoreRequest<TxID>,
            Response = TxStoreResponse<Tx>,
            Future = impl Future<Output = Result<TxStoreResponse<Tx>, tower::BoxError>> + Send + 'static,
            Error = tower::BoxError,
        > + Send
        + 'static,
    Arc<std::sync::Mutex<HashMap<TxID, (Tx, State)>>>,
) {
    let txs = Arc::new(std::sync::Mutex::new(HashMap::new()));
    let txs_2 = Arc::clone(&txs);

    (
        service_fn(move |req: TxStoreRequest<TxID>| {
            let txs = Arc::clone(&txs);
            async move {
                match req {
                    TxStoreRequest::Get(tx_id) => {
                        let tx_state = txs.lock().unwrap().get(&tx_id).cloned();
                        Ok(TxStoreResponse::Transaction(tx_state))
                    }
                    TxStoreRequest::Promote(tx_id) => {
                        let _ = txs
                            .lock()
                            .unwrap()
                            .get_mut(&tx_id)
                            .map(|tx| tx.1 = State::Fluff);

                        Ok(TxStoreResponse::Ok)
                    }
                }
            }
        }),
        txs_2,
    )
}
