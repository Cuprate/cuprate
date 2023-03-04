//! This contains 
//! 

use std::marker::PhantomData;
use std::task::Poll;

use futures::AsyncRead;
use futures::Stream;
use futures::ready;
use pin_project::pin_project;

use crate::BucketError;
use crate::LEVIN_SIGNATURE;
use crate::LevinBody;
use crate::PROTOCOL_VERSION;
use crate::bucket_stream::BucketStream;
use crate::header::Flags;


#[pin_project]
pub struct MessageStream<D: LevinBody, S: AsyncRead + std::marker::Unpin> {
    #[pin]
    bucket_stream: BucketStream<S>,
    phantom: PhantomData<D>,
}

impl<D: LevinBody, S: AsyncRead + std::marker::Unpin> MessageStream<D, S> {
    pub fn new(stream: S) -> Self {
        MessageStream { 
            bucket_stream: BucketStream::new(stream), 
            phantom: PhantomData
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
            Err(e) => Poll::Ready(Some(Err(e.into()))),
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
                    || (bucket.header.return_code == 0 && bucket.header.flags == Flags::RESPONSE)
                {
                    return Err(BucketError::Error(bucket.header.return_code))?;
                }


                Poll::Ready(Some(D::decode_message(
                    &bucket.body,
                    bucket.header.flags,
                    bucket.header.have_to_return_data,
                    bucket.header.command,
                )))
            }
        }
    }
}
