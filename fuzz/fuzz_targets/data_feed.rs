//! The data feed and its signature (`pack::verify_feed`), against the test
//! key. The input is
//! `<first length: u16 LE><first><second>` (`fuzz/seed.sh`).
#![no_main]

use libfuzzer_sys::fuzz_target;
use loom_data::pack;
use loom_data::signature::{KeyPair, TRUST_FORMAT, TrustList};

fuzz_target!(|data: &[u8]| {
    let Some((len, rest)) = data.split_first_chunk::<2>() else {
        return;
    };
    let at = usize::from(u16::from_le_bytes(*len)).min(rest.len());
    let (feed, signature) = rest.split_at(at);
    let trust = TrustList {
        format: TRUST_FORMAT,
        keys: vec![KeyPair::test_key().trusted("test")],
        revoked: Vec::new(),
    };
    let _ = pack::verify_feed(feed, signature, &trust, Some(1));
});
