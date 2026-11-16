//! A build document (`dto::v1::from_json`).
#![no_main]

use libfuzzer_sys::fuzz_target;
use loom_serialization::dto::v1;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = v1::from_json(text);
    }
});
