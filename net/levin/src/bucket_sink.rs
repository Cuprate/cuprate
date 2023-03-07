//! This module provides a `BucketSink` struct, which writes buckets to the
//! provided `AsyncWrite`. If you are a user of this library you should
//! probably use `MessageSink` instead.

use std::collections::VecDeque;
use std::pin::Pin;
use std::task::Poll;

use bytes::{Buf, BytesMut};
use futures::ready;
use futures::sink::Sink;
use futures::AsyncWrite;
use pin_project::pin_project;

use crate::{Bucket, BucketError};

/// A BucketSink writes Bucket instances to the provided AsyncWrite target.
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
