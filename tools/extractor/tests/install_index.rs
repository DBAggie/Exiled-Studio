//! Install-backed index check (Spike A, PR A2). Ignored by default: CI has no
//! game install and no Oodle decoder (no GGG bytes in the repo).
//!
//! Maintainer run (PowerShell), with `unoodle` installed as for A1:
//! ```text
//! $env:LOOM_POE1_INSTALL = "D:\SteamLibrary\steamapps\common\Path of Exile"
//! $env:LOOM_POE2_INSTALL = "D:\SteamLibrary\steamapps\common\Path of Exile 2"
//! cargo test --release -p loom-extractor --test install_index -- --ignored --nocapture
//! ```
//! `LOOM_OODLE_TOOL` overrides the decoder path (default: `unoodle` on PATH).

use std::path::PathBuf;

use loom_extractor::decompress::ExternalTool;
use loom_extractor::hash::path_hash;
use loom_extractor::install::{hex, read_index};

/// Most index records must match a rebuilt path. PoE2 build 26476662 has 5
/// unmatched of 4,324,296 (`docs/spikes/spike-a-extraction.md`, section 4).
const MAX_UNMATCHED: usize = 100;

#[test]
#[ignore = "needs a game install and a separately installed Oodle decoder"]
fn read_index_should_list_every_install_file() {
    let program =
        std::env::var_os("LOOM_OODLE_TOOL").map_or_else(|| PathBuf::from("unoodle"), PathBuf::from);
    let work_dir = ScratchDir(std::env::temp_dir().join(format!("loom-a2-{}", std::process::id())));
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
        let read = read_index(&install, &tool).expect("read index");
        let index = &read.index;
        let paths = index.paths().expect("rebuild paths");
        let listing = index.list().expect("list files");

        // Every path and every record is accounted for exactly once.
        assert_eq!(
            listing.files.len() + listing.paths_without_record.len(),
            paths.len(),
            "paths accounted for"
        );
        assert_eq!(
            listing.files.len() + listing.records_without_path.len(),
            index.files.len(),
            "records accounted for (no duplicate hashes expected)"
        );
        assert!(listing.paths_without_record.len() <= MAX_UNMATCHED);
        assert!(listing.records_without_path.len() <= MAX_UNMATCHED);
        assert!(listing.path_reps_without_directory.is_empty());
        assert!(
            listing.files.windows(2).all(|w| w[0] <= w[1]),
            "files sorted by path"
        );

        let stats = listing
            .files
            .iter()
            // PoE1 keeps tables in `data/`, PoE2 in `data/balance/`.
            .find(|f| f.path == "data/stats.datc64" || f.path == "data/balance/stats.datc64")
            .expect("the stats table is listed");
        assert_eq!(stats.record.path_hash, path_hash(&stats.path));

        println!(
            "{var}: sha256 {}, {} bundles, {} records, {} paths, {} listed, \
             {} paths without record, {} records without path, {} path reps",
            hex(&read.sha256),
            index.bundles.len(),
            index.files.len(),
            paths.len(),
            listing.files.len(),
            listing.paths_without_record.len(),
            listing.records_without_path.len(),
            index.path_reps.len(),
        );
    }
}

/// Removes the scratch directory on drop, also when a check panics.
struct ScratchDir(PathBuf);

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
