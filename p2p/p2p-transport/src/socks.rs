//! Socks Transport
//!
//! This module defines a transport method for the `ClearNet` network zone using a generic SOCKS5 proxy.
//!

//---------------------------------------------------------------------------------------------------- Imports

use std::{
    io::{self, ErrorKind},
    net::SocketAddr,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    time::{timeout, Duration},
};
use tokio_socks::tcp::Socks5Stream;
use tokio_util::codec::{FramedRead, FramedWrite};

use cuprate_p2p_core::{ClearNet, NetworkZone, Transport};
use cuprate_wire::MoneroWireCodec;

use crate::DisabledListener;

/// Check if a Socks5 proxy is listening at `addr` via a protocol handshake.
pub async fn is_socks5_proxy(addr: SocketAddr) -> bool {
    let Ok(Ok(mut stream)) = timeout(Duration::from_secs(3), TcpStream::connect(addr)).await else {
        return false;
    };

    // Socks5 greeting
    if stream.write_all(&[0x05, 0x01, 0x00]).await.is_err() {
        return false;
    }

    let mut buf = [0u8; 2];
    stream.read_exact(&mut buf).await.is_ok() && buf[0] == 0x05
}

//---------------------------------------------------------------------------------------------------- Configuration

/// Socks5 proxied TCP transport.
#[derive(Debug, Clone, Copy, Default)]
pub struct Socks;

#[derive(Clone)]
pub struct SocksClientConfig {
    /// Proxy address
    pub proxy: SocketAddr,

    /// According to RFC 1929, if authentication is enabled, both username and password fields MUST NOT be empty.
    pub authentication: Option<(String, String)>,
}

//---------------------------------------------------------------------------------------------------- Transport

#[async_trait::async_trait]
impl Transport<ClearNet> for Socks {
    type ClientConfig = SocksClientConfig;
    type ServerConfig = ();

    type Stream = FramedRead<OwnedReadHalf, MoneroWireCodec>;
    type Sink = FramedWrite<OwnedWriteHalf, MoneroWireCodec>;
    type Listener = DisabledListener<ClearNet, OwnedReadHalf, OwnedWriteHalf>;

    async fn connect_to_peer(
        addr: <ClearNet as NetworkZone>::Addr,
        config: &Self::ClientConfig,
    ) -> Result<(Self::Stream, Self::Sink), io::Error> {
        // Optional authentication
        let proxy = if let Some((username, password)) = config.authentication.as_ref() {
            Socks5Stream::connect_with_password(config.proxy, addr, username, password).await
        } else {
            Socks5Stream::connect(config.proxy, addr.to_string()).await
        };

        proxy
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
        _config: Self::ServerConfig,
    ) -> Result<Self::Listener, io::Error> {
        panic!("In proxy mode, inbound is disabled!");
    }
}
