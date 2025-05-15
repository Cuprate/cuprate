//! Dummy Transport
//!
//! This module defines a dummy transport method with no generation logic.
//! It is only useful as a placeholder or performing logical p2p tests.
//!
//! It's implementation isn't constrained and can be used for any `Z: NetworkZone`
//!

use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use futures::Stream;
use tokio::io::{DuplexStream, ReadHalf, WriteHalf};
use tokio_util::codec::{FramedRead, FramedWrite};

use cuprate_wire::MoneroWireCodec;

use crate::{NetworkZone, Transport};

/// A dummy transport method with no generation logic. It is only useful as a placeholder or for tests.
#[derive(Clone)]
pub struct DummyTransport;

type DummyTransportStream = FramedRead<ReadHalf<DuplexStream>, MoneroWireCodec>;
type DummyTransportSink = FramedWrite<WriteHalf<DuplexStream>, MoneroWireCodec>;

pub struct DummyTransportListener<Z: NetworkZone> {
    _zone: PhantomData<Z>,
}

impl<Z: NetworkZone> Stream for DummyTransportListener<Z> {
    type Item = Result<(Option<Z::Addr>, DummyTransportStream, DummyTransportSink), std::io::Error>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        panic!("The dummy transport listener is never meant to be used.");
    }
}

#[async_trait::async_trait]
impl<Z: NetworkZone> Transport<Z> for DummyTransport {
    type ClientConfig = ();
    type ServerConfig = ();

    type Stream = DummyTransportStream;
    type Sink = DummyTransportSink;
    type Listener = DummyTransportListener<Z>;

    async fn connect_to_peer(
        _addr: Z::Addr,
        _config: &Self::ClientConfig,
    ) -> Result<(Self::Stream, Self::Sink), std::io::Error> {
        panic!("The dummy transport method is never meant to be used.");
    }

    async fn incoming_connection_listener(
        _config: Self::ServerConfig,
    ) -> Result<Self::Listener, std::io::Error> {
        panic!("The dummy transport method is never meant to be used.");
    }
}
