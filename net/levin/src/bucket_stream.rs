// Rust Levin Library
// Written in 2023 by
//   Cuprate Contributors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//

//! This module provides a `BucketStream` struct, which is a stream of `Bucket`s,
//! where only the header is decoded. If you are a user of this library you should
//! probably use `MessageStream` instead.

use std::task::Poll;

use bytes::{Buf, BytesMut};
use futures::stream::Stream;
use futures::{ready, AsyncRead};
use pin_project::pin_project;

use super::{Bucket, BucketError, BucketHead};

/// An enum representing the decoding state of a `BucketStream`.
#[derive(Debug, Clone)]
enum BucketDecoder {
    /// Waiting for the header of a `Bucket`.
    WaitingForHeader,
    /// Waiting for the body of a `Bucket` with the given header.
    WaitingForBody(BucketHead),
}

impl BucketDecoder {
    /// Returns the number of bytes needed to complete the current decoding state.
    pub fn bytes_needed(&self) -> usize {
        match self {
            Self::WaitingForHeader => BucketHead::SIZE,
            Self::WaitingForBody(bucket_head) => bucket_head.size as usize,
        }
    }

    /// Tries to decode a `Bucket` from the given buffer, returning the decoded `Bucket` and the
    /// number of bytes consumed from the buffer.
    pub fn try_decode_bucket(
        &mut self,
        mut buf: &[u8],
    ) -> Result<(Option<Bucket>, usize), BucketError> {
        let mut len = 0;

        // first we decode header
        if let BucketDecoder::WaitingForHeader = self {
            if buf.len() < BucketHead::SIZE {
                return Ok((None, 0));
            }
            let header = BucketHead::from_bytes(&mut buf)?;
            len += BucketHead::SIZE;
            *self = BucketDecoder::WaitingForBody(header);
        };

        // next we check we have enough bytes to fill the body
        if let &mut Self::WaitingForBody(head) = self {
            if buf.len() < head.size as usize {
                return Ok((None, len));
            }
            *self = BucketDecoder::WaitingForHeader;
            Ok((
                Some(Bucket {
                    header: head,
                    body: buf.copy_to_bytes(buf.len()),
                }),
                len + head.size as usize,
            ))
        } else {
            unreachable!()
        }
    }
}

/// A stream of `Bucket`s, with only the header decoded.
#[pin_project]
#[derive(Debug, Clone)]
pub struct BucketStream<S> {
    #[pin]
    stream: S,
    decoder: BucketDecoder,
    buffer: BytesMut,
}

impl<S: AsyncRead> BucketStream<S> {
    /// Creates a new `BucketStream` from the given `AsyncRead` stream.
    pub fn new(stream: S) -> Self {
        BucketStream {
            stream,
            decoder: BucketDecoder::WaitingForHeader,
            buffer: BytesMut::with_capacity(1024),
        }
    }
}

impl<S: AsyncRead + std::marker::Unpin> Stream for BucketStream<S> {
    type Item = Result<Bucket, BucketError>;

    /// Attempt to read from the underlying stream into the buffer until enough bytes are received to construct a `Bucket`.
    ///
    /// If enough bytes are received, return the decoded `Bucket`, if not enough bytes are received to construct a `Bucket`,
    /// return `Poll::Pending`. This will never return `Poll::Ready(None)`.
    ///
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();
        let mut stream = this.stream;
        let decoder = this.decoder;
        let buffer = this.buffer;

        loop {
            // this is a bit ugly but all we are doing is calculating the amount of bytes we
            // need to build the rest of a bucket if this is zero it means we need to start
            // reading a new bucket
            let mut bytes_needed = buffer.len().saturating_sub(decoder.bytes_needed());
            if bytes_needed == 0 {
                bytes_needed = 1024
            }

            let mut buf = vec![0; bytes_needed];
            match ready!(stream.as_mut().poll_read(cx, &mut buf)) {
                Err(e) => match e.kind() {
                    std::io::ErrorKind::WouldBlock => return std::task::Poll::Pending,
                    std::io::ErrorKind::Interrupted => continue,
                    _ => return Poll::Ready(Some(Err(BucketError::IO(e)))),
                },
                Ok(len) => {
                    buffer.extend(&buf[..len]);

                    let (bucket, len) = decoder.try_decode_bucket(buffer)?;
                    buffer.advance(len);
                    if let Some(bucket) = bucket {
                        return Poll::Ready(Some(Ok(bucket)));
                    } else {
                        continue;
                    }
                }
            }
        }
    }
}
