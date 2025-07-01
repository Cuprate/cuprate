//! Arti Transport
//!
//! This module defines a transport method for the `Tor` network zone using the `arti_client` library.
//!

//---------------------------------------------------------------------------------------------------- Imports

use std::{
    io::{self, ErrorKind},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use arti_client::{DataReader, DataWriter, TorClient, TorClientConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use tokio_util::codec::{FramedRead, FramedWrite};
use tor_cell::relaycell::msg::Connected;
use tor_config_path::CfgPathResolver;
use tor_hsservice::{handle_rend_requests, OnionService, RunningOnionService};
use tor_proto::stream::IncomingStreamRequest;
use tor_rtcompat::PreferredRuntime;

use cuprate_p2p_core::{ClearNet, NetworkZone, Tor, Transport};
use cuprate_wire::MoneroWireCodec;

use crate::DisabledListener;

//---------------------------------------------------------------------------------------------------- Configuration

#[derive(Clone)]
pub struct ArtiClientConfig {
    /// Arti bootstrapped client
    pub client: TorClient<PreferredRuntime>,
}

pub struct ArtiServerConfig {
    /// Arti onion service
    pub onion_svc: OnionService,
    /// Listening port
    pub port: u16,

    // Mandatory resources for launching the onion service
    client: TorClient<PreferredRuntime>,
    path_resolver: Arc<CfgPathResolver>,
}

impl ArtiServerConfig {
    pub fn new(
        onion_svc: OnionService,
        port: u16,
        client: &TorClient<PreferredRuntime>,
        config: &TorClientConfig,
    ) -> Self {
        let path_resolver: &CfgPathResolver = config.as_ref();

        Self {
            onion_svc,
            port,
            client: client.clone(),
            path_resolver: Arc::new(path_resolver.clone()),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Transport

type PinnedStream<I> = Pin<Box<dyn Stream<Item = I> + Send>>;

/// An onion service listening for incoming peer connections.
pub struct OnionListener {
    /// A handle to the onion service instance.
    _onion_svc: Arc<RunningOnionService>,
    /// A modified stream that produce a data stream and sink from rendez-vous requests.
    listener: PinnedStream<Result<(DataReader, DataWriter), io::Error>>,
}

impl Stream for OnionListener {
    type Item = Result<
        (
            Option<<Tor as NetworkZone>::Addr>,
            FramedRead<DataReader, MoneroWireCodec>,
            FramedWrite<DataWriter, MoneroWireCodec>,
        ),
        io::Error,
    >;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.listener.poll_next_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(req) => Poll::Ready(req.map(|r| {
                r.map(|(stream, sink)| {
                    (
                        None, // Inbound is anonymous
                        FramedRead::new(stream, MoneroWireCodec::default()),
                        FramedWrite::new(sink, MoneroWireCodec::default()),
                    )
                })
            })),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Arti;

#[async_trait]
impl Transport<Tor> for Arti {
    type ClientConfig = ArtiClientConfig;
    type ServerConfig = ArtiServerConfig;

    type Stream = FramedRead<DataReader, MoneroWireCodec>;
    type Sink = FramedWrite<DataWriter, MoneroWireCodec>;
    type Listener = OnionListener;

    async fn connect_to_peer(
        addr: <Tor as NetworkZone>::Addr,
        config: &Self::ClientConfig,
    ) -> Result<(Self::Stream, Self::Sink), io::Error> {
        config
            .client
            .connect((addr.addr_string(), addr.port()))
            .await
            .map_err(|e| io::Error::new(ErrorKind::ConnectionAborted, e.to_string()))
            .map(|stream| {
                let (stream, sink) = stream.split();
                (
                    FramedRead::new(stream, MoneroWireCodec::default()),
                    FramedWrite::new(sink, MoneroWireCodec::default()),
                )
            })
    }

    async fn incoming_connection_listener(
        config: Self::ServerConfig,
    ) -> Result<Self::Listener, io::Error> {
        // Launch onion service
        #[expect(clippy::clone_on_ref_ptr)]
        let (svc, rdv_stream) = config
            .onion_svc
            .launch(
                config.client.runtime().clone(),
                config.client.dirmgr().clone(),
                config.client.hs_circ_pool().clone(),
                config.path_resolver,
            )
            .unwrap();

        // Accept all rendez-vous and await correct stream request
        let req_stream = handle_rend_requests(rdv_stream).then(move |sreq| async move {
            match sreq.request() {
                // As specified in: <https://spec.torproject.org/rend-spec/managing-streams.html>
                //
                // A client that wishes to open a data stream with us needs to send a BEGIN message with an empty address
                // and no flags. We additionally filter requests to the correct port configured and advertised on P2P.
                IncomingStreamRequest::Begin(r)
                    if r.port() == config.port && r.addr().is_empty() && r.flags().is_empty() =>
                {
                    let stream = sreq
                        .accept(Connected::new_empty())
                        .await
                        .map_err(|e| io::Error::new(ErrorKind::BrokenPipe, e.to_string()))?;

                    Ok(stream.split())
                }
                req => {
                    let err = match req {
                        IncomingStreamRequest::BeginDir(_) => {
                            Err(io::Error::other("Received invalid command: BeginDir"))
                        }
                        IncomingStreamRequest::Resolve(_) => {
                            Err(io::Error::other("Received invalid command: Resolve"))
                        }
                        _ => unreachable!(),
                    };
                    sreq.shutdown_circuit()
                        .expect("Should never panic, unless programming error from arti's end.");
                    err
                }
            }
        });

        Ok(OnionListener {
            _onion_svc: svc,
            listener: Box::pin(req_stream),
        })
    }
}

#[async_trait]
impl Transport<ClearNet> for Arti {
    type ClientConfig = ArtiClientConfig;
    type ServerConfig = ();

    type Stream = FramedRead<DataReader, MoneroWireCodec>;
    type Sink = FramedWrite<DataWriter, MoneroWireCodec>;
    type Listener = DisabledListener<ClearNet, DataReader, DataWriter>;

    async fn connect_to_peer(
        addr: <ClearNet as NetworkZone>::Addr,
        config: &Self::ClientConfig,
    ) -> Result<(Self::Stream, Self::Sink), io::Error> {
        config
            .client
            .connect(addr.to_string())
            .await
            .map_err(|e| io::Error::new(ErrorKind::ConnectionAborted, e.to_string()))
            .map(|stream| {
                let (stream, sink) = stream.split();
                (
                    FramedRead::new(stream, MoneroWireCodec::default()),
                    FramedWrite::new(sink, MoneroWireCodec::default()),
                )
            })
    }

    async fn incoming_connection_listener(
        _config: Self::ServerConfig,
    ) -> Result<Self::Listener, io::Error> {
        panic!("In anonymized clearnet mode, inbound is disabled!");
    }
}
