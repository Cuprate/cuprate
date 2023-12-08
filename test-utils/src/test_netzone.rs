use std::{
    fmt::Formatter,
    io::Error,
    net::{Ipv4Addr, SocketAddr},
    pin::Pin,
    task::{Context, Poll},
};

use borsh::{BorshDeserialize, BorshSerialize};
use futures::{channel::mpsc::Sender as InnerSender, stream::BoxStream, Sink};

use monero_wire::{
    network_address::{NetworkAddress, NetworkAddressIncorrectZone},
    BucketError, Message,
};

use monero_p2p::{NetZoneAddress, NetworkZone};

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct TestNetZoneAddr(pub u32);

impl NetZoneAddress for TestNetZoneAddr {
    type BanID = Self;

    fn ban_id(&self) -> Self::BanID {
        *self
    }

    fn should_add_to_peer_list(&self) -> bool {
        true
    }
}

impl std::fmt::Display for TestNetZoneAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("test client, id: {}", self.0).as_str())
    }
}

impl From<TestNetZoneAddr> for NetworkAddress {
    fn from(value: TestNetZoneAddr) -> Self {
        NetworkAddress::Clear(SocketAddr::new(Ipv4Addr::from(value.0).into(), 18080))
    }
}

impl TryFrom<NetworkAddress> for TestNetZoneAddr {
    type Error = NetworkAddressIncorrectZone;

    fn try_from(value: NetworkAddress) -> Result<Self, Self::Error> {
        match value {
            NetworkAddress::Clear(soc) => match soc {
                SocketAddr::V4(v4) => Ok(TestNetZoneAddr(u32::from_be_bytes(v4.ip().octets()))),
                _ => panic!("None v4 address in test code"),
            },
        }
    }
}

pub struct Sender {
    inner: InnerSender<Message>,
}

impl From<InnerSender<Message>> for Sender {
    fn from(inner: InnerSender<Message>) -> Self {
        Sender { inner }
    }
}

impl Sink<Message> for Sender {
    type Error = BucketError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut()
            .inner
            .poll_ready(cx)
            .map_err(|_| BucketError::IO(std::io::Error::other("mock connection channel closed")))
    }

    fn start_send(self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        self.get_mut()
            .inner
            .start_send(item)
            .map_err(|_| BucketError::IO(std::io::Error::other("mock connection channel closed")))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().inner)
            .poll_flush(cx)
            .map_err(|_| BucketError::IO(std::io::Error::other("mock connection channel closed")))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().inner)
            .poll_close(cx)
            .map_err(|_| BucketError::IO(std::io::Error::other("mock connection channel closed")))
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TestNetZone<const ALLOW_SYNC: bool, const DANDELION_PP: bool, const CHECK_NODE_ID: bool>;

#[async_trait::async_trait]
impl<const ALLOW_SYNC: bool, const DANDELION_PP: bool, const CHECK_NODE_ID: bool> NetworkZone
    for TestNetZone<ALLOW_SYNC, DANDELION_PP, CHECK_NODE_ID>
{
    const NAME: &'static str = "Testing";
    const ALLOW_SYNC: bool = ALLOW_SYNC;
    const DANDELION_PP: bool = DANDELION_PP;
    const CHECK_NODE_ID: bool = CHECK_NODE_ID;

    type Addr = TestNetZoneAddr;
    type Stream = BoxStream<'static, Result<Message, BucketError>>;
    type Sink = Sender;
    type ServerCfg = ();

    async fn connect_to_peer(_: Self::Addr) -> Result<(Self::Stream, Self::Sink), Error> {
        unimplemented!()
    }

    async fn incoming_connection_listener(_: Self::ServerCfg) -> () {
        unimplemented!()
    }
}
