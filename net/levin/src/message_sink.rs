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

//! This module provides a `MessageSink` struct, which serializes user defined messages
//! into levin `Bucket`s and passes them onto the `BucketSink`

use std::marker::PhantomData;
use std::pin::Pin;
use std::task::Poll;

use futures::AsyncWrite;
use futures::Sink;
use pin_project::pin_project;

use crate::bucket_sink::BucketSink;
use crate::Bucket;
use crate::BucketError;
use crate::BucketHead;
use crate::LevinBody;

/// A Sink that converts levin messages to buckets and passes them onto the `BucketSink`
#[pin_project]
pub struct MessageSink<W: AsyncWrite + std::marker::Unpin, E: LevinBody> {
    #[pin]
    bucket_sink: BucketSink<W>,
    phantom: PhantomData<E>,
}

impl<W: AsyncWrite + std::marker::Unpin, E: LevinBody> Sink<E> for MessageSink<W, E> {
    type Error = BucketError;

    fn poll_ready(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.project().bucket_sink.poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: E) -> Result<(), Self::Error> {
        let (return_code, command, have_to_return_data, flags, body) = item.encode()?;
        let header = BucketHead::build(
            body.len() as u64,
            have_to_return_data,
            command,
            flags.into(),
            return_code,
        );

        let bucket = Bucket { header, body };

        self.project().bucket_sink.start_send(bucket)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.project().bucket_sink.poll_flush(cx)
    }

    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.project().bucket_sink.poll_close(cx)
    }
}
