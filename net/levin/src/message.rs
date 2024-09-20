//! Levin Messages
//!
//! This module contains the [`LevinMessage`], which allows sending bucket body's, full buckets or dummy messages.
//! The codec will not return [`LevinMessage`] instead it will only return bucket body's. [`LevinMessage`] allows
//! for more control over what is actually sent over the wire at certain times.
use bytes::{Bytes, BytesMut};

use cuprate_helper::cast::usize_to_u64;

use crate::{
    header::{Flags, HEADER_SIZE},
    Bucket, BucketBuilder, BucketError, BucketHead, LevinBody, LevinCommand, Protocol,
};

/// A levin message that can be sent to a peer.
pub enum LevinMessage<T: LevinBody> {
    /// A message body.
    ///
    /// A levin header will be added to this message before it is sent to the peer.
    Body(T),
    /// A full levin bucket.
    ///
    /// This bucket will be sent to the peer directly with no extra information.
    ///
    /// This should only be used to send fragmented messages: [`make_fragmented_messages`]
    Bucket(Bucket<T::Command>),
    /// A dummy message.
    ///
    /// A dummy message which the peer will ignore. The dummy message will be the exact size
    /// (in bytes) of the given `usize` on the wire.
    Dummy(usize),
}

impl<T: LevinBody> From<T> for LevinMessage<T> {
    fn from(value: T) -> Self {
        Self::Body(value)
    }
}

impl<T: LevinBody> From<Bucket<T::Command>> for LevinMessage<T> {
    fn from(value: Bucket<T::Command>) -> Self {
        Self::Bucket(value)
    }
}

/// This represents a dummy message to send to a peer.
///
/// The message, including the header, will be the exact size of the given `usize`.
/// This exists because it seems weird to do this:
/// ```rust,ignore
/// peer.send(1_000);
/// ```
/// This is a lot clearer:
/// ```rust,ignore
/// peer.send(Dummy(1_000));
/// ```
pub struct Dummy(pub usize);

impl<T: LevinBody> From<Dummy> for LevinMessage<T> {
    fn from(value: Dummy) -> Self {
        Self::Dummy(value.0)
    }
}

/// Fragments the provided message into buckets which, when serialised, will all be the size of `fragment_size`.
///
/// This function will produce many buckets that have to be sent in order. When the peer receives these buckets
/// they will combine them to produce the original message.
///
/// The last bucket may be padded with zeros to make it the correct size, the format used to encode the body must
/// allow for extra data at the end of the message this to work.
///
/// `fragment_size` must be more than 2 * [`HEADER_SIZE`] otherwise this will panic.
pub fn make_fragmented_messages<T: LevinBody>(
    protocol: &Protocol,
    fragment_size: usize,
    message: T,
) -> Result<Vec<Bucket<T::Command>>, BucketError> {
    assert!(
        fragment_size * 2 >= HEADER_SIZE,
        "Fragment size: {fragment_size}, is too small, must be at least {}",
        2 * HEADER_SIZE
    );

    let mut builder = BucketBuilder::new(protocol);
    message.encode(&mut builder)?;
    let mut bucket = builder.finish();

    // Make sure we are not trying to fragment a fragment.
    if !bucket
        .header
        .flags
        .intersects(Flags::REQUEST | Flags::RESPONSE)
    {
        // If a bucket does not have the request or response bits set it is a fragment.
        return Err(BucketError::InvalidFragmentedMessage(
            "Can't make a fragmented message out of a message which is already fragmented",
        ));
    }

    // Check if the bucket can fit in one fragment.
    if bucket.body.len() + HEADER_SIZE <= fragment_size {
        // If it can pad the bucket upto the fragment size and just return this bucket.
        if bucket.body.len() + HEADER_SIZE < fragment_size {
            let mut new_body = BytesMut::from(bucket.body.as_ref());
            // Epee's binary format will ignore extra data at the end so just pad with 0.
            new_body.resize(fragment_size - HEADER_SIZE, 0);

            bucket.body = new_body.freeze();
            bucket.header.size = usize_to_u64(fragment_size - HEADER_SIZE);
        }

        return Ok(vec![bucket]);
    }

    // A header put on all fragments.
    // The first fragment will set the START flag, the last will set the END flag.
    let fragment_head = BucketHead {
        signature: protocol.signature,
        size: usize_to_u64(fragment_size - HEADER_SIZE),
        have_to_return_data: false,
        // Just use a default command.
        command: T::Command::from(0),
        return_code: 0,
        flags: Flags::empty(),
        protocol_version: protocol.version,
    };

    // data_space - the amount of actual data we can fit in each fragment.
    let data_space = fragment_size - HEADER_SIZE;

    let amount_of_fragments = (bucket.body.len() + HEADER_SIZE).div_ceil(data_space);

    let mut first_bucket_body = BytesMut::with_capacity(fragment_size);
    // Fragmented messages store the whole fragmented bucket in the combined payloads not just the body
    // so the first bucket contains 2 headers, a fragment header and the actual bucket header we are sending.
    bucket.header.write_bytes_into(&mut first_bucket_body);
    first_bucket_body.extend_from_slice(
        bucket
            .body
            .split_to(fragment_size - (HEADER_SIZE * 2))
            .as_ref(),
    );

    let mut buckets = Vec::with_capacity(amount_of_fragments);
    buckets.push(Bucket {
        header: fragment_head.clone(),
        body: first_bucket_body.freeze(),
    });

    for mut bytes in (1..amount_of_fragments).map(|_| {
        bucket
            .body
            .split_to((fragment_size - HEADER_SIZE).min(bucket.body.len()))
    }) {
        // make sure this fragment has the correct size - the last one might not, so pad it.
        if bytes.len() + HEADER_SIZE < fragment_size {
            let mut new_bytes = BytesMut::from(bytes.as_ref());
            // Epee's binary format will ignore extra data at the end so just pad with 0.
            new_bytes.resize(fragment_size - HEADER_SIZE, 0);
            bytes = new_bytes.freeze();
        }

        buckets.push(Bucket {
            header: fragment_head.clone(),
            body: bytes,
        });
    }

    buckets
        .first_mut()
        .unwrap()
        .header
        .flags
        .toggle(Flags::START_FRAGMENT);
    buckets
        .last_mut()
        .unwrap()
        .header
        .flags
        .toggle(Flags::END_FRAGMENT);

    Ok(buckets)
}

/// Makes a dummy message, which will be the size of `size` when sent over the wire.
pub(crate) fn make_dummy_message<T: LevinCommand>(protocol: &Protocol, size: usize) -> Bucket<T> {
    // A header to put on the dummy message.
    let header = BucketHead {
        signature: protocol.signature,
        size: usize_to_u64(size),
        have_to_return_data: false,
        // Just use a default command.
        command: T::from(0),
        return_code: 0,
        flags: Flags::DUMMY,
        protocol_version: protocol.version,
    };

    let body = Bytes::from(vec![0; size - HEADER_SIZE]);

    Bucket { header, body }
}
