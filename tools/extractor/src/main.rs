//! Maintainer-run game-data extractor: reads a local game install (Bundles2
//! index, `.dat` tables, stat descriptions) and writes canonical bundles,
//! recording game version, schema revision, and input hashes (ADR-005).
//! Never ships GGG art or audio (ADR-000).
//!
//! Subcommands so far (Spike A, `dev/active/spike-a-bundles2-extraction/`):
//! - `list-files <install> [--decoder <program>] [--sample <n> | --all]`:
//!   reads `Bundles2/_.index.bin`, rebuilds every path, matches paths to file
//!   records, and prints counts, the index hash, every unmatched entry, and
//!   the first `n` files (default 10) or all of them.
//! - `extract <install> --game poe1|poe2 --out <dir> [--decoder <program>]`:
//!   extracts the Spike A tables and every table they reference into
//!   `<dir>/tables/*.json`, plus `<dir>/manifest.json` (see
//!   [`loom_extractor::extract`]). `<dir>` (and `<dir>.partial`, where the
//!   output is written before it is renamed) must not exist yet; this is
//!   checked before anything is decoded. Exits with an error if a root table
//!   is not `ok`, any table failed, or any key is past the end of its target
//!   (`Manifest::problems`); the output is written either way.
//!
//!   Provenance: the extractor's git commit and dirty flag are read when the
//!   command runs, from the source tree the binary was built from. They are
//!   right when the binary is built from the current tree just before the
//!   run, as `cargo run` and `tools/extractor/scripts/check-determinism.sh` do.

#![warn(clippy::indexing_slicing)]

use std::collections::BTreeMap;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use loom_extractor::decompress::ExternalTool;
use loom_extractor::extract::{
    Decoder, ExtractError, Extraction, Extractor, Inputs, MANIFEST_PATH, Manifest, Plan,
    SPIKE_TABLES, Status, TABLES_DIR,
};
use loom_extractor::index::{FileRecord, Index, Listing};
use loom_extractor::install::{
    INDEX_PATH, InstallError, InstallIndex, hex, read_files, read_index, steam_build_id,
};
use loom_extractor::schema::{Game, Schema};
use sha2::{Digest, Sha256};

const USAGE: &str = "usage:
  loom-extractor list-files <install> [--decoder <program>] [--sample <n> | --all]
  loom-extractor extract <install> --game poe1|poe2 --out <dir> [--decoder <program>]

  <install>            game folder that contains Bundles2/
  --decoder <program>  Oodle decoder to run (default: unoodle on PATH;
                       see docs/spikes/spike-a-extraction.md, section 2)
  --sample <n>         print the first n files by path (default 10)
  --all                print every file
  --game <game>        which game the install is: poe1 or poe2
  --out <dir>          output folder, must not exist (convention:
                       local/extract/<game>/<steam build id>)";

/// A parsed command line.
#[derive(Debug, PartialEq, Eq)]
enum Command {
    ListFiles(ListFiles),
    Extract(Extract),
}

/// Parsed `list-files` arguments.
#[derive(Debug, PartialEq, Eq)]
struct ListFiles {
    install: PathBuf,
    decoder: PathBuf,
    /// `None` prints every file.
    sample: Option<usize>,
}

/// Parsed `extract` arguments.
#[derive(Debug, PartialEq, Eq)]
struct Extract {
    install: PathBuf,
    decoder: PathBuf,
    game: Game,
    out: PathBuf,
}

fn parse_args(args: &[String]) -> Result<Command, String> {
    let mut args = args.iter();
    let extract = match args.next().map(String::as_str) {
        Some("list-files") => false,
        Some("extract") => true,
        Some(other) => return Err(format!("unknown subcommand `{other}`")),
        None => return Err("missing subcommand".to_owned()),
    };
    let mut install = None;
    let mut decoder = PathBuf::from("unoodle");
    let mut sample = Some(10);
    let mut game = None;
    let mut out = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--decoder" => {
                decoder = args.next().ok_or("--decoder needs a program")?.into();
            }
            "--sample" if !extract => {
                let n = args.next().ok_or("--sample needs a number")?;
                sample = Some(n.parse().map_err(|_| format!("bad --sample `{n}`"))?);
            }
            "--all" if !extract => sample = None,
            "--game" if extract => {
                game = Some(match args.next().map(String::as_str) {
                    Some("poe1") => Game::Poe1,
                    Some("poe2") => Game::Poe2,
                    _ => return Err("--game needs poe1 or poe2".to_owned()),
                });
            }
            "--out" if extract => out = Some(args.next().ok_or("--out needs a folder")?.into()),
            flag if flag.starts_with("--") => return Err(format!("unknown option `{flag}`")),
            path if install.is_none() => install = Some(PathBuf::from(path)),
            extra => return Err(format!("unexpected argument `{extra}`")),
        }
    }
    let install = install.ok_or("missing <install>")?;
    Ok(if extract {
        Command::Extract(Extract {
            install,
            decoder,
            game: game.ok_or("missing --game")?,
            out: out.ok_or("missing --out")?,
        })
    } else {
        Command::ListFiles(ListFiles {
            install,
            decoder,
            sample,
        })
    })
}

/// Scratch directory for the decoder's files, removed on drop.
struct ScratchDir(PathBuf);

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn bundle_name(index: &Index, bundle: u32) -> &str {
    usize::try_from(bundle)
        .ok()
        .and_then(|i| index.bundles.get(i))
        .map_or("?", |b| b.name.as_str())
}

fn print_listing(
    out: &mut impl Write,
    cmd: &ListFiles,
    install: &InstallIndex,
    listing: &Listing,
) -> std::io::Result<()> {
    let index = &install.index;
    writeln!(out, "index                {INDEX_PATH}")?;
    writeln!(out, "sha256               {}", hex(&install.sha256))?;
    writeln!(
        out,
        "size                 {} bytes, decoded {} bytes",
        install.file_len, install.decoded_len
    )?;
    writeln!(out, "decoder              {}", cmd.decoder.display())?;
    writeln!(out, "bundles              {}", index.bundles.len())?;
    writeln!(out, "file records         {}", index.files.len())?;
    writeln!(out, "path reps            {}", index.path_reps.len())?;
    writeln!(out, "path data            {} bytes", index.path_data.len())?;
    writeln!(out, "listed files         {}", listing.files.len())?;
    writeln!(
        out,
        "paths without record {}",
        listing.paths_without_record.len()
    )?;
    for path in &listing.paths_without_record {
        writeln!(out, "  {path}")?;
    }
    writeln!(
        out,
        "records without path {}",
        listing.records_without_path.len()
    )?;
    for rec in &listing.records_without_path {
        writeln!(
            out,
            "  hash {:016x}  {}  offset {}  size {}",
            rec.path_hash,
            bundle_name(index, rec.bundle),
            rec.offset,
            rec.size
        )?;
    }
    writeln!(
        out,
        "dirs without path    {}",
        listing.path_reps_without_directory.len()
    )?;
    for rep in &listing.path_reps_without_directory {
        writeln!(out, "  hash {:016x}  size {}", rep.hash, rep.size)?;
    }
    let shown = cmd.sample.unwrap_or(listing.files.len());
    writeln!(out, "files (path, bundle, offset, size), first {shown}:")?;
    for file in listing.files.iter().take(shown) {
        writeln!(
            out,
            "  {}\t{}\t{}\t{}",
            file.path,
            bundle_name(index, file.record.bundle),
            file.record.offset,
            file.record.size
        )?;
    }
    out.flush()
}

impl ScratchDir {
    fn create() -> Result<Self, String> {
        let dir = std::env::temp_dir().join(format!("loom-extractor-{}", std::process::id()));
        // `create_dir` fails if the directory exists, so a pre-made one is never used.
        std::fs::create_dir(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        Ok(Self(dir))
    }
}

fn run_list_files(cmd: &ListFiles) -> Result<(), String> {
    let scratch = ScratchDir::create()?;
    let decoder = ExternalTool::new(&cmd.decoder, &scratch.0);

    let install =
        read_index(&cmd.install, &decoder).map_err(|e| format!("error[{}]: {e}", e.code()))?;
    let listing = install
        .index
        .list()
        .map_err(|e| format!("error[{}]: {e}", e.code()))?;

    let mut out = BufWriter::new(std::io::stdout().lock());
    print_listing(&mut out, cmd, &install, &listing)
        .map_err(|e| format!("cannot write output: {e}"))
}

/// An `extract` failure, as a message for the user that starts with its
/// stable code (`error[CODE]: ...`, ADR-014).
#[derive(Debug)]
struct Failure(String);

impl Failure {
    fn new(code: &str, message: impl std::fmt::Display) -> Self {
        Self(format!("error[{code}]: {message}"))
    }
}

impl From<InstallError> for Failure {
    fn from(e: InstallError) -> Self {
        Self::new(e.code(), e)
    }
}

impl From<ExtractError> for Failure {
    fn from(e: ExtractError) -> Self {
        Self::new(e.code(), e)
    }
}

/// Runs `git` in the extractor's source tree; `None` if it fails.
fn git(args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// The decoder's file name (no folder: no machine paths in output) and what
/// `<program> --version` prints.
fn decoder_info(program: &Path) -> Decoder {
    let version = std::process::Command::new(program)
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned());
    Decoder {
        program: program
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
        version,
    }
}

/// Index paths and records of the planned tables that have a file.
fn planned_files(
    plan: &Plan<'_>,
    listing: &Listing,
) -> Result<(Vec<String>, Vec<FileRecord>), Failure> {
    // Paths in the index are lowercase; compare that way to be safe.
    let mut by_path: BTreeMap<String, Vec<FileRecord>> =
        plan.sources().map(|path| (path, Vec::new())).collect();
    for file in &listing.files {
        if let Some(records) = by_path.get_mut(&file.path.to_ascii_lowercase()) {
            records.push(file.record);
        }
    }
    let mut paths = Vec::new();
    let mut records = Vec::new();
    for (path, found) in by_path {
        match found.as_slice() {
            [] => {}
            [record] => {
                paths.push(path);
                records.push(*record);
            }
            _ => {
                return Err(Failure::new(
                    "EXTRACT_CLI_DUPLICATE_INDEX_PATH",
                    format!("{path} is listed {} times in the index", found.len()),
                ));
            }
        }
    }
    Ok((paths, records))
}

/// Folder the output is written into before it is renamed to `out`, so a
/// run that stops halfway never leaves a complete-looking `out`.
fn partial_dir(out: &Path) -> PathBuf {
    let mut name = out.as_os_str().to_owned();
    name.push(".partial");
    PathBuf::from(name)
}

/// Fails if `out` or its partial folder exists (checked before decoding).
fn check_output_free(out: &Path) -> Result<(), Failure> {
    for path in [out.to_path_buf(), partial_dir(out)] {
        if std::fs::symlink_metadata(&path).is_ok() {
            return Err(Failure::new(
                "EXTRACT_CLI_OUTPUT_EXISTS",
                format!("{} exists; remove it or pick another --out", path.display()),
            ));
        }
    }
    Ok(())
}

/// Writes the table files and the manifest into `<out>.partial`, then
/// renames it to `out`. Neither may exist.
fn write_output(
    out: &Path,
    files: &BTreeMap<String, Vec<u8>>,
    manifest: &[u8],
) -> Result<(), Failure> {
    let fail = |path: &Path, e: std::io::Error| {
        Failure::new(
            "EXTRACT_CLI_OUTPUT_WRITE",
            format!("cannot write {}: {e}", path.display()),
        )
    };
    check_output_free(out)?;
    if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| fail(parent, e))?;
    }
    let partial = partial_dir(out);
    // `create_dir` fails if the folder exists, so no stale file survives.
    std::fs::create_dir(&partial).map_err(|e| fail(&partial, e))?;
    let tables = partial.join(TABLES_DIR);
    std::fs::create_dir(&tables).map_err(|e| fail(&tables, e))?;
    for (path, bytes) in files {
        let path = partial.join(path);
        std::fs::write(&path, bytes).map_err(|e| fail(&path, e))?;
    }
    let path = partial.join(MANIFEST_PATH);
    std::fs::write(&path, manifest).map_err(|e| fail(&path, e))?;
    std::fs::rename(&partial, out).map_err(|e| fail(out, e))
}

/// Prints the run summary.
fn print_summary(m: &Manifest, manifest: &[u8]) {
    let mut by_status: BTreeMap<Status, usize> = BTreeMap::new();
    for table in m.tables.values() {
        *by_status.entry(table.status).or_default() += 1;
    }
    let keys = || m.tables.values().flat_map(|t| t.key_columns.values());
    let dangling: usize = keys().filter_map(|k| k.dangling).sum();
    let unchecked = keys().filter(|k| k.dangling.is_none()).count();
    let rows: usize = m.files.values().map(|f| f.rows).sum();
    let non_finite: u64 = m.tables.values().map(|t| t.non_finite_floats).sum();
    let unknown = "unknown";
    println!("game                 {}", m.game);
    println!(
        "steam build id       {}",
        m.steam_build_id.as_deref().unwrap_or(unknown)
    );
    println!("index sha256         {}", m.index.sha256);
    println!(
        "extractor commit     {} (dirty: {})",
        m.extractor.commit.as_deref().unwrap_or(unknown),
        m.extractor
            .dirty
            .map_or(unknown, |d| if d { "yes" } else { "no" })
    );
    println!(
        "decoder              {} ({})",
        m.decoder.program,
        m.decoder.version.as_deref().unwrap_or(unknown)
    );
    println!("bundles read         {}", m.bundles.len());
    println!("tables planned       {}", m.tables.len());
    for (status, count) in &by_status {
        println!("  {:<18} {count}", format!("{status:?}"));
    }
    for (name, table) in &m.tables {
        if table.status != Status::Ok {
            println!("  {name}: {:?}", table.status);
        }
    }
    println!("files written        {} ({rows} rows)", m.files.len());
    println!("dangling keys        {dangling} ({unchecked} key columns unchecked)");
    println!("non-finite floats    {non_finite}");
    println!("manifest sha256      {}", hex(&Sha256::digest(manifest)));
}

/// Runs `extract`; `Ok(false)` if the output was written but has problems.
fn run_extract(cmd: &Extract) -> Result<bool, Failure> {
    check_output_free(&cmd.out)?;
    let scratch = ScratchDir::create().map_err(|e| Failure::new("EXTRACT_CLI_SCRATCH", e))?;
    let decoder = ExternalTool::new(&cmd.decoder, &scratch.0);
    let install = read_index(&cmd.install, &decoder)?;
    let listing = install.index.list().map_err(InstallError::from)?;
    let schema = Schema::vendored().map_err(|e| Failure::new(e.code(), e))?;
    let plan = Plan::new(&schema, cmd.game, &SPIKE_TABLES);
    let (paths, records) = planned_files(&plan, &listing)?;

    let mut extraction = Extraction::new(plan);
    let bundles = read_files(
        &cmd.install,
        &install.index,
        &records,
        &decoder,
        |i, bytes| {
            let path = paths.get(i).map_or("", String::as_str);
            extraction.add(path, bytes).map_err(Failure::from)
        },
    )?;
    let output = extraction.finish(Inputs {
        steam_build_id: steam_build_id(&cmd.install),
        index_sha256: install.sha256,
        bundles,
        decoder: decoder_info(&cmd.decoder),
        extractor: Extractor {
            commit: git(&["rev-parse", "HEAD"]),
            dirty: git(&["status", "--porcelain"]).map(|s| !s.is_empty()),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        },
    });
    let manifest = output.manifest.to_json()?;
    write_output(&cmd.out, &output.files, &manifest)?;
    print_summary(&output.manifest, &manifest);

    let problems = output.manifest.problems();
    for problem in &problems {
        eprintln!("loom-extractor: problem: {problem}");
    }
    Ok(problems.is_empty())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = match parse_args(&args) {
        Ok(cmd) => cmd,
        Err(message) => {
            eprintln!("loom-extractor: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let result = match &cmd {
        Command::ListFiles(cmd) => run_list_files(cmd).map(|()| true),
        Command::Extract(cmd) => run_extract(cmd).map_err(|Failure(message)| message),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(message) => {
            eprintln!("loom-extractor: {message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loom_extractor::index::ListedFile;

    /// A scratch folder for one test, removed on drop.
    struct TestDir(PathBuf);

    impl TestDir {
        fn new(test: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("loom-main-{test}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            Self(dir)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn code(failure: &Failure) -> &str {
        failure
            .0
            .strip_prefix("error[")
            .and_then(|s| s.split(']').next())
            .unwrap_or("")
    }

    const fn record(offset: u32) -> FileRecord {
        FileRecord {
            path_hash: 0,
            bundle: 0,
            offset,
            size: 1,
        }
    }

    fn listing(files: &[(&str, u32)]) -> Listing {
        Listing {
            files: files
                .iter()
                .map(|&(path, offset)| ListedFile {
                    path: path.to_owned(),
                    record: record(offset),
                })
                .collect(),
            paths_without_record: Vec::new(),
            records_without_path: Vec::new(),
            path_reps_without_directory: Vec::new(),
        }
    }

    #[test]
    fn planned_files_should_match_table_paths_ignoring_case() {
        let schema = Schema::vendored().unwrap();
        let plan = Plan::new(&schema, Game::Poe1, &["Stats"]);
        let (paths, records) = planned_files(
            &plan,
            &listing(&[
                ("data/Stats.datc64", 1),
                ("data/french/stats.datc64", 2),
                ("data/mods.datc64", 3),
            ]),
        )
        .unwrap();
        assert_eq!(paths, ["data/stats.datc64"]);
        assert_eq!(records, [record(1)]);

        let (paths, _) = planned_files(&plan, &listing(&[])).unwrap();
        assert!(paths.is_empty());

        let twice = listing(&[("data/stats.datc64", 1), ("data/stats.datc64", 2)]);
        let err = planned_files(&plan, &twice).unwrap_err();
        assert_eq!(code(&err), "EXTRACT_CLI_DUPLICATE_INDEX_PATH");
    }

    #[test]
    fn write_output_should_rename_partial_and_refuse_existing_output() {
        let dir = TestDir::new("write");
        let out = dir.0.join("nested/out");
        let files: BTreeMap<String, Vec<u8>> = [("tables/T.json".to_owned(), b"{}\n".to_vec())]
            .into_iter()
            .collect();
        write_output(&out, &files, b"m\n").unwrap();
        assert_eq!(std::fs::read(out.join("tables/T.json")).unwrap(), b"{}\n");
        assert_eq!(std::fs::read(out.join(MANIFEST_PATH)).unwrap(), b"m\n");
        assert!(!partial_dir(&out).exists());

        let err = write_output(&out, &files, b"m\n").unwrap_err();
        assert_eq!(code(&err), "EXTRACT_CLI_OUTPUT_EXISTS");
        let other = dir.0.join("other");
        std::fs::create_dir_all(partial_dir(&other)).unwrap();
        let err = check_output_free(&other).unwrap_err();
        assert_eq!(code(&err), "EXTRACT_CLI_OUTPUT_EXISTS");
    }

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn parse_args_should_use_defaults() {
        let cmd = parse_args(&args(&["list-files", "D:/game"])).unwrap();
        assert_eq!(
            cmd,
            Command::ListFiles(ListFiles {
                install: PathBuf::from("D:/game"),
                decoder: PathBuf::from("unoodle"),
                sample: Some(10),
            })
        );
    }

    #[test]
    fn parse_args_should_read_options() {
        let list = |list: &[&str]| match parse_args(&args(list)).unwrap() {
            Command::ListFiles(cmd) => cmd,
            Command::Extract(_) => panic!("not list-files"),
        };
        let cmd = list(&["list-files", "--decoder", "x/unoodle", "D:/game", "--all"]);
        assert_eq!(cmd.decoder, PathBuf::from("x/unoodle"));
        assert_eq!(cmd.sample, None);
        let cmd = list(&["list-files", "D:/game", "--sample", "3"]);
        assert_eq!(cmd.sample, Some(3));
    }

    #[test]
    fn parse_args_should_read_extract() {
        let cmd = parse_args(&args(&[
            "extract",
            "D:/game",
            "--game",
            "poe2",
            "--out",
            "local/x",
            "--decoder",
            "u",
        ]))
        .unwrap();
        assert_eq!(
            cmd,
            Command::Extract(Extract {
                install: PathBuf::from("D:/game"),
                decoder: PathBuf::from("u"),
                game: Game::Poe2,
                out: PathBuf::from("local/x"),
            })
        );
    }

    #[test]
    fn parse_args_should_reject_bad_input() {
        for bad in [
            &[][..],
            &["extract"][..],
            &["list-files"][..],
            &["list-files", "a", "b"][..],
            &["list-files", "a", "--sample", "x"][..],
            &["list-files", "a", "--decoder"][..],
            &["list-files", "a", "--bogus"][..],
            &["list-files", "a", "--game", "poe1"][..],
            &["extract", "a", "--out", "o"][..],
            &["extract", "a", "--game", "poe1"][..],
            &["extract", "a", "--game", "poe3", "--out", "o"][..],
            &["extract", "--game", "poe1", "--out", "o"][..],
            &["extract", "a", "--game", "poe1", "--out", "o", "--all"][..],
        ] {
            assert!(parse_args(&args(bad)).is_err(), "{bad:?}");
        }
    }
}
