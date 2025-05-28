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

use std::{fmt::Debug, marker::PhantomData};

use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use cuprate_helper::cast::u64_to_usize;

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
        Self {
            state: LevinBucketState::WaitingForHeader,
            protocol: Protocol::default(),
            handshake_message_seen: false,
        }
    }
}

impl<C> LevinBucketCodec<C> {
    pub const fn new(protocol: Protocol) -> Self {
        Self {
            state: LevinBucketState::WaitingForHeader,
            protocol,
            handshake_message_seen: false,
        }
    }
}

impl<C: LevinCommand + Debug> Decoder for LevinBucketCodec<C> {
    type Item = Bucket<C>;
    type Error = BucketError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            match &self.state {
                LevinBucketState::WaitingForHeader => {
                    if src.len() < HEADER_SIZE {
                        return Ok(None);
                    }

                    let head = BucketHead::<C>::from_bytes(src);

                    #[cfg(feature = "tracing")]
                    tracing::trace!(
                        "Received new bucket header, command: {:?}, waiting for body, body len: {}",
                        head.command,
                        head.size
                    );

                    if head.size > self.protocol.max_packet_size
                        || head.size > head.command.bucket_size_limit()
                    {
                        #[cfg(feature = "tracing")]
                        tracing::debug!("Peer sent message which is too large.");

                        return Err(BucketError::BucketExceededMaxSize);
                    }

                    if !self.handshake_message_seen {
                        if head.size > self.protocol.max_packet_size_before_handshake {
                            #[cfg(feature = "tracing")]
                            tracing::debug!("Peer sent message which is too large.");

                            return Err(BucketError::BucketExceededMaxSize);
                        }

                        if head.command.is_handshake() {
                            #[cfg(feature = "tracing")]
                            tracing::debug!(
                                "Peer handshake message seen, increasing bucket size limit."
                            );

                            self.handshake_message_seen = true;
                        }
                    }

                    drop(std::mem::replace(
                        &mut self.state,
                        LevinBucketState::WaitingForBody(head),
                    ));
                }
                LevinBucketState::WaitingForBody(head) => {
                    let body_len = u64_to_usize(head.size);
                    if src.len() < body_len {
                        src.reserve(body_len - src.len());
                        return Ok(None);
                    }

                    let LevinBucketState::WaitingForBody(header) =
                        std::mem::replace(&mut self.state, LevinBucketState::WaitingForHeader)
                    else {
                        unreachable!()
                    };

                    #[cfg(feature = "tracing")]
                    tracing::trace!("Received full bucket for command: {:?}", header.command);

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
            dst.reserve(additional);
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

                        #[cfg(feature = "tracing")]
                        tracing::trace!("Received DUMMY bucket from peer, ignoring.");
                        // We may have another bucket in `src`.
                        continue;
                    }

                    if flags.contains(Flags::END_FRAGMENT) {
                        return Err(BucketError::InvalidHeaderFlags(
                            "Flag end fragment received before a start fragment",
                        ));
                    }

                    if flags.contains(Flags::START_FRAGMENT) {
                        // monerod does not require a start flag before starting a fragmented message,
                        // but will always produce one, so it is ok for us to require one.

                        #[cfg(feature = "tracing")]
                        tracing::debug!("Bucket is a fragment, waiting for rest of message.");

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

                        #[cfg(feature = "tracing")]
                        tracing::trace!("Received DUMMY bucket from peer, ignoring.");
                        // We may have another bucket in `src`.
                        continue;
                    }

                    let max_size = u64_to_usize(if self.bucket_codec.handshake_message_seen {
                        self.bucket_codec.protocol.max_packet_size
                    } else {
                        self.bucket_codec.protocol.max_packet_size_before_handshake
                    });

                    if bytes.len().saturating_add(bucket.body.len()) > max_size {
                        return Err(BucketError::InvalidFragmentedMessage(
                            "Fragmented message exceeded maximum size",
                        ));
                    }

                    #[cfg(feature = "tracing")]
                    tracing::trace!("Received another bucket fragment.");

                    bytes.extend_from_slice(bucket.body.as_ref());

                    if flags.contains(Flags::END_FRAGMENT) {
                        // make sure we only look at the internal bucket and don't use this.
                        drop(bucket);

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
                        if bytes.len().saturating_sub(HEADER_SIZE) < u64_to_usize(header.size) {
                            return Err(BucketError::InvalidFragmentedMessage(
                                "Fragmented message does not have enough bytes to fill bucket body",
                            ));
                        }

                        #[cfg(feature = "tracing")]
                        tracing::debug!(
                            "Received final fragment, combined message command: {:?}.",
                            header.command
                        );

                        let message_type = MessageType::from_flags_and_have_to_return(
                            header.flags,
                            header.have_to_return_data,
                        )?;

                        if header.command.is_handshake() {
                            #[cfg(feature = "tracing")]
                            tracing::debug!(
                                "Peer handshake message seen, increasing bucket size limit."
                            );

                            self.bucket_codec.handshake_message_seen = true;
                        }

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
                let mut bucket_builder = BucketBuilder::new(&self.bucket_codec.protocol);
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
