mod pool;
mod router;

use std::{collections::HashMap, future::Future, hash::Hash, sync::Arc};

use futures::TryStreamExt;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tower::{
    discover::{Discover, ServiceList},
    util::service_fn,
    Service, ServiceExt,
};

use crate::{
    traits::{TxStoreRequest, TxStoreResponse},
    State,
};

pub fn mock_discover_svc<Req: Send + 'static>() -> (
    impl Discover<
        Key = usize,
        Service = impl Service<
            Req,
            Future = impl Future<Output = Result<(), tower::BoxError>> + Send + 'static,
            Error = tower::BoxError,
        > + Send
                      + 'static,
        Error = tower::BoxError,
    >,
    UnboundedReceiver<(u64, Req)>,
) {
    let (tx, rx) = mpsc::unbounded_channel();

    let discover = ServiceList::new((0..).map(move |i| {
        let tx_2 = tx.clone();

        service_fn(move |req| {
            tx_2.send((i, req)).unwrap();

            async move { Ok::<(), tower::BoxError>(()) }
        })
    }))
    .map_err(Into::into);

    (discover, rx)
}

pub fn mock_broadcast_svc<Req: Send + 'static>() -> (
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
pub fn mock_in_memory_backing_pool<
    Tx: Clone + Send + 'static,
    TxID: Clone + Hash + Eq + Send + 'static,
>() -> (
    impl Service<
            TxStoreRequest<Tx, TxID>,
            Response = TxStoreResponse<Tx, TxID>,
            Future = impl Future<Output = Result<TxStoreResponse<Tx, TxID>, tower::BoxError>>
                         + Send
                         + 'static,
            Error = tower::BoxError,
        > + Send
        + 'static,
    Arc<std::sync::Mutex<HashMap<TxID, (Tx, State)>>>,
) {
    let txs = Arc::new(std::sync::Mutex::new(HashMap::new()));
    let txs_2 = txs.clone();

    (
        service_fn(move |req: TxStoreRequest<Tx, TxID>| {
            let txs = txs.clone();
            async move {
                match req {
                    TxStoreRequest::Store(tx, tx_id, state) => {
                        txs.lock().unwrap().insert(tx_id, (tx, state));
                        Ok(TxStoreResponse::Ok)
                    }
                    TxStoreRequest::Get(tx_id) => {
                        let tx_state = txs.lock().unwrap().get(&tx_id).cloned();
                        Ok(TxStoreResponse::Transaction(tx_state))
                    }
                    TxStoreRequest::Contains(tx_id) => Ok(TxStoreResponse::Contains(
                        txs.lock().unwrap().get(&tx_id).map(|res| res.1),
                    )),
                    TxStoreRequest::IDsInStemPool => {
                        // horribly inefficient, but it's test code :)
                        let ids = txs
                            .lock()
                            .unwrap()
                            .iter()
                            .filter(|(_, (_, state))| matches!(state, State::Stem))
                            .map(|tx| tx.0.clone())
                            .collect::<Vec<_>>();

                        Ok(TxStoreResponse::IDs(ids))
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
