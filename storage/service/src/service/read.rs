use std::{
    sync::Arc,
    task::{Context, Poll},
};

use futures::channel::oneshot;
use rayon::ThreadPool;
use tower::Service;

use cuprate_database::ConcreteEnv;
use cuprate_helper::asynch::InfallibleOneshotReceiver;

/// The [`rayon::ThreadPool`] service.
///
/// Uses an inner request handler and a rayon thread-pool to asynchronously handler requests.
pub struct DatabaseReadService<Req, Res, Err> {
    /// The rayon thread-pool.
    pool: Arc<ThreadPool>,

    /// The function used to handle request.
    inner_handler: Arc<dyn Fn(Req) -> Result<Res, Err> + Send + Sync + 'static>,
}

impl<Req, Res, Err> Clone for DatabaseReadService<Req, Res, Err> {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            inner_handler: self.inner_handler.clone(),
        }
    }
}

impl<Req, Res, Err> DatabaseReadService<Req, Res, Err>
where
    Req: Send + 'static,
    Res: Send + 'static,
    Err: Send + 'static,
{
    pub fn new(
        env: Arc<ConcreteEnv>,
        pool: Arc<ThreadPool>,
        req_handler: impl Fn(&ConcreteEnv, Req) -> Result<Res, Err> + Send + Sync + 'static,
    ) -> Self {
        let inner_handler = Arc::new(move |req| req_handler(&env, req));

        Self {
            pool,
            inner_handler,
        }
    }
}

impl<Req, Res, Err> Service<Req> for DatabaseReadService<Req, Res, Err>
where
    Req: Send + 'static,
    Res: Send + 'static,
    Err: Send + 'static,
{
    type Response = Res;
    type Error = Err;
    type Future = InfallibleOneshotReceiver<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Req) -> Self::Future {
        // Response channel we `.await` on.
        let (response_sender, receiver) = oneshot::channel();

        let handler = self.inner_handler.clone();

        // Spawn the request in the rayon DB thread-pool.
        //
        // Note that this uses `self.pool` instead of `rayon::spawn`
        // such that any `rayon` parallel code that runs within
        // the passed closure uses the same `rayon` threadpool.
        self.pool.spawn(move || {
            drop(response_sender.send(handler(req)));
        });

        InfallibleOneshotReceiver::from(receiver)
    }
}
