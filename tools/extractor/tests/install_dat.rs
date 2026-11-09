//! Install-backed dat check (Spike A, PR A3): reads every `.datc64` table of
//! the game's table directory with the vendored schema and reports, per game,
//! how many tables decode, which have a row-size mismatch, and which have no
//! schema. No table may fail to decode. Ignored by default: CI has no game
//! install and no Oodle decoder (no GGG bytes in the repo).
//!
//! Maintainer run (PowerShell), with `unoodle` installed as for A1:
//! ```text
//! $env:LOOM_POE1_INSTALL = "D:\SteamLibrary\steamapps\common\Path of Exile"
//! $env:LOOM_POE2_INSTALL = "D:\SteamLibrary\steamapps\common\Path of Exile 2"
//! cargo test --release -p loom-extractor --test install_dat -- --ignored --nocapture
//! ```
//! `LOOM_OODLE_TOOL` overrides the decoder path (default: `unoodle` on PATH).

use std::path::PathBuf;

use loom_extractor::dat::read_table;
use loom_extractor::decompress::ExternalTool;
use loom_extractor::index::{FileRecord, ListedFile};
use loom_extractor::install::{InstallError, hex, read_files, read_index};
use loom_extractor::schema::{Game, Schema, TableSchema};

/// Tables Spike A extracts (A4); each must decode with no row-size mismatch.
const SPIKE_TABLES: [&str; 6] = [
    "stats",
    "mods",
    "baseitemtypes",
    "passiveskills",
    "activeskills",
    "grantedeffects",
];

/// Per-game result.
#[derive(Default)]
struct Report {
    ok: Vec<String>,
    /// Stem, schema width, file width, columns read, schema columns.
    mismatched: Vec<(String, usize, usize, usize, usize)>,
    failed: Vec<(String, String)>,
    no_schema: Vec<String>,
    empty: usize,
    rows: usize,
}

impl Report {
    fn check(&mut self, stem: &str, bytes: &[u8], schema: &TableSchema) {
        match read_table(bytes, schema) {
            Ok(table) => match table.mismatch {
                Some(m) => self.mismatched.push((
                    stem.to_owned(),
                    m.expected,
                    m.actual,
                    table.columns_read,
                    schema.columns.len(),
                )),
                None => {
                    self.empty += usize::from(table.rows.is_empty());
                    self.rows += table.rows.len();
                    self.ok.push(stem.to_owned());
                }
            },
            Err(e) => self
                .failed
                .push((stem.to_owned(), format!("{}: {e}", e.code()))),
        }
    }
}

#[test]
#[ignore = "needs a game install and a separately installed Oodle decoder"]
fn read_table_should_decode_install_tables() {
    let program =
        std::env::var_os("LOOM_OODLE_TOOL").map_or_else(|| PathBuf::from("unoodle"), PathBuf::from);
    let work_dir = ScratchDir(std::env::temp_dir().join(format!("loom-a3-{}", std::process::id())));
    std::fs::create_dir_all(&work_dir.0).expect("create scratch dir");
    let tool = ExternalTool::new(program, &work_dir.0);
    let schema = Schema::vendored().expect("vendored schema");

    let installs: Vec<(Game, &str, PathBuf)> = [
        (Game::Poe1, "LOOM_POE1_INSTALL", "data/"),
        (Game::Poe2, "LOOM_POE2_INSTALL", "data/balance/"),
    ]
    .into_iter()
    .filter_map(|(game, var, dir)| std::env::var_os(var).map(|p| (game, dir, PathBuf::from(p))))
    .collect();
    assert!(
        !installs.is_empty(),
        "set LOOM_POE1_INSTALL and/or LOOM_POE2_INSTALL"
    );

    for (game, dir, install) in installs {
        let read = read_index(&install, &tool).expect("read index");
        let listing = read.index.list().expect("list files");
        // Tables directly in the game's table directory (not language copies).
        let tables: Vec<(&str, &ListedFile)> = listing
            .files
            .iter()
            .filter_map(|f| {
                let name = f.path.strip_prefix(dir)?;
                let stem = name.strip_suffix(".datc64")?;
                (!stem.contains('/')).then_some((stem, f))
            })
            .collect();
        let records: Vec<FileRecord> = tables.iter().map(|(_, f)| f.record).collect();

        let mut report = Report::default();
        read_files(&install, &read.index, &records, &tool, |position, bytes| {
            let stem = tables[position].0;
            match schema.table(game, stem) {
                Some(table) => report.check(stem, bytes, table),
                None => report.no_schema.push(stem.to_owned()),
            }
            Ok::<_, InstallError>(())
        })
        .expect("read table files");

        let schema_only = schema
            .tables_for(game)
            .filter(|t| !tables.iter().any(|(s, _)| t.name.eq_ignore_ascii_case(s)))
            .count();
        report.mismatched.sort();
        report.failed.sort();
        report.no_schema.sort();

        println!(
            "{game:?}: index sha256 {}, {} tables in {dir}, {} ok ({} empty, {} rows), \
             {} row-size mismatches, {} failed, {} without schema; \
             {} schema tables without a file",
            hex(&read.sha256),
            tables.len(),
            report.ok.len(),
            report.empty,
            report.rows,
            report.mismatched.len(),
            report.failed.len(),
            report.no_schema.len(),
            schema_only,
        );
        for (stem, expected, actual, read_cols, cols) in &report.mismatched {
            println!(
                "  mismatch {stem}: schema {expected} bytes, file {actual}; \
                 {read_cols}/{cols} columns read"
            );
        }
        for (stem, error) in &report.failed {
            println!("  failed {stem}: {error}");
        }
        println!("  without schema: {}", report.no_schema.join(", "));

        assert!(
            report.failed.is_empty(),
            "{game:?}: tables failed to decode"
        );
        for stem in SPIKE_TABLES {
            assert!(
                report.ok.iter().any(|s| s == stem),
                "{game:?}: Spike A table {stem} must decode without a mismatch"
            );
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
