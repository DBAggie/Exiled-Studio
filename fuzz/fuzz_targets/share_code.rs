//! A pasted share code (`share_code::import`: base64, zlib, PoB XML).
#![no_main]

use libfuzzer_sys::fuzz_target;
use loom_serialization::legacy::share_code;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = share_code::import(text);
    }
});
