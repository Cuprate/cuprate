use std::{collections::VecDeque, marker::PhantomData};
use std::pin::Pin;
use std::task::Poll;

use bytes::{Buf, BytesMut, Bytes};
use futures::ready;
use futures::sink::Sink;
use futures::AsyncWrite;
use pin_project::pin_project;

use crate::header::Flags;
use crate::{Bucket, BucketError, BucketHead};

#[pin_project]
pub struct BucketSink<W: AsyncWrite + std::marker::Unpin> {
    #[pin]
    writer: W,
    buffer: VecDeque<BytesMut>,
}

impl<W: AsyncWrite + std::marker::Unpin> Sink<Bucket> for BucketSink<W> {
    type Error = BucketError;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, item: Bucket) -> Result<(), Self::Error> {
        let buf = item.to_bytes();
        self.buffer.push_back(BytesMut::from(&buf[..]));
        Ok(())
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        let mut w = this.writer;
        let buffer = this.buffer;

        loop {
            match ready!(w.as_mut().poll_flush(cx)) {
                Err(err) => return Poll::Ready(Err(err.into())),
                Ok(()) => {
                    if let Some(buf) = buffer.front() {
                        match ready!(w.as_mut().poll_write(cx, buf)) {
                            Err(e) => match e.kind() {
                                std::io::ErrorKind::WouldBlock => return std::task::Poll::Pending,
                                _ => return Poll::Ready(Err(e.into())),
                            },
                            Ok(len) => {
                                if len == buffer[0].len() {
                                    buffer.pop_front();
                                } else {
                                    buffer[0].advance(len);
                                }
                            }
                        }
                    } else {
                        return Poll::Ready(Ok(()));
                    }
                }
            }
        }
    }

    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        ready!(self.project().writer.poll_close(cx))?;
        Poll::Ready(Ok(()))
    }
}

pub trait Encode {
    /// Encodes the message 
    /// 
    /// returns: 
    ///     return_code: i32,
    ///     command: u32,
    ///     have_to_return: bool,
    ///     flag: Flags - must only be Request or Response
    ///     bytes: Bytes
    fn encode(&self) -> Result<(i32, u32, bool, Flags, Bytes), BucketError>;
}

#[pin_project]
pub struct MessageSink<W: AsyncWrite + std::marker::Unpin, E: Encode> {
    #[pin]
    bucket_sink: BucketSink<W>,
    phantom: PhantomData<E>
}

impl<W: AsyncWrite + std::marker::Unpin, E: Encode> Sink<E> for MessageSink<W, E>{
    type Error = BucketError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().bucket_sink.poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: E) -> Result<(), Self::Error> {
        let (return_code, command, have_to_return_data, flags, body) = item.encode()?;
        let header = BucketHead::build(body.len() as u64, have_to_return_data, command, flags, return_code);

        let bucket = Bucket{header, body};

        self.project().bucket_sink.start_send(bucket)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().bucket_sink.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().bucket_sink.poll_close(cx)
    }
}