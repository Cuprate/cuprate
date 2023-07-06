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

use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use crate::{Bucket, BucketError, BucketHead};

/// The levin tokio-codec for decoding and encoding
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
                        body: src.copy_to_bytes(body_len),
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
