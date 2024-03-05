use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::task::{Context, Poll};

use monero_wire::MoneroWireCodec;

use futures::Stream;
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpListener, TcpStream,
};
use tokio_util::codec::{FramedRead, FramedWrite};

use crate::{NetZoneAddress, NetworkZone};

impl NetZoneAddress for SocketAddr {
    type BanID = IpAddr;

    fn ban_id(&self) -> Self::BanID {
        self.ip()
    }

    fn should_add_to_peer_list(&self) -> bool {
        todo!()
    }
}

pub struct ClearNetServerCfg {
    pub addr: SocketAddr,
}

#[derive(Clone, Copy)]
pub enum ClearNet {}

#[async_trait::async_trait]
impl NetworkZone for ClearNet {
    const NAME: &'static str = "ClearNet";
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
            .map_ok(|(stream, addr)| {
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
