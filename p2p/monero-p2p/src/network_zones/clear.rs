use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    pin::Pin,
    task::{Context, Poll},
};

use futures::Stream;
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpListener, TcpStream,
};
use tokio_util::codec::{FramedRead, FramedWrite};

use monero_wire::MoneroWireCodec;

use crate::{NetZoneAddress, NetworkZone};

impl NetZoneAddress for SocketAddr {
    type BanID = IpAddr;

    fn set_port(&mut self, port: u16) {
        SocketAddr::set_port(self, port)
    }

    fn ban_id(&self) -> Self::BanID {
        self.ip()
    }

    fn make_canonical(&mut self) {
        let ip = self.ip().to_canonical();
        self.set_ip(ip);
    }

    fn should_add_to_peer_list(&self) -> bool {
        // TODO
        true
    }
}

pub struct ClearNetServerCfg {
    pub addr: SocketAddr,
}

#[derive(Clone, Copy)]
pub enum ClearNet {}

const fn ip_v4(a: u8, b: u8, c: u8, d: u8, port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(a, b, c, d)), port)
}

#[async_trait::async_trait]
impl NetworkZone for ClearNet {
    const NAME: &'static str = "ClearNet";

    const SEEDS: &'static [Self::Addr] = &[
        ip_v4(37, 187, 74, 171, 18080),
        ip_v4(192, 99, 8, 110, 18080),
    ];

    const ALLOW_SYNC: bool = true;
    const DANDELION_PP: bool = true;
    const CHECK_NODE_ID: bool = true;

    type Addr = SocketAddr;
    type Stream = FramedRead<OwnedReadHalf, MoneroWireCodec>;
    type Sink = FramedWrite<OwnedWriteHalf, MoneroWireCodec>;
    type Listener = InBoundStream;

    type ServerCfg = ClearNetServerCfg;

    async fn connect_to_peer(
        addr: Self::Addr,
    ) -> Result<(Self::Stream, Self::Sink), std::io::Error> {
        let (read, write) = TcpStream::connect(addr).await?.into_split();
        Ok((
            FramedRead::new(read, MoneroWireCodec::default()),
            FramedWrite::new(write, MoneroWireCodec::default()),
        ))
    }

    async fn incoming_connection_listener(
        config: Self::ServerCfg,
    ) -> Result<Self::Listener, std::io::Error> {
        let listener = TcpListener::bind(config.addr).await?;
        Ok(InBoundStream { listener })
    }
}

pub struct InBoundStream {
    listener: TcpListener,
}

impl Stream for InBoundStream {
    type Item = Result<
        (
            Option<SocketAddr>,
            FramedRead<OwnedReadHalf, MoneroWireCodec>,
            FramedWrite<OwnedWriteHalf, MoneroWireCodec>,
        ),
        std::io::Error,
    >;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.listener
            .poll_accept(cx)
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
