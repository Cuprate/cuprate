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

//! This modual provides a `MessageStream` which deserializes partially decoded `Bucket`s
//! into full Buckets using a user provided `LevinBody`

use std::marker::PhantomData;
use std::task::Poll;

use futures::ready;
use futures::AsyncRead;
use futures::Stream;
use pin_project::pin_project;

use crate::bucket_stream::BucketStream;
use crate::BucketError;
use crate::LevinBody;
use crate::MessageType;
use crate::LEVIN_SIGNATURE;
use crate::PROTOCOL_VERSION;

/// A stream that reads from the underlying `BucketStream` and uses the the
/// methods on the `LevinBody` trait to decode the inner messages(bodies)
#[pin_project]
pub struct MessageStream<S, D> {
    #[pin]
    bucket_stream: BucketStream<S>,
    phantom: PhantomData<D>,
}

impl<D: LevinBody, S: AsyncRead + std::marker::Unpin> MessageStream<S, D> {
    /// Creates a new stream from the provided `AsyncRead`
    pub fn new(stream: S) -> Self {
        MessageStream {
            bucket_stream: BucketStream::new(stream),
            phantom: PhantomData,
        }
    }
}

impl<D: LevinBody, S: AsyncRead + std::marker::Unpin> Stream for MessageStream<S, D> {
    type Item = Result<D, BucketError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match ready!(this.bucket_stream.poll_next(cx)).expect("BucketStream will never return None")
        {
            Err(e) => Poll::Ready(Some(Err(e))),
            Ok(bucket) => {
                if bucket.header.signature != LEVIN_SIGNATURE {
                    return Err(BucketError::IncorrectSignature(bucket.header.signature))?;
                }

                if bucket.header.protocol_version != PROTOCOL_VERSION {
                    return Err(BucketError::UnknownProtocolVersion(
                        bucket.header.protocol_version,
                    ))?;
                }

                // TODO: we shouldn't return an error if the peer sends an error response we should define a new network
                // message: Error.
                if bucket.header.return_code < 0
                    || (bucket.header.return_code == 0 && bucket.header.flags.is_response())
                {
                    return Err(BucketError::Error(bucket.header.return_code))?;
                }

                if bucket.header.flags.is_dummy() {
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }

                Poll::Ready(Some(D::decode_message(
                    &bucket.body,
                    MessageType::from_flags_and_have_to_return(
                        bucket.header.flags,
                        bucket.header.have_to_return_data,
                    )?,
                    bucket.header.command,
                )))
            }
        }
    }
}
