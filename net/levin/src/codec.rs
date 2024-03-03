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

use std::marker::PhantomData;

use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use crate::{
    header::{Flags, HEADER_SIZE},
    message::{make_dummy_message, LevinMessage},
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
                    if src.len() < HEADER_SIZE {
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

                    let LevinBucketState::WaitingForBody(header) =
                        std::mem::replace(&mut self.state, LevinBucketState::WaitingForHeader)
                    else {
                        unreachable!()
                    };

                    return Ok(Some(Bucket {
                        header,
                        body: src.copy_to_bytes(body_len),
                    }));
                }
            }
        }
    }
}

impl<C: LevinCommand> Encoder<Bucket<C>> for LevinBucketCodec<C> {
    type Error = BucketError;
    fn encode(&mut self, item: Bucket<C>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        if let Some(additional) = (HEADER_SIZE + item.body.len()).checked_sub(dst.capacity()) {
            dst.reserve(additional)
        }

        item.header.write_bytes_into(dst);
        dst.put_slice(&item.body);
        Ok(())
    }
}

#[derive(Default, Debug, Clone)]
enum MessageState {
    #[default]
    WaitingForBucket,
    /// Waiting for the rest of a fragmented message.
    ///
    /// We keep the fragmented message as a Vec<u8> instead of [`Bytes`](bytes::Bytes) as [`Bytes`](bytes::Bytes) could point to a
    /// large allocation even if the [`Bytes`](bytes::Bytes) itself is small, so is not safe to keep around for long.
    /// To prevent this attack vector completely we just use Vec<u8> for fragmented messages.
    WaitingForRestOfFragment(Vec<u8>),
}

/// A tokio-codec for levin messages or in other words the decoded body
/// of a levin bucket.
#[derive(Debug, Clone)]
pub struct LevinMessageCodec<T: LevinBody> {
    message_ty: PhantomData<T>,
    bucket_codec: LevinBucketCodec<T::Command>,
    state: MessageState,
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
                    let Some(mut bucket) = self.bucket_codec.decode(src)? else {
                        return Ok(None);
                    };

                    let flags = &bucket.header.flags;

                    if flags.contains(Flags::DUMMY) {
                        // Dummy message
                        return Ok(None);
                    };

                    if flags.contains(Flags::END_FRAGMENT) {
                        return Err(BucketError::InvalidHeaderFlags(
                            "Flag end fragment received before a start fragment",
                        ));
                    };

                    if flags.contains(Flags::START_FRAGMENT) {
                        // monerod does not require a start flag before starting a fragmented message,
                        // but will always produce one, so it is ok for us to require one.
                        self.state = MessageState::WaitingForRestOfFragment(bucket.body.to_vec());

                        continue;
                    }

                    // Normal, non fragmented bucket

                    let message_type = MessageType::from_flags_and_have_to_return(
                        bucket.header.flags,
                        bucket.header.have_to_return_data,
                    )?;

                    return Ok(Some(T::decode_message(
                        &mut bucket.body,
                        message_type,
                        bucket.header.command,
                    )?));
                }
                MessageState::WaitingForRestOfFragment(bytes) => {
                    let Some(bucket) = self.bucket_codec.decode(src)? else {
                        return Ok(None);
                    };

                    let flags = &bucket.header.flags;

                    if flags.contains(Flags::DUMMY) {
                        // Dummy message
                        return Ok(None);
                    };

                    let max_size = if self.bucket_codec.handshake_message_seen {
                        self.bucket_codec.protocol.max_packet_size
                    } else {
                        self.bucket_codec.protocol.max_packet_size_before_handshake
                    }
                    .try_into()
                    .expect("Levin max message size is too large, does not fit into a usize.");

                    if bytes.len().saturating_add(bucket.body.len()) > max_size {
                        return Err(BucketError::InvalidFragmentedMessage(
                            "Fragmented message exceeded maximum size",
                        ));
                    }

                    bytes.extend_from_slice(bucket.body.as_ref());

                    if flags.contains(Flags::END_FRAGMENT) {
                        let MessageState::WaitingForRestOfFragment(bytes) =
                            std::mem::replace(&mut self.state, MessageState::WaitingForBucket)
                        else {
                            unreachable!();
                        };

                        // Check there are enough bytes in the fragment to build a header.
                        if bytes.len() < HEADER_SIZE {
                            return Err(BucketError::InvalidFragmentedMessage(
                                "Fragmented message is not large enough to build a bucket.",
                            ));
                        }

                        let mut header_bytes = BytesMut::from(&bytes[0..HEADER_SIZE]);

                        let header = BucketHead::<T::Command>::from_bytes(&mut header_bytes);

                        if header.size > header.command.bucket_size_limit() {
                            return Err(BucketError::BucketExceededMaxSize);
                        }

                        // Check the fragmented message contains enough bytes to build the message.
                        if bytes.len().saturating_sub(HEADER_SIZE)
                            < header
                                .size
                                .try_into()
                                .map_err(|_| BucketError::BucketExceededMaxSize)?
                        {
                            return Err(BucketError::InvalidFragmentedMessage(
                                "Fragmented message does not have enough bytes to fill bucket body",
                            ));
                        }

                        let message_type = MessageType::from_flags_and_have_to_return(
                            header.flags,
                            header.have_to_return_data,
                        )?;

                        return Ok(Some(T::decode_message(
                            &mut &bytes[HEADER_SIZE..],
                            message_type,
                            header.command,
                        )?));
                    }
                }
            }
        }
    }
}

impl<T: LevinBody> Encoder<LevinMessage<T>> for LevinMessageCodec<T> {
    type Error = BucketError;
    fn encode(&mut self, item: LevinMessage<T>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            LevinMessage::Body(body) => {
                let mut bucket_builder = BucketBuilder::default();
                body.encode(&mut bucket_builder)?;
                let bucket = bucket_builder.finish();
                self.bucket_codec.encode(bucket, dst)
            }
            LevinMessage::Bucket(bucket) => self.bucket_codec.encode(bucket, dst),
            LevinMessage::Dummy(size) => {
                let bucket = make_dummy_message(&self.bucket_codec.protocol, size);
                self.bucket_codec.encode(bucket, dst)
            }
        }
    }
}
