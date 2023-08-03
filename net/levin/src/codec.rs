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
    Bucket, BucketBuilder, BucketError, BucketHead, LevinBody, LevinCommand, MessageType, Protocol,
};

#[derive(Debug, Clone)]
pub enum LevinBucketState<C> {
    /// Waiting for the peer to send a header.
    WaitingForHeader,
    /// Waiting for a peer to send a body.
    WaitingForBody(BucketHead<C>),
}

/// The levin tokio-codec for decoding and encoding raw levin buckets
///
#[derive(Debug, Clone)]
pub struct LevinBucketCodec<C> {
    state: LevinBucketState<C>,
    protocol: Protocol,
    handshake_message_seen: bool,
}

impl<C> Default for LevinBucketCodec<C> {
    fn default() -> Self {
        LevinBucketCodec {
            state: LevinBucketState::WaitingForHeader,
            protocol: Protocol::default(),
            handshake_message_seen: false,
        }
    }
}

impl<C> LevinBucketCodec<C> {
    pub fn new(protocol: Protocol) -> Self {
        LevinBucketCodec {
            state: LevinBucketState::WaitingForHeader,
            protocol,
            handshake_message_seen: false,
        }
    }
}

impl<C: LevinCommand> Decoder for LevinBucketCodec<C> {
    type Item = Bucket<C>;
    type Error = BucketError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            match &self.state {
                LevinBucketState::WaitingForHeader => {
                    if src.len() < BucketHead::<C>::SIZE {
                        return Ok(None);
                    };

                    let head = BucketHead::<C>::from_bytes(src);

                    if head.size > self.protocol.max_packet_size
                        || head.size > head.command.bucket_size_limit()
                    {
                        return Err(BucketError::BucketExceededMaxSize);
                    }

                    if !self.handshake_message_seen {
                        if head.size > self.protocol.max_packet_size_before_handshake {
                            return Err(BucketError::BucketExceededMaxSize);
                        }

                        if head.command.is_handshake() {
                            self.handshake_message_seen = true;
                        }
                    }

                    let _ =
                        std::mem::replace(&mut self.state, LevinBucketState::WaitingForBody(head));
                }
                LevinBucketState::WaitingForBody(head) => {
                    let body_len = head
                        .size
                        .try_into()
                        .map_err(|_| BucketError::BucketExceededMaxSize)?;
                    if src.len() < body_len {
                        src.reserve(body_len - src.len());
                        return Ok(None);
                    }

                    let LevinBucketState::WaitingForBody(header) = std::mem::replace(&mut self.state, LevinBucketState::WaitingForHeader) else {
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

impl<C: LevinCommand> Encoder<Bucket<C>> for LevinBucketCodec<C> {
    type Error = BucketError;
    fn encode(&mut self, item: Bucket<C>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        if dst.capacity() < BucketHead::<C>::SIZE + item.body.len() {
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

#[derive(Default, Debug, Clone)]
enum MessageState<C> {
    #[default]
    WaitingForBucket,
    WaitingForRestOfFragment(Vec<u8>, MessageType, C),
}

/// A tokio-codec for levin messages or in other words the decoded body
/// of a levin bucket.
#[derive(Debug, Clone)]
pub struct LevinMessageCodec<T: LevinBody> {
    message_ty: PhantomData<T>,
    bucket_codec: LevinBucketCodec<T::Command>,
    state: MessageState<T::Command>,
}

impl<T: LevinBody> Default for LevinMessageCodec<T> {
    fn default() -> Self {
        Self {
            message_ty: Default::default(),
            bucket_codec: Default::default(),
            state: Default::default(),
        }
    }
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

                    let flags = &bucket.header.flags;

                    if flags.is_start_fragment() && flags.is_end_fragment() {
                        // Dummy message
                        return Ok(None);
                    };

                    if flags.is_end_fragment() {
                        return Err(BucketError::InvalidHeaderFlags(
                            "Flag end fragment received before a start fragment",
                        ));
                    };

                    if !flags.is_request() && !flags.is_response() {
                        return Err(BucketError::InvalidHeaderFlags(
                            "Request and response flags both not set",
                        ));
                    };

                    let message_type = MessageType::from_flags_and_have_to_return(
                        bucket.header.flags,
                        bucket.header.have_to_return_data,
                    )?;

                    if flags.is_start_fragment() {
                        let _ = std::mem::replace(
                            &mut self.state,
                            MessageState::WaitingForRestOfFragment(
                                bucket.body.to_vec(),
                                message_type,
                                bucket.header.command,
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

                    let flags = &bucket.header.flags;

                    if flags.is_start_fragment() && flags.is_end_fragment() {
                        // Dummy message
                        return Ok(None);
                    };

                    if !flags.is_request() && !flags.is_response() {
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
                            "Command not consistent across fragments",
                        ));
                    }

                    if bytes.len().saturating_add(bucket.body.len())
                        > command.bucket_size_limit().try_into().unwrap()
                    {
                        return Err(BucketError::InvalidFragmentedMessage(
                            "Fragmented message exceeded maximum size",
                        ));
                    }

                    bytes.append(&mut bucket.body.to_vec());

                    if flags.is_end_fragment() {
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
