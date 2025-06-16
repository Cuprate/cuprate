//! Tor Daemon Transport
//!
//! This module defines a transport method for the `Tor` network zone using an external Tor daemon supporting SOCKS5.
//!

//---------------------------------------------------------------------------------------------------- Imports

use std::{
    io::{self, ErrorKind},
    net::{IpAddr, SocketAddr},
    pin::Pin,
    task::{Context, Poll},
};

use async_trait::async_trait;
use futures::Stream;
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpListener,
};
use tokio_socks::tcp::Socks5Stream;
use tokio_util::codec::{FramedRead, FramedWrite};

use cuprate_p2p_core::{NetworkZone, Tor, Transport};
use cuprate_wire::MoneroWireCodec;

//---------------------------------------------------------------------------------------------------- Configuration

#[derive(Clone, Copy)]
pub struct DaemonClientConfig {
    /// Socket address of the external Tor daemon
    pub tor_daemon: SocketAddr,
}

#[derive(Clone, Copy)]
pub struct DaemonServerConfig {
    /// Listening IP Address.
    pub ip: IpAddr,

    /// Listening TCP Port.
    pub port: u16,
}

//---------------------------------------------------------------------------------------------------- Transport

/// A simple TCP server waiting for connections from the Tor daemon
pub struct DaemonInboundStream {
    listener: TcpListener,
}

impl Stream for DaemonInboundStream {
    type Item = Result<
        (
            Option<<Tor as NetworkZone>::Addr>,
            FramedRead<OwnedReadHalf, MoneroWireCodec>,
            FramedWrite<OwnedWriteHalf, MoneroWireCodec>,
        ),
        io::Error,
    >;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.listener
            .poll_accept(cx)
            .map_ok(|(stream, _)| {
                let (stream, sink) = stream.into_split();

                (
                    None, // Inbound is anonymous
                    FramedRead::new(stream, MoneroWireCodec::default()),
                    FramedWrite::new(sink, MoneroWireCodec::default()),
                )
            })
            .map(Some)
    }
}

#[derive(Clone, Copy)]
pub struct Daemon;

#[async_trait]
impl Transport<Tor> for Daemon {
    type ClientConfig = DaemonClientConfig;
    type ServerConfig = DaemonServerConfig;

    type Stream = FramedRead<OwnedReadHalf, MoneroWireCodec>;
    type Sink = FramedWrite<OwnedWriteHalf, MoneroWireCodec>;
    type Listener = DaemonInboundStream;

    async fn connect_to_peer(
        addr: <Tor as NetworkZone>::Addr,
        config: &Self::ClientConfig,
    ) -> Result<(Self::Stream, Self::Sink), io::Error> {
        Socks5Stream::connect(config.tor_daemon, addr.to_string())
            .await
            .map_err(|e| io::Error::new(ErrorKind::ConnectionAborted, e.to_string()))
            .map(|stream| {
                let (stream, sink) = stream.into_inner().into_split();
                (
                    FramedRead::new(stream, MoneroWireCodec::default()),
                    FramedWrite::new(sink, MoneroWireCodec::default()),
                )
            })
    }

    async fn incoming_connection_listener(
        config: Self::ServerConfig,
    ) -> Result<Self::Listener, io::Error> {
        let listener = TcpListener::bind((config.ip, config.port)).await?;

        Ok(DaemonInboundStream { listener })
    }
}
