#![no_main]

use libfuzzer_sys::fuzz_target;

use cuprate_levin::{LevinBody, MessageType};
use cuprate_wire::{Message, LevinCommand};

const HEADER: &[u8] = b"\x01\x11\x01\x01\x01\x01\x02\x01\x01";

fuzz_target!(|data: (&[u8], MessageType, LevinCommand)| {
    let bytes = [HEADER, data.0].concat();

    drop(Message::decode_message(&mut bytes.as_slice(), data.1, data.2));
});
