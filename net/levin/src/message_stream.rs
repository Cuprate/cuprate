//! This contains
//!

use std::marker::PhantomData;
use std::task::Poll;

use futures::ready;
use futures::AsyncRead;
use futures::Stream;
use pin_project::pin_project;

use crate::bucket_stream::BucketStream;
use crate::BucketError;
use crate::LevinBody;
use crate::LEVIN_SIGNATURE;
use crate::PROTOCOL_VERSION;

/// A stream that reads from the underlying `BucketStream` and uses the the
/// methods on the `LevinBody` trait to decode the inner messages(bodies)
#[pin_project]
pub struct MessageStream<D: LevinBody, S: AsyncRead + std::marker::Unpin> {
    #[pin]
    bucket_stream: BucketStream<S>,
    phantom: PhantomData<D>,
}

impl<D: LevinBody, S: AsyncRead + std::marker::Unpin> MessageStream<D, S> {
    /// Creates a new stream from the provided `AsyncRead`
    pub fn new(stream: S) -> Self {
        MessageStream {
            bucket_stream: BucketStream::new(stream),
            phantom: PhantomData,
        }
    }
}

impl<D: LevinBody, S: AsyncRead + std::marker::Unpin> Stream for MessageStream<D, S> {
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
                    bucket.header.flags.try_into()?,
                    bucket.header.have_to_return_data,
                    bucket.header.command,
                )))
            }
        }
    }
}
