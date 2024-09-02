use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::{SinkExt, StreamExt};
use proptest::{prelude::any_with, prop_assert_eq, proptest, sample::size_range};
use rand::Fill;
use tokio::{
    io::duplex,
    time::{timeout, Duration},
};
use tokio_util::codec::{FramedRead, FramedWrite};

use cuprate_helper::cast::u64_to_usize;

use cuprate_levin::{
    message::make_fragmented_messages, BucketBuilder, BucketError, LevinBody, LevinCommand,
    LevinMessageCodec, MessageType, Protocol,
};

/// A timeout put on streams so tests don't stall.
const TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TestCommands(u32);

impl From<u32> for TestCommands {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<TestCommands> for u32 {
    fn from(value: TestCommands) -> Self {
        value.0
    }
}

impl LevinCommand for TestCommands {
    fn bucket_size_limit(&self) -> u64 {
        u64::MAX
    }

    fn is_handshake(&self) -> bool {
        self.0 == 1
    }
}

#[derive(Clone)]
enum TestBody {
    Bytes(usize, Bytes),
}

impl LevinBody for TestBody {
    type Command = TestCommands;

    fn decode_message<B: Buf>(
        body: &mut B,
        _: MessageType,
        _: Self::Command,
    ) -> Result<Self, BucketError> {
        let size = u64_to_usize(body.get_u64_le());
        // bucket
        Ok(TestBody::Bytes(size, body.copy_to_bytes(size)))
    }

    fn encode(self, builder: &mut BucketBuilder<Self::Command>) -> Result<(), BucketError> {
        match self {
            TestBody::Bytes(len, bytes) => {
                let mut buf = BytesMut::new();
                buf.put_u64_le(len as u64);
                buf.extend_from_slice(bytes.as_ref());

                builder.set_command(TestCommands(1));
                builder.set_message_type(MessageType::Notification);
                builder.set_return_code(0);
                builder.set_body(buf.freeze());
            }
        }

        Ok(())
    }
}

#[tokio::test]
async fn codec_fragmented_messages() {
    // Set up the fake connection
    let (write, read) = duplex(100_000);

    let mut read = FramedRead::new(read, LevinMessageCodec::<TestBody>::default());
    let mut write = FramedWrite::new(write, LevinMessageCodec::<TestBody>::default());

    // Create the message to fragment
    let mut buf = BytesMut::from(vec![0; 10_000].as_slice());
    let mut rng = rand::thread_rng();
    buf.try_fill(&mut rng).unwrap();

    let message = TestBody::Bytes(buf.len(), buf.freeze());

    let fragments = make_fragmented_messages(&Protocol::default(), 3_000, message.clone()).unwrap();

    for frag in fragments {
        // Send each fragment
        timeout(TEST_TIMEOUT, write.send(frag.into()))
            .await
            .unwrap()
            .unwrap();
    }

    // only one message should be received.
    let message2 = timeout(TEST_TIMEOUT, read.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    match (message, message2) {
        (TestBody::Bytes(_, buf), TestBody::Bytes(_, buf2)) => assert_eq!(buf, buf2),
    }
}

proptest! {
    #[test]
    fn make_fragmented_messages_correct_size(fragment_size in 100_usize..5000, message_size in 0_usize..100_000) {
        let mut bytes = BytesMut::new();
        bytes.resize(message_size, 10);

        let fragments = make_fragmented_messages(&Protocol::default(), fragment_size, TestBody::Bytes(bytes.len(), bytes.freeze())).unwrap();
        let len = fragments.len();

        for (i, fragment) in fragments.into_iter().enumerate() {
            prop_assert_eq!(fragment.body.len() + 33, fragment_size, "numb_fragments:{}, index: {}", len, i);
            prop_assert_eq!(fragment.header.size + 33, fragment_size as u64);
        }
    }

    #[test]
    fn make_fragmented_messages_consistent(fragment_size in 100_usize..5_000, message in any_with::<Vec<u8>>(size_range(50_000).lift())) {
        let fragments = make_fragmented_messages(&Protocol::default(), fragment_size, TestBody::Bytes(message.len(), Bytes::copy_from_slice(message.as_slice()))).unwrap();

        let mut message2 = Vec::with_capacity(message.len());

        // remove the header and the bytes length.
        message2.extend_from_slice(&fragments[0].body[(33 + 8)..]);

        for frag in fragments.iter().skip(1) {
            message2.extend_from_slice(frag.body.as_ref())
        }

        prop_assert_eq!(message.as_slice(), &message2[0..message.len()], "numb_fragments: {}", fragments.len());

        for byte in message2[message.len()..].iter(){
            prop_assert_eq!(*byte, 0);
        }
    }

}
