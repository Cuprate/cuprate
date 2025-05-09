#![no_main]

use bytes::{BufMut, BytesMut};
use tokio_util::codec::Decoder;
use libfuzzer_sys::fuzz_target;

use cuprate_levin::BucketHead;
use cuprate_wire::{LevinCommand, MoneroWireCodec};


fuzz_target!(|data: Vec<(BucketHead<LevinCommand>, Vec<u8>)>| {
    let mut codec = MoneroWireCodec::default();

    for (bucket, body) in data {
        let mut bytes = BytesMut::new();

        bucket.write_bytes_into(&mut bytes);
        bytes.put_slice(&body);

        drop(codec.decode(&mut bytes));
    }
});

