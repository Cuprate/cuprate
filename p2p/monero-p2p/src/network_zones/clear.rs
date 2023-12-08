use std::net::{IpAddr, SocketAddr};

use monero_wire::MoneroWireCodec;

use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
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

#[derive(Clone, Copy)]
pub struct ClearNet;

pub struct ClearNetServerCfg {}

#[async_trait::async_trait]
impl NetworkZone for ClearNet {
    const NAME: &'static str = "ClearNet";
    const ALLOW_SYNC: bool = true;
    const DANDELION_PP: bool = true;
    const CHECK_NODE_ID: bool = true;

    type Addr = SocketAddr;
    type Stream = FramedRead<OwnedReadHalf, MoneroWireCodec>;
    type Sink = FramedWrite<OwnedWriteHalf, MoneroWireCodec>;

    type ServerCfg = ();

    async fn connect_to_peer(
        addr: Self::Addr,
    ) -> Result<(Self::Stream, Self::Sink), std::io::Error> {
        let (read, write) = TcpStream::connect(addr).await?.into_split();
        Ok((
            FramedRead::new(read, MoneroWireCodec::default()),
            FramedWrite::new(write, MoneroWireCodec::default()),
        ))
    }

    async fn incoming_connection_listener(config: Self::ServerCfg) -> () {
        todo!()
    }
}
