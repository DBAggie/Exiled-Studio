//! Install-backed decompression check (Spike A, PR A1). Ignored by default:
//! CI has no game install and no Oodle decoder (no GGG bytes in the repo).
//!
//! Maintainer run (PowerShell):
//! ```text
//! cargo install --locked oozextract@0.5.7 --features cli   # provides `unoodle`
//! $env:LOOM_POE1_INSTALL = "D:\SteamLibrary\steamapps\common\Path of Exile"
//! $env:LOOM_POE2_INSTALL = "D:\SteamLibrary\steamapps\common\Path of Exile 2"
//! cargo test -p loom-extractor --test install_decompress -- --ignored --nocapture
//! ```
//! `LOOM_OODLE_TOOL` overrides the decoder path (default: `unoodle` on PATH).
//!
//! The bundle header comes from `Bundle::parse` (PR A2), which checks the
//! header arithmetic; this test checks that chunks decode on their own too.

#![expect(
    clippy::expect_used,
    reason = "test helpers: a failed check should panic with its message"
)]

use std::path::{Path, PathBuf};

use loom_extractor::bundle::Bundle;
use loom_extractor::decompress::{Decompressor, ExternalTool};

fn u32_at(bytes: &[u8], offset: usize) -> usize {
    let raw: [u8; 4] = bytes
        .get(offset..offset + 4)
        .and_then(|s| s.try_into().ok())
        .expect("field within buffer");
    usize::try_from(u32::from_le_bytes(raw)).expect("u32 fits usize")
}

/// Decodes the payload in one call and the first and last chunks on their
/// own, and compares them.
fn decode_bundle(tool: &ExternalTool, path: &Path) -> Vec<u8> {
    let bytes = std::fs::read(path).expect("read bundle");
    let bundle = Bundle::parse(&bytes).expect("valid bundle header");
    let h = &bundle.header;
    let payload = bundle.payload;
    let whole = bundle.decode(tool).expect("decode whole payload");

    let first_len = h.decompressed_len.min(h.granularity);
    let first_size = usize::try_from(h.chunk_sizes[0]).expect("fits usize");
    let first = tool
        .decompress(&payload[..first_size], first_len)
        .expect("decode first chunk");
    assert_eq!(
        first,
        whole[..first_len],
        "first chunk matches whole-payload decode"
    );

    let last_size =
        usize::try_from(*h.chunk_sizes.last().expect("at least one chunk")).expect("fits usize");
    let last_start_out = (h.chunk_sizes.len() - 1) * h.granularity;
    let last = tool
        .decompress(
            &payload[payload.len() - last_size..],
            h.decompressed_len - last_start_out,
        )
        .expect("decode last chunk");
    assert_eq!(
        last,
        whole[last_start_out..],
        "last chunk matches whole-payload decode"
    );

    println!(
        "  {}: {} bytes -> {} bytes, {} chunks of {}",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
        bytes.len(),
        whole.len(),
        h.chunk_sizes.len(),
        h.granularity,
    );
    whole
}

#[test]
#[ignore = "needs a game install and a separately installed Oodle decoder"]
fn external_tool_should_decode_install_index_bundles() {
    let program =
        std::env::var_os("LOOM_OODLE_TOOL").map_or_else(|| PathBuf::from("unoodle"), PathBuf::from);
    let work_dir = ScratchDir(std::env::temp_dir().join(format!("loom-a1-{}", std::process::id())));
    std::fs::create_dir_all(&work_dir.0).expect("create scratch dir");
    let tool = ExternalTool::new(program, &work_dir.0);

    let installs: Vec<(&str, PathBuf)> = ["LOOM_POE1_INSTALL", "LOOM_POE2_INSTALL"]
        .into_iter()
        .filter_map(|var| std::env::var_os(var).map(|p| (var, PathBuf::from(p))))
        .collect();
    assert!(
        !installs.is_empty(),
        "set LOOM_POE1_INSTALL and/or LOOM_POE2_INSTALL"
    );

    for (var, install) in installs {
        println!("{var}:");
        let bundles = install.join("Bundles2");
        let index = decode_bundle(&tool, &bundles.join("_.index.bin"));
        println!("  index bundle count (first u32): {}", u32_at(&index, 0));
        for name in ["_.index.high.bin", "_.index.low.bin"] {
            let decoded = decode_bundle(&tool, &bundles.join(name));
            println!("  {name} decoded: {decoded:02x?}");
        }
    }
}

/// Removes the scratch directory on drop, also when a check panics.
struct ScratchDir(PathBuf);

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
