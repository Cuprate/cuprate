//! Semaphore Limit service
//!
//! Semaphore limiting for the blocks endpoints of the RPC server:
//!
//! - `/get_blocks.bin`
//! - `/get_blocks_by_height.bin` (and their aliases)
//! - `/json_rpc` => `get_block` or `getblock` method
//!
//! At most `N` requests are handled simultaneously; additional requests are
//! put on hold for a specific period in milliseconds. Requests still queued past that
//! period are rejected with `503 Service Unavailable` and a `Retry-After` header.
//!
//! The permit is held until the response has been fully sent (or dropped).

use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use axum::{
    body::{Body, Bytes},
    extract::MatchedPath,
    http::{header::RETRY_AFTER, Method},
};
use hyper::{
    body::{Body as HttpBody, Frame, SizeHint},
    Request, Response, StatusCode,
};
use pin_project_lite::pin_project;
use serde::Deserialize;
use tokio::{
    sync::{OwnedSemaphorePermit, Semaphore},
    time::timeout,
};
use tower::{Layer, Service};

/// A look at the `method` field of a `/json_rpc` payload.
#[derive(Deserialize)]
struct MethodLook<'a> {
    method: Option<&'a str>,
}

impl MethodLook<'_> {
    /// Whether a JSON-RPC method is a blocks method.
    fn is_blocks(method: &str) -> bool {
        method == "get_block" || method == "getblock"
    }
}

pin_project! {
    #[project = BlockSemaphoreResponseProj]
    pub(super) enum ResponseFuture<F, E> {
        /// Not a blocks endpoint, forwarded.
        Future {
            #[pin]
            future: F,
        },
        /// A blocks endpoint request waiting for a concurrency permit,
        /// in order to be served.
        Queued {
            #[pin]
            future: Pin<Box<dyn Future<Output = Result<Response<Body>, E>> + Send>>,
        },
    }
}

impl<F, E> Future for ResponseFuture<F, E>
where
    F: Future<Output = Result<Response<Body>, E>>,
{
    type Output = Result<Response<Body>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            BlockSemaphoreResponseProj::Future { future } => future.poll(cx),
            BlockSemaphoreResponseProj::Queued { future } => future.poll(cx),
        }
    }
}

enum EndpointType {
    Block,
    JsonRpc,
    Other,
}

/// A response body holding the semaphore permit until the stream
/// ends or is dropped.
struct PermitBody {
    inner: Body,
    /// This permit is freed when the Body is dropped.
    _permit: OwnedSemaphorePermit,
}

impl HttpBody for PermitBody {
    type Data = Bytes;
    type Error = axum::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Pin::new(&mut self.inner).poll_frame(cx)
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

#[derive(Clone)]
pub(super) struct BlockSemaphoreLimit<S> {
    inner: S,
    semaphore: Arc<Semaphore>,
    timeout: Duration,
    retry_after: Arc<str>,
}

impl<S> BlockSemaphoreLimit<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    /// Wait for a concurrency permit for up to `timeout`, then serve the
    /// request holding the permit, or respond `503 Service Unavailable`
    /// with a `Retry-After` header.
    async fn acquire_and_serve(
        mut inner: S,
        semaphore: Arc<Semaphore>,
        wait: Duration,
        retry_after: Arc<str>,
        request: Request<Body>,
    ) -> Result<Response<Body>, S::Error> {
        let Ok(Ok(permit)) = timeout(wait, semaphore.acquire_owned()).await else {
            // Too much time in queue, go back to client.
            return Ok(Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header(RETRY_AFTER, retry_after.as_ref())
                .body(Body::empty())
                .unwrap());
        };

        let response = inner.call(request).await?;

        Ok(response.map(|body| {
            Body::new(PermitBody {
                inner: body,
                _permit: permit,
            })
        }))
    }
}

impl<S> Service<Request<Body>> for BlockSemaphoreLimit<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = ResponseFuture<S::Future, S::Error>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        // Get the endpoint type to decide whether to queue or forward the request.
        let endpoint = match request.extensions().get::<MatchedPath>() {
            Some(path)
                if matches!(
                    path.as_str(),
                    "/get_blocks.bin"
                        | "/getblocks.bin"
                        | "/get_blocks_by_height.bin"
                        | "/getblocks_by_height.bin"
                ) =>
            {
                EndpointType::Block
            }
            Some(path) if path.as_str() == "/json_rpc" => EndpointType::JsonRpc,
            _ => EndpointType::Other,
        };

        match endpoint {
            EndpointType::JsonRpc => {
                let mut inner = self.inner.clone();
                let semaphore = Arc::clone(&self.semaphore);
                let timeout = self.timeout;
                let retry_after = Arc::clone(&self.retry_after);

                ResponseFuture::Queued {
                    future: Box::pin(async move {
                        // Buffer the body, look at the method in the JSON payload.
                        let (parts, body) = request.into_parts();

                        let Ok(bytes) = axum::body::to_bytes(body, usize::MAX).await else {
                            return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::empty())
                                .unwrap());
                        };

                        let is_blocks = serde_json::from_slice::<MethodLook<'_>>(&bytes)
                            .ok()
                            .and_then(|peek| peek.method)
                            .is_some_and(MethodLook::is_blocks);

                        let request = Request::from_parts(parts, Body::from(bytes));

                        if is_blocks {
                            Self::acquire_and_serve(inner, semaphore, timeout, retry_after, request)
                                .await
                        } else {
                            inner.call(request).await
                        }
                    }),
                }
            }
            EndpointType::Block => ResponseFuture::Queued {
                future: Box::pin(Self::acquire_and_serve(
                    self.inner.clone(),
                    Arc::clone(&self.semaphore),
                    self.timeout,
                    Arc::clone(&self.retry_after),
                    request,
                )),
            },
            EndpointType::Other => ResponseFuture::Future {
                future: self.inner.call(request),
            },
        }
    }
}

/// A [`Layer`] that applies the blocks endpoint [`BlockSemaphoreLimit`].
#[derive(Clone)]
pub(super) struct BlockSemaphoreLimitLayer {
    semaphore: Arc<Semaphore>,
    timeout: Duration,
    retry_after: Arc<str>,
}

impl BlockSemaphoreLimitLayer {
    pub fn new(limit: u64, timeout: Duration) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(usize::try_from(limit).unwrap_or(usize::MAX))),
            timeout,
            retry_after: timeout.as_millis().div_ceil(1000).to_string().into(),
        }
    }
}

impl<S> Layer<S> for BlockSemaphoreLimitLayer {
    type Service = BlockSemaphoreLimit<S>;

    fn layer(&self, inner: S) -> Self::Service {
        BlockSemaphoreLimit {
            inner,
            semaphore: Arc::clone(&self.semaphore),
            timeout: self.timeout,
            retry_after: Arc::clone(&self.retry_after),
        }
    }
}
