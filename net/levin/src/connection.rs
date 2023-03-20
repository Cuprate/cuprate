mod bucket_builder;

use std::{sync::mpsc::Sender, fmt::Debug};

use futures::{AsyncRead, AsyncWrite, SinkExt, StreamExt, stream::Fuse};
use async_trait::async_trait;
use thiserror::Error;

use crate::bucket::{Bucket, BucketError};
use crate::bucket::bucket_stream::BucketStream;
use crate::bucket::bucket_sink::BucketSink;

pub use bucket_builder::BucketBuilder;

pub struct Disconnected;

/// A simple connection tracker that sends a `Disconnected` message down a MPSC channel
/// when the peer disconnects
pub struct ConnectionTracker {
    tx: Sender<Disconnected>,
}

impl Drop for ConnectionTracker {
    fn drop(&mut self) {
        let _ = self.tx.send(Disconnected);
    }
}

pub type ClientResponseChan<T> = futures::channel::oneshot::Sender<Result<Option<T>, RequestError>>;

#[async_trait]
pub trait LevinService {
    type Error: Debug + Into<ConnectionError>;
    type ClientRequest: ClientRequest<Self::PeerResponse>;
    type PeerResponse: PeerResponse;
    type Response: Into<BucketBuilder>;

    async fn handle_notification(
        &mut self,
        command: u32,
        body: bytes::Bytes,
    ) -> Result<Option<Self::Response>, Self::Error>;
    async fn handle_request(&mut self, command: u32, body: bytes::Bytes) -> Result<Self::Response, Self::Error>;
    async fn handle_error(&mut self, command: u32, error: i32) -> Result<(), Self::Error>;
    async fn shutdown(&mut self);
}

pub trait ClientRequest<T>: Into<BucketBuilder> {
    fn is_levin_request(&self) -> bool;
    /// This must return Some(tx) on first call
    fn tx(&mut self) -> Option<ClientResponseChan<T>>;
    /// Will not be called if `is_levin_request` returns false
    fn command(&self) -> u32;
}

pub trait PeerResponse: Sized {
    fn decode(command: u32, body: bytes::Bytes) -> Result<Self, BucketError>;
}

pub struct RequestError;

enum State<T> {
    AwaitingRequest,
    AwaitingResponse { command: u32, tx: ClientResponseChan<T> },
}

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("Peer sent unexpected response")]
    PeerSentUnexpectedResponse,
    #[error("Peer sent wrong response expected: {exp}, got: {got}")]
    PeerSentWrongResponse { exp: u32, got: u32 },
    #[error("Internal client sent a request without response channel")]
    ClientRequestDidNotIncludeResponseChan,
    #[error("The client message channel(s) was closed")]
    ClientChanClosed,
    #[error("Failed to construct bucket: {0}")]
    FailedToConstructBucket(&'static str),
    #[error("Bucket error: {0}")]
    BucketError(#[from] BucketError),
    #[error("Internal Service Error")]
    ServiceError { saveable: bool },
}

pub struct LevinConnection<Srm, Snk, Svc>
where
    Srm: AsyncRead + std::marker::Unpin,
    Snk: AsyncWrite + std::marker::Unpin,
    Svc: LevinService,
{
    stream: Fuse<BucketStream<Srm>>,
    sink: BucketSink<Snk>,
    svc: Svc,
    state: State<Svc::PeerResponse>,
    internal_rx: Fuse<futures::channel::mpsc::Receiver<Svc::ClientRequest>>,
    #[allow(dead_code)]
    tracker: ConnectionTracker,
}

impl<Srm, Snk, Svc> LevinConnection<Srm, Snk, Svc>
where
    Srm: AsyncRead + std::marker::Unpin,
    Snk: AsyncWrite + std::marker::Unpin,
    Svc: LevinService,
{
    async fn send_bucket_to_peer(&mut self, bucket: Bucket) -> Result<(), ConnectionError> {
        self.sink.send(bucket).await?;
        Ok(())
    }

    async fn handle_new_bucket(&mut self, bucket: Bucket) -> Result<(), ConnectionError> {
        if bucket.header.is_error() {
            self.svc
                .handle_error(bucket.header.command, bucket.header.return_code)
                .await
                .map_err(|e| e.into())?;
            Ok(())
        } else if bucket.header.flags.is_request() {
            let res = self
                .svc
                .handle_request(bucket.header.command, bucket.body)
                .await
                .map_err(|e| e.into())?;
            let bucket_builder: BucketBuilder = res.into();
            self.send_bucket_to_peer(bucket_builder.try_into()?).await?;
            Ok(())
        }
        // notifications
        else if bucket.header.flags.is_response() && !bucket.header.have_to_return_data {
            let res = self
                .svc
                .handle_notification(bucket.header.command, bucket.body)
                .await
                .map_err(|e| e.into())?;
            if let Some(res) = res {
                let bucket_builder: BucketBuilder = res.into();
                self.send_bucket_to_peer(bucket_builder.try_into()?).await?;
            }
            Ok(())
        } else {
            Err(ConnectionError::PeerSentUnexpectedResponse)
        }
    }

    async fn handle_new_bucket_potential_response(&mut self, bucket: Bucket) -> Result<(), ConnectionError> {
        if !bucket.header.flags.is_response() || !bucket.header.have_to_return_data {
            // notifications and requests
            self.handle_new_bucket(bucket).await?;
            Ok(())
        } else {
            // we can do this because responses must come back in order
            let state = std::mem::replace(&mut self.state, State::AwaitingRequest);
            if let State::AwaitingResponse { command, tx } = state {
                // we know the bucket is a response
                if bucket.header.command == command {
                    // Levin checks if the return code is bigger than 1 (true) for responses, which is
                    // a bit annoying.
                    if !bucket.header.is_ok() {
                        let _ = tx.send(Err(RequestError));
                    } else {
                        let _ = tx.send(Ok(Some(Svc::PeerResponse::decode(command, bucket.body)?)));
                    }
                    Ok(())
                } else {
                    Err(ConnectionError::PeerSentWrongResponse {
                        exp: command,
                        got: bucket.header.command,
                    })
                }
            } else {
                unreachable!("this function will only be called when in State::AwaitingResponse")
            }
        }
    }

    async fn handle_new_internal_client_message(
        &mut self,
        mut message: Svc::ClientRequest,
    ) -> Result<(), ConnectionError> {
        if message.is_levin_request() {
            let tx = message
                .tx()
                .ok_or(ConnectionError::ClientRequestDidNotIncludeResponseChan)?;
            self.state = State::AwaitingResponse {
                command: message.command(),
                tx,
            };
        }
        let bucket_builder: BucketBuilder = message.into();
        let bucket: Bucket = bucket_builder.try_into()?;
        self.send_bucket_to_peer(bucket).await?;
        Ok(())
    }

    async fn state_awaiting_request(&mut self) -> Result<(), ConnectionError> {
        futures::select! {
        peer_bucket = self.stream.next()  => {
            let peer_bucket = peer_bucket.expect("Bucket stream will never return None")?;
            self.handle_new_bucket(peer_bucket).await?;
            },
            message = self.internal_rx.next() => {
            let message = message.ok_or(ConnectionError::ClientChanClosed)?;
            self.handle_new_internal_client_message(message).await?;
            }
        }
        Ok(())
    }

    async fn handle_awaiting_response(&mut self) -> Result<(), ConnectionError> {
        let potential_response = self
            .stream
            .next()
            .await
            .expect("Bucket stream will never return None")?;
        self.handle_new_bucket_potential_response(potential_response).await
    }

    pub async fn run(mut self) {
        loop {
            if let Err(_) = match self.state {
                State::AwaitingRequest => self.state_awaiting_request().await,
                State::AwaitingResponse { .. } => self.handle_awaiting_response().await,
            } {
                self.shutdown().await;
                return;
            }
        }
    }

    async fn shutdown(&mut self) {
        let internal_rx = self.internal_rx.get_mut();
        internal_rx.close();
        while let Some(mut req) = internal_rx.next().await {
            let _ = match req.tx() {
                Some(tx) => tx.send(Err(RequestError)),
                None => Ok(()), // I think this is probably the best thing to do in this case, the other option is panicking
            };
        }
        self.svc.shutdown().await;
    }
}
