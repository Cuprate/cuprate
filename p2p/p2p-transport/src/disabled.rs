use std::{
    io,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use futures::Stream;
use tokio_util::codec::{FramedRead, FramedWrite};

use cuprate_p2p_core::NetworkZone;
use cuprate_wire::MoneroWireCodec;

/// In proxied clearnet mode, inbound is disabled.
pub struct DisabledListener<Z: NetworkZone, R, W> {
    _zone: PhantomData<Z>,
    _reader: PhantomData<R>,
    _writer: PhantomData<W>,
}

impl<Z: NetworkZone, R, W> Stream for DisabledListener<Z, R, W> {
    type Item = Result<
        (
            Option<Z::Addr>,
            FramedRead<R, MoneroWireCodec>,
            FramedWrite<W, MoneroWireCodec>,
        ),
        io::Error,
    >;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Panic within [`Transport::incoming_connection_listener`]
        unreachable!()
    }
}
