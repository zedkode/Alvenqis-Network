#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    alvenqis_node::p2p::fuzzing::decode_message_payloads(data);
});
