//! TCP Transport
//!
//! This module defines the default transport method used by `monerod`: TCP Sockets.
//!
//! Since TCP Sockets can only connect to IP addresses, it's implementation is constrained
//! to `Z: NetworkZone<Addr = SocketAddr>`
//!

use std::{
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use futures::Stream;
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpListener, TcpStream,
};
use tokio_util::codec::{FramedRead, FramedWrite};

use cuprate_wire::MoneroWireCodec;

use crate::{NetworkZone, Transport};

/// Classic, TCP Socket based default transport.
#[derive(Debug, Clone, Copy, Default)]
pub struct Tcp;

#[derive(Debug, Clone)]
/// Mandatory parameters for starting the TCP p2p inbound listener
pub struct TcpServerConfig {
    /// Listening IPv4 Address.
    pub ipv4: Option<Ipv4Addr>,
    /// Listening IPv6 Address.
    pub ipv6: Option<Ipv6Addr>,

    /// Listening IPv4 Port.
    pub port: u16,

    /// Number of milliseconds before timeout at TCP writing
    send_timeout: Duration,
}

impl Default for TcpServerConfig {
    fn default() -> Self {
        Self {
            ipv4: Some(Ipv4Addr::UNSPECIFIED),
            ipv6: None,
            port: 18081,
            send_timeout: Duration::from_secs(20),
        }
    }
}

/// A set of listener to which new peers can connect to
pub struct TcpInBoundStream {
    /// IPv4 TCP listener
    listener_v4: Option<TcpListener>,
    /// IPv6 TCP listener
    listener_v6: Option<TcpListener>,
    /// Send Timeout
    _send_timeout: Duration,
}

impl Stream for TcpInBoundStream {
    type Item = Result<
        (
            Option<SocketAddr>,
            FramedRead<OwnedReadHalf, MoneroWireCodec>,
            FramedWrite<OwnedWriteHalf, MoneroWireCodec>,
        ),
        std::io::Error,
    >;

    /// SAFETY: Caller must ensure that at least one of the listener is `Some`, otherwise this function
    /// will always return `Poll::Pending`
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.listener_v4
            .as_ref()
            .and_then(|l| match l.poll_accept(cx) {
                Poll::Ready(r) => Some(Poll::Ready(r)),
                Poll::Pending => None,
            })
            .or(self
                .listener_v6
                .as_ref()
                .and_then(|l| match l.poll_accept(cx) {
                    Poll::Ready(r) => Some(Poll::Ready(r)),
                    Poll::Pending => None,
                }))
            .unwrap_or(Poll::Pending)
            .map_ok(|(stream, mut addr)| {
                let ip = addr.ip().to_canonical();
                addr.set_ip(ip);

                let (read, write) = stream.into_split();
                (
                    Some(addr),
                    FramedRead::new(read, MoneroWireCodec::default()),
                    FramedWrite::new(write, MoneroWireCodec::default()),
                )
            })
            .map(Some)
    }
}

#[async_trait::async_trait]
impl<Z: NetworkZone<Addr = SocketAddr>> Transport<Z> for Tcp {
    type ClientConfig = ();
    type ServerConfig = TcpServerConfig;

    type Stream = FramedRead<OwnedReadHalf, MoneroWireCodec>;
    type Sink = FramedWrite<OwnedWriteHalf, MoneroWireCodec>;
    type Listener = TcpInBoundStream;

    async fn connect_to_peer(
        addr: Z::Addr,
        _config: &Self::ClientConfig,
    ) -> Result<(Self::Stream, Self::Sink), std::io::Error> {
        let (read, write) = TcpStream::connect(addr).await?.into_split();
        Ok((
            FramedRead::new(read, MoneroWireCodec::default()),
            FramedWrite::new(write, MoneroWireCodec::default()),
        ))
    }

    async fn incoming_connection_listener(
        config: Self::ServerConfig,
    ) -> Result<Self::Listener, std::io::Error> {
        // Start up the IPv4/6 listeners
        let ipv4_listener = if let Some(ipv4) = config.ipv4 {
            Some(TcpListener::bind(SocketAddr::new(ipv4.into(), config.port)).await?)
        } else {
            None
        };

        // Linux INADDR_ANY bind all local interfaces regardless of the IP versioning.
        #[cfg(target_os = "linux")]
        if config.ipv4 == Some(Ipv4Addr::UNSPECIFIED) && config.ipv6 == Some(Ipv6Addr::UNSPECIFIED)
        {
            return Ok(TcpInBoundStream {
                listener_v4: ipv4_listener,
                listener_v6: None,
                _send_timeout: config.send_timeout,
            });
        }

        let ipv6_listener = if let Some(ipv6) = config.ipv6 {
            Some(TcpListener::bind(SocketAddr::new(ipv6.into(), config.port)).await?)
        } else {
            None
        };

        Ok(TcpInBoundStream {
            listener_v4: ipv4_listener,
            listener_v6: ipv6_listener,
            _send_timeout: config.send_timeout,
        })
    }
}
