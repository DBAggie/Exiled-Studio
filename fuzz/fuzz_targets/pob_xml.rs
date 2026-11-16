//! PoB XML, as a user opens or pastes it (`pob_xml::import`).
#![no_main]

use libfuzzer_sys::fuzz_target;
use loom_serialization::legacy::pob_xml;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = pob_xml::import(text);
    }
});
