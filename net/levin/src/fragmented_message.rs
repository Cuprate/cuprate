use bytes::BytesMut;

use super::{
    header::HEADER_SIZE, Bucket, BucketBuilder, BucketError, BucketHead, Flags, LevinBody, Protocol,
};

pub fn make_fragmented_messages<T: LevinBody>(
    protocol: &Protocol,
    fragment_size: usize,
    message: T,
) -> Result<Vec<Bucket<T::Command>>, BucketError> {
    let mut builder = BucketBuilder::default();
    message.encode(&mut builder)?;
    let mut bucket = builder.finish();

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
            bucket.header.size = fragment_size
                .try_into()
                .expect("Bucket size does not fit into u64");
        }

        return Ok(vec![bucket]);
    }

    // A header put on all fragments.
    // The first fragment will set the START flag, the last will set the END flag.
    let fragment_head = BucketHead {
        signature: protocol.signature,
        size: (fragment_size - HEADER_SIZE)
            .try_into()
            .expect("Bucket size does not fit into u64"),
        have_to_return_data: false,
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
