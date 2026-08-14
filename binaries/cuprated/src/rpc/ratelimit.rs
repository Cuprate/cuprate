//! Rate Limit service
//!
//! Per-IP size-time-budget rate limiting for the RPC server.
//!
//! Every IP address (from the connection info inserted into the request extensions
//! by [`Router::into_make_service_with_connect_info`], see [`crate::rpc::server`]) holds
//! two budgets:
//!
//! - a size budget (bytes), debited by the size of every response it receives,
//! - a time budget (milliseconds), debited by the processing time of every
//!   one of its requests.
//!
//! Both budgets are refilled by a fixed income per second, capped at their
//! configured maximum. A request received while either budget is exhausted is
//! rejected immediately with `429 Too Many Requests`; the request that
//! exhausts a budget is still served in full, pushing the budget negative to
//! punish overshoot.
//!
//! Per-IP state is erased by [`crate::rpc::server`] once the IP's last
//! connection has been closed for a grace period, keeping the cache proportional
//! to the amount of recently-connected IP addresses.
//!
//! [`Router::into_make_service_with_connect_info`]: axum::Router::into_make_service_with_connect_info

use std::{
    future::Future,
    mem::take,
    net::{IpAddr, SocketAddr},
    pin::Pin,
    sync::Arc,
    task::{ready, Context, Poll},
};

use axum::{
    body::{Body, Bytes},
    extract::connect_info::ConnectInfo,
};
use dashmap::DashMap;
use hyper::{
    body::{Body as HttpBody, Frame, SizeHint},
    Request, Response, StatusCode,
};
use pin_project_lite::pin_project;
use tokio::time::Instant;
use tower::{Layer, Service};

/// The per-IP rate limit budgets of an RPC server, from the RPC configuration.
#[derive(Copy, Clone, Debug)]
pub(super) struct RateLimitBudget {
    /// The maximum size budget. The amount of response bytes an IP address
    /// can hold at once.
    max_budget_size: i64,
    /// The size budget income. The amount of response bytes refilled
    /// per second.
    income_size: i64,
    /// The maximum time budget. The amount of request processing time,
    /// in milliseconds, an IP address can hold at once.
    max_budget_time: i64,
    /// The time budget income. The amount of milliseconds refilled
    /// per second.
    income_time: i64,
}

impl RateLimitBudget {
    /// Create a new `RateLimitBudget`.
    pub fn new(
        max_budget_size: u64,
        income_size: u64,
        max_budget_time: u64,
        income_time: u64,
    ) -> Self {
        Self {
            max_budget_size: i64::try_from(max_budget_size).unwrap_or(i64::MAX),
            income_size: i64::try_from(income_size).unwrap_or(i64::MAX),
            max_budget_time: i64::try_from(max_budget_time).unwrap_or(i64::MAX),
            income_time: i64::try_from(income_time).unwrap_or(i64::MAX),
        }
    }
}

pin_project! {
    #[project = ResponseFutureProj]
    pub(super) enum ResponseFuture<F> {
        /// The request was rate limited: respond `429 Too Many Requests`.
        RateLimited,
        /// The request is being served, its processing time and response size
        /// measured and debited from the IP address' budgets.
        Future {
            #[pin]
            future: F,
            start: Instant,
            ip: IpAddr,
            cache: Arc<RpcRateLimitCache>,
        },
    }
}

impl<F, E> Future for ResponseFuture<F>
where
    F: Future<Output = Result<Response<Body>, E>>,
{
    type Output = Result<Response<Body>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            ResponseFutureProj::RateLimited => Poll::Ready(Ok(Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .body(Body::empty())
                .unwrap())),
            ResponseFutureProj::Future {
                future,
                start,
                ip,
                cache,
            } => {
                let response = ready!(future.poll(cx))?;

                // The request has finished processing. Time to debit its budget.
                let millis = i64::try_from(start.elapsed().as_millis()).unwrap_or(i64::MAX);
                cache.debit_time(ip, millis);

                // Count streamed body bytes, debiting the response size.
                Poll::Ready(Ok(response.map(|body| {
                    Body::new(DebitBody::new(body, Arc::clone(cache), *ip))
                })))
            }
        }
    }
}

/// The double budget of an IP address.
struct Budget {
    /// Remaining response bytes.
    size: i64,
    /// Remaining request processing milliseconds.
    time: i64,
    /// The instant the last response was served (or dropped),
    /// for refill.
    last_request: Instant,
}

/// The shared per-IP rate limit budgets of an RPC server.
pub(super) struct RpcRateLimitCache {
    budgets: DashMap<IpAddr, Budget>,
    /// The budget configuration.
    budget: RateLimitBudget,
}

impl RpcRateLimitCache {
    /// Create a new `RpcRateLimitCache` with the given budget configuration.
    pub(super) fn new(budget: RateLimitBudget) -> Self {
        Self {
            budgets: DashMap::new(),
            budget,
        }
    }

    /// Refill the IP's budgets by their incomes and check if a request is
    /// admitted: `true` if both budgets are positive, `false` if the request
    /// must be rejected with `429 Too Many Requests`.
    fn check(&self, ip: IpAddr) -> bool {
        let budget = self.budget;
        let now = Instant::now();

        let mut entry = self.budgets.entry(ip).or_insert(Budget {
            size: budget.max_budget_size,
            time: budget.max_budget_time,
            last_request: now,
        });

        // Refill the budgets by their respective incomes.
        let elapsed =
            i64::try_from(now.duration_since(entry.last_request).as_millis()).unwrap_or(i64::MAX);

        let refill_size = budget.income_size.saturating_mul(elapsed) / 1000;
        let refill_time = budget.income_time.saturating_mul(elapsed) / 1000;

        entry.size = budget
            .max_budget_size
            .min(entry.size.saturating_add(refill_size));
        entry.time = budget
            .max_budget_time
            .min(entry.time.saturating_add(refill_time));

        entry.size > 0 && entry.time > 0
    }

    /// Debit response bytes from the IP's size budget, if still tracked.
    fn debit_size(&self, ip: &IpAddr, bytes: i64) {
        if let Some(mut budget) = self.budgets.get_mut(ip) {
            budget.size -= bytes;
            // Last request is updated here since called after sending the request.
            budget.last_request = Instant::now();
        }
    }

    /// Debit request processing milliseconds from the IP's time budget,
    /// if still tracked.
    fn debit_time(&self, ip: &IpAddr, millis: i64) {
        if let Some(mut budget) = self.budgets.get_mut(ip) {
            budget.time -= millis;
        }
    }

    /// Remove the rate limit state of an IP address.
    pub(super) fn remove_ip(&self, ip: &IpAddr) {
        self.budgets.remove(ip);
    }
}

/// A response body wrapper counting streamed bytes, debiting them from the
/// IP address' size budget once the stream ends or is dropped.
struct DebitBody {
    inner: Body,
    /// Number of bytes counted in the stream so far
    counted: i64,
    /// Atomic reference to the rate limit cache
    cache: Arc<RpcRateLimitCache>,
    /// IP address to debit budget from
    ip: IpAddr,
}

impl DebitBody {
    const fn new(inner: Body, cache: Arc<RpcRateLimitCache>, ip: IpAddr) -> Self {
        Self {
            inner,
            counted: 0,
            cache,
            ip,
        }
    }

    /// Debit the bytes counted so far.
    fn debit(&mut self) {
        self.cache.debit_size(&self.ip, take(&mut self.counted));
    }
}

impl HttpBody for DebitBody {
    type Data = Bytes;
    type Error = axum::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match ready!(Pin::new(&mut self.inner).poll_frame(cx)) {
            Some(Ok(frame)) => {
                if let Some(data) = frame.data_ref() {
                    self.counted += i64::try_from(data.len()).unwrap_or(i64::MAX);
                }
                Poll::Ready(Some(Ok(frame)))
            }
            // The stream ended. debit the bytes counted so far.
            result => {
                self.debit();
                Poll::Ready(result)
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

impl Drop for DebitBody {
    /// Mandatory to avoid disconnecting clients from not being debited.
    fn drop(&mut self) {
        self.debit();
    }
}

/// Enforces a per-IP size-time-budget rate limit: requests received while
/// either budget is exhausted are rejected with `429 Too Many Requests`.
/// (see `rpc::ratelimit.rs`)
#[derive(Clone)]
pub(super) struct RateLimit<S> {
    inner: S,
    cache: Arc<RpcRateLimitCache>,
}

impl<S> Service<Request<Body>> for RateLimit<S>
where
    S: Service<Request<Body>, Response = Response<Body>>,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = ResponseFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        // The client IP address is inserted, if available, into the request extensions
        // by `Router::into_make_service_with_connect_info` (see `rpc::server`).
        let Some(ip) = request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(addr)| addr.ip())
        else {
            panic!("Service must be wrapped inside of a IntoMakeConnectionInfo");
        };

        if !self.cache.check(ip) {
            return ResponseFuture::RateLimited;
        }

        ResponseFuture::Future {
            future: self.inner.call(request),
            start: Instant::now(),
            ip,
            cache: Arc::clone(&self.cache),
        }
    }
}

/// A [`Layer`] that applies per-IP size-time-budget rate limiting, rejecting
/// excess requests with `429 Too Many Requests`.
#[derive(Clone)]
pub(super) struct RateLimitLayer {
    /// Atomic reference to the rate limit cache
    cache: Arc<RpcRateLimitCache>,
}

impl RateLimitLayer {
    /// Create a new `RateLimitLayer` with the given rate limit cache.
    pub const fn new(cache: Arc<RpcRateLimitCache>) -> Self {
        Self { cache }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimit<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimit {
            inner,
            cache: Arc::clone(&self.cache),
        }
    }
}
