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

//! A tokio-codec for levin buckets

use std::io::ErrorKind;
use std::marker::PhantomData;

use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use crate::{
    Bucket, BucketBuilder, BucketError, BucketHead, LevinBody, MessageType,
    LEVIN_DEFAULT_MAX_PACKET_SIZE,
};

/// The levin tokio-codec for decoding and encoding levin buckets
pub enum LevinCodec {
    /// Waiting for the peer to send a header.
    WaitingForHeader,
    /// Waiting for a peer to send a body.
    WaitingForBody(BucketHead),
}

impl Decoder for LevinCodec {
    type Item = Bucket;
    type Error = BucketError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            match self {
                LevinCodec::WaitingForHeader => {
                    if src.len() < BucketHead::SIZE {
                        return Ok(None);
                    };

                    let head = BucketHead::from_bytes(src)?;
                    let _ = std::mem::replace(self, LevinCodec::WaitingForBody(head));
                }
                LevinCodec::WaitingForBody(head) => {
                    // We size check header while decoding it.
                    let body_len = head.size.try_into().unwrap();
                    if src.len() < body_len {
                        src.reserve(body_len - src.len());
                        return Ok(None);
                    }

                    let LevinCodec::WaitingForBody(header) = std::mem::replace(self, LevinCodec::WaitingForHeader) else {
                        unreachable!()
                    };

                    return Ok(Some(Bucket {
                        header,
                        body: src.copy_to_bytes(body_len).into(),
                    }));
                }
            }
        }
    }
}

impl Encoder<Bucket> for LevinCodec {
    type Error = BucketError;
    fn encode(&mut self, item: Bucket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        if dst.capacity() < BucketHead::SIZE + item.body.len() {
            return Err(BucketError::IO(std::io::Error::new(
                ErrorKind::OutOfMemory,
                "Not enough capacity to write the bucket",
            )));
        }
        item.header.write_bytes(dst);
        dst.put_slice(&item.body);
        Ok(())
    }
}

enum MessageState {
    WaitingForBucket,
    WaitingForRestOfFragment(Vec<u8>, MessageType, u32),
}

/// A tokio-codec for levin messages or in other words the decoded body
/// of a levin bucket.
pub struct LevinMessageCodec<T> {
    message_ty: PhantomData<T>,
    bucket_codec: LevinCodec,
    state: MessageState,
}

impl<T: LevinBody> Decoder for LevinMessageCodec<T> {
    type Item = T;
    type Error = BucketError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            match &mut self.state {
                MessageState::WaitingForBucket => {
                    let Some(bucket) = self.bucket_codec.decode(src)? else {
                        return Ok(None);
                    };

                    let end_fragment = bucket.header.flags.end_fragment;
                    let start_fragment = bucket.header.flags.start_fragment;
                    let request = bucket.header.flags.request;
                    let response = bucket.header.flags.response;

                    if start_fragment && end_fragment {
                        // Dummy message
                        return Ok(None);
                    };

                    if end_fragment {
                        return Err(BucketError::InvalidHeaderFlags(
                            "Flag end fragment received before a start fragment",
                        ));
                    };

                    if !request && !response {
                        return Err(BucketError::InvalidHeaderFlags(
                            "Request and response flags both not set",
                        ));
                    };

                    let message_type = MessageType::from_flags_and_have_to_return(
                        bucket.header.flags,
                        bucket.header.have_to_return_data,
                    )?;

                    if start_fragment {
                        let _ = std::mem::replace(
                            &mut self.state,
                            MessageState::WaitingForRestOfFragment(
                                bucket.body.to_vec(),
                                message_type,
                                bucket.header.protocol_version,
                            ),
                        );

                        continue;
                    }

                    return Ok(Some(T::decode_message(
                        &bucket.body,
                        message_type,
                        bucket.header.command,
                    )?));
                }
                MessageState::WaitingForRestOfFragment(bytes, ty, command) => {
                    let Some(bucket) = self.bucket_codec.decode(src)? else {
                        return Ok(None);
                    };

                    let end_fragment = bucket.header.flags.end_fragment;
                    let start_fragment = bucket.header.flags.start_fragment;
                    let request = bucket.header.flags.request;
                    let response = bucket.header.flags.response;

                    if start_fragment && end_fragment {
                        // Dummy message
                        return Ok(None);
                    };

                    if !request && !response {
                        return Err(BucketError::InvalidHeaderFlags(
                            "Request and response flags both not set",
                        ));
                    };

                    let message_type = MessageType::from_flags_and_have_to_return(
                        bucket.header.flags,
                        bucket.header.have_to_return_data,
                    )?;

                    if message_type != *ty {
                        return Err(BucketError::InvalidFragmentedMessage(
                            "Message type was inconsistent across fragments",
                        ));
                    }

                    if bucket.header.command != *command {
                        return Err(BucketError::InvalidFragmentedMessage(
                            "Command not consistent across message",
                        ));
                    }

                    if bytes.len() + bucket.body.len()
                        > LEVIN_DEFAULT_MAX_PACKET_SIZE.try_into().unwrap()
                    {
                        return Err(BucketError::InvalidFragmentedMessage(
                            "Fragmented message exceeded maximum size",
                        ));
                    }

                    bytes.append(&mut bucket.body.to_vec());

                    if end_fragment {
                        let MessageState::WaitingForRestOfFragment(bytes, ty, command) =
                            std::mem::replace(&mut self.state, MessageState::WaitingForBucket) else {
                            unreachable!();
                        };

                        return Ok(Some(T::decode_message(&bytes, ty, command)?));
                    }
                }
            }
        }
    }
}

impl<T: LevinBody> Encoder<T> for LevinMessageCodec<T> {
    type Error = BucketError;
    fn encode(&mut self, item: T, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut bucket_builder = BucketBuilder::default();
        item.encode(&mut bucket_builder)?;
        let bucket = bucket_builder.finish();
        self.bucket_codec.encode(bucket, dst)
    }
}
