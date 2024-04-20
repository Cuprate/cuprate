use std::future::Future;

use futures::TryStreamExt;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tower::{
    discover::{Discover, ServiceList},
    util::service_fn,
    Service,
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
