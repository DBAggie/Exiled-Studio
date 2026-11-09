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
//!   `<dir>/tables/*.json`, the game's stat-description files into
//!   `<dir>/stat_descriptions/**/*.json` (see [`loom_extractor::statdesc`]),
//!   every passive skill graph (`*.psg`) into
//!   `<dir>/passive_skill_graphs/**/*.json` (see [`loom_extractor::psg`]),
//!   plus `<dir>/manifest.json` (see [`loom_extractor::extract`]). `<dir>` (and `<dir>.partial`, where the
//!   output is written before it is renamed) must not exist yet; this is
//!   checked before anything is decoded. Exits with an error if a root table
//!   is not `ok`, any table failed, any key is past the end of its target,
//!   no stat-description file was found, or one did not decode, or no graph
//!   was found, one did not parse, or one has a dangling connection
//!   (`Manifest::problems`); the output is written either way. Stat-description
//!   lines that were not parsed are listed in the manifest, not errors.
//!
//! - `cross-check <extract-dir> <pob-dir> --game poe1|poe2 --out <dir>`:
//!   compares an `extract` output with a Path of Building checkout and writes
//!   `<dir>/report.json` (every gap with its category and rule) and
//!   `<dir>/summary.json` (the same without the `pob-filtered` key lists);
//!   see [`loom_extractor::crosscheck`]. Exits with an error if the Spike A
//!   gate is not met; the report is written either way.
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

use loom_extractor::crosscheck::{self, Gaps, Report};
use loom_extractor::decompress::ExternalTool;
use loom_extractor::extract::{
    Decoder, ExtractError, Extraction, Extractor, Inputs, MANIFEST_PATH, Manifest, Plan,
    SPIKE_TABLES, Status, TABLES_DIR,
};
use loom_extractor::index::{FileRecord, Index, Listing};
use loom_extractor::install::{
    INDEX_PATH, InstallError, InstallIndex, hex, read_files, read_index, steam_build_id,
};
use loom_extractor::psg::{self, GraphManifest};
use loom_extractor::schema::{Game, Schema};
use loom_extractor::statdesc::{FileStatus, StatDescManifest, source_kind};
use sha2::{Digest, Sha256};

const USAGE: &str = "usage:
  loom-extractor list-files <install> [--decoder <program>] [--sample <n> | --all]
  loom-extractor extract <install> --game poe1|poe2 --out <dir> [--decoder <program>]
  loom-extractor cross-check <extract-dir> <pob-dir> --game poe1|poe2 --out <dir>

  <install>            game folder that contains Bundles2/
  <extract-dir>        an extract output folder
  <pob-dir>            a Path of Building checkout (a clone in local/,
                       never the legacy repository)
  --decoder <program>  Oodle decoder to run (default: unoodle on PATH;
                       see docs/spikes/spike-a-extraction.md, section 2)
  --sample <n>         print the first n files by path (default 10)
  --all                print every file
  --game <game>        which game the install is: poe1 or poe2
  --out <dir>          output folder, must not exist (convention:
                       local/extract/<game>/<steam build id>, or
                       local/cross-check/<game>/<steam build id>)";

/// A parsed command line.
#[derive(Debug, PartialEq, Eq)]
enum Command {
    ListFiles(ListFiles),
    Extract(Extract),
    CrossCheck(CrossCheck),
}

/// Parsed `cross-check` arguments.
#[derive(Debug, PartialEq, Eq)]
struct CrossCheck {
    /// An `extract` output folder.
    extract: PathBuf,
    /// A Path of Building checkout.
    pob: PathBuf,
    game: Game,
    out: PathBuf,
}

/// Which subcommand is being parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    ListFiles,
    Extract,
    CrossCheck,
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
    let kind = match args.next().map(String::as_str) {
        Some("list-files") => Kind::ListFiles,
        Some("extract") => Kind::Extract,
        Some("cross-check") => Kind::CrossCheck,
        Some(other) => return Err(format!("unknown subcommand `{other}`")),
        None => return Err("missing subcommand".to_owned()),
    };
    let wanted = if kind == Kind::CrossCheck { 2 } else { 1 };
    let mut positional: Vec<PathBuf> = Vec::new();
    let mut decoder = PathBuf::from("unoodle");
    let mut sample = Some(10);
    let mut game = None;
    let mut out = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--decoder" if kind != Kind::CrossCheck => {
                decoder = args.next().ok_or("--decoder needs a program")?.into();
            }
            "--sample" if kind == Kind::ListFiles => {
                let n = args.next().ok_or("--sample needs a number")?;
                sample = Some(n.parse().map_err(|_| format!("bad --sample `{n}`"))?);
            }
            "--all" if kind == Kind::ListFiles => sample = None,
            "--game" if kind != Kind::ListFiles => {
                game = Some(match args.next().map(String::as_str) {
                    Some("poe1") => Game::Poe1,
                    Some("poe2") => Game::Poe2,
                    _ => return Err("--game needs poe1 or poe2".to_owned()),
                });
            }
            "--out" if kind != Kind::ListFiles => {
                out = Some(args.next().ok_or("--out needs a folder")?.into());
            }
            flag if flag.starts_with("--") => return Err(format!("unknown option `{flag}`")),
            path if positional.len() < wanted => positional.push(PathBuf::from(path)),
            extra => return Err(format!("unexpected argument `{extra}`")),
        }
    }
    let mut positional = positional.into_iter();
    let mut path = |what: &str| positional.next().ok_or(format!("missing {what}"));
    Ok(match kind {
        Kind::ListFiles => Command::ListFiles(ListFiles {
            install: path("<install>")?,
            decoder,
            sample,
        }),
        Kind::Extract => Command::Extract(Extract {
            install: path("<install>")?,
            decoder,
            game: game.ok_or("missing --game")?,
            out: out.ok_or("missing --out")?,
        }),
        Kind::CrossCheck => Command::CrossCheck(CrossCheck {
            extract: path("<extract-dir>")?,
            pob: path("<pob-dir>")?,
            game: game.ok_or("missing --game")?,
            out: out.ok_or("missing --out")?,
        }),
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

/// Index paths and records of the planned tables that have a file, and of
/// every file in the game's stat-description folder.
fn planned_files(
    plan: &Plan<'_>,
    listing: &Listing,
) -> Result<(Vec<String>, Vec<FileRecord>), Failure> {
    // Paths in the index are lowercase; compare that way to be safe.
    let mut by_path: BTreeMap<String, Vec<FileRecord>> =
        plan.sources().map(|path| (path, Vec::new())).collect();
    for file in &listing.files {
        let path = file.path.to_ascii_lowercase();
        if source_kind(plan.game(), &path).is_some() || psg::is_graph(&path) {
            by_path.entry(path.clone()).or_default();
        }
        if let Some(records) = by_path.get_mut(&path) {
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
    let files = files
        .iter()
        .map(|(path, bytes)| (path.as_str(), bytes.as_slice()))
        .chain(std::iter::once((MANIFEST_PATH, manifest)));
    write_files(out, files)
}

/// Writes `files` (relative path, bytes) into `<out>.partial`, then renames
/// it to `out`. Neither may exist.
fn write_files<'f>(
    out: &Path,
    files: impl IntoIterator<Item = (&'f str, &'f [u8])>,
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
    for (path, bytes) in files {
        let path = partial.join(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| fail(parent, e))?;
        }
        std::fs::write(&path, bytes).map_err(|e| fail(&path, e))?;
    }
    std::fs::rename(&partial, out).map_err(|e| fail(out, e))
}

/// Prints the stat-description part of the run summary.
fn print_stat_descriptions(sd: &StatDescManifest) {
    let mut by_status: BTreeMap<FileStatus, usize> = BTreeMap::new();
    for file in sd.files.values() {
        *by_status.entry(file.status).or_default() += 1;
    }
    let descriptions: usize = sd.files.values().map(|f| f.descriptions).sum();
    let variants: usize = sd.files.values().map(|f| f.english_variants).sum();
    let no_description: usize = sd.files.values().map(|f| f.no_description).sum();
    println!("stat desc files      {} in {}", sd.files.len(), sd.dir);
    for (status, count) in &by_status {
        println!("  {:<18} {count}", format!("{status:?}"));
    }
    println!("descriptions         {descriptions} ({variants} English variants)");
    println!("no_description       {no_description}");
    println!("handlers             {} distinct", sd.handlers.len());
    println!("unparsed lines       {}", sd.findings.len());
    for (kind, count) in &sd.findings_by_kind {
        println!("  {:<18} {count}", format!("{kind:?}"));
    }
}

/// Prints the graph part of the run summary.
fn print_graphs(graphs: &GraphManifest) {
    println!("passive skill graphs {}", graphs.files.len());
    for (path, g) in &graphs.files {
        let n = |v: Option<usize>| v.map_or_else(|| "-".to_owned(), |v| v.to_string());
        println!(
            "  {path}: {:?}, {} nodes, {} connections, {} roots{}",
            g.status,
            n(g.nodes),
            n(g.connections),
            n(g.roots),
            g.error
                .as_ref()
                .map_or_else(String::new, |e| format!(" [{}] {}", e.code, e.message))
        );
    }
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
    println!("dangling keys        {dangling} ({unchecked} key columns unchecked)");
    println!("non-finite floats    {non_finite}");
    print_stat_descriptions(&m.stat_descriptions);
    print_graphs(&m.passive_skill_graphs);
    println!("files written        {} ({rows} rows)", m.files.len());
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

/// Path of `path` inside `root`, with `/` separators.
fn relative(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let parts: Option<Vec<&str>> = rel.components().map(|c| c.as_os_str().to_str()).collect();
    Some(parts?.join("/"))
}

/// Files in `dir` (and below, if `deep`) whose name ends in `ext`, sorted.
fn files_in(dir: &Path, ext: &str, deep: bool) -> Result<Vec<PathBuf>, Failure> {
    let fail = |e: std::io::Error| {
        Failure::new(
            "EXTRACT_CLI_READ",
            format!("cannot read {}: {e}", dir.display()),
        )
    };
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(fail)? {
        let path = entry.map_err(fail)?.path();
        if path.is_dir() {
            if deep {
                out.extend(files_in(&path, ext, deep)?);
            }
        } else if path.to_str().is_some_and(|p| p.ends_with(ext)) {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, Failure> {
    std::fs::read(path).map_err(|e| {
        Failure::new(
            "EXTRACT_CLI_READ",
            format!("cannot read {}: {e}", path.display()),
        )
    })
}

fn read_text(path: &Path) -> Result<String, Failure> {
    String::from_utf8(read_bytes(path)?).map_err(|e| {
        Failure::new(
            "EXTRACT_CLI_READ",
            format!("{} is not UTF-8: {e}", path.display()),
        )
    })
}

/// The commit a checkout's `.git/HEAD` points at (detached, or through one ref).
fn checkout_commit(repo: &Path) -> Option<String> {
    let git = repo.join(".git");
    let head = std::fs::read_to_string(git.join("HEAD")).ok()?;
    let head = head.trim();
    let commit = match head.strip_prefix("ref: ") {
        Some(reference) => std::fs::read_to_string(git.join(reference)).ok()?,
        None => head.to_owned(),
    };
    let commit = commit.trim();
    (commit.len() == 40 && commit.bytes().all(|b| b.is_ascii_hexdigit())).then(|| commit.to_owned())
}

/// Prints one direction's gaps by category and rule.
fn print_gaps(direction: &str, gaps: &Gaps) {
    println!("    {direction:<12} {}", gaps.total);
    for (category, rules) in &gaps.by_category {
        for (rule, result) in rules {
            let category = format!("{category:?}");
            println!("      {category:<15} {rule:<34} {}", result.count);
        }
    }
}

/// Prints the cross-check summary.
fn print_report(report: &Report) {
    let unknown = "unknown";
    let inputs = &report.inputs;
    println!("game                 {}", report.game);
    println!(
        "steam build id       {}",
        inputs.steam_build_id.as_deref().unwrap_or(unknown)
    );
    println!(
        "pob commit           {} (tree {})",
        inputs.provenance.pob_commit.as_deref().unwrap_or(unknown),
        inputs.pob_tree_version
    );
    for (name, family) in &report.families {
        let g = &family.gate;
        println!(
            "{name:<10} pob {}  ours {}  matched {}  gate {}",
            family.pob_keys,
            family.our_keys,
            family.matched,
            if g.met { "met" } else { "NOT met" }
        );
        print_gaps("pob-missing", &family.pob_missing);
        print_gaps("ours-extra", &family.ours_extra);
        println!(
            "    patch-delta {} (limit {}), uncategorized {}, unparsed {}",
            g.patch_delta, g.patch_delta_limit, g.uncategorized, g.unparsed
        );
        for (path, excluded) in &family.excluded_pob_files {
            println!(
                "    excluded {path}: {} keys, {} equal ours ({})",
                excluded.keys, excluded.keys_matching_ours, excluded.reason
            );
        }
        for rule in &family.disabled_rules {
            println!("    disabled rule {rule}");
        }
    }
    println!(
        "gate                 {}",
        if report.gate.met { "met" } else { "NOT met" }
    );
    for reason in &report.gate.not_met {
        println!("  {reason}");
    }
}

/// Runs `cross-check`; `Ok(false)` if the report was written but the gate is not met.
fn run_cross_check(cmd: &CrossCheck) -> Result<bool, Failure> {
    check_output_free(&cmd.out)?;
    let schema = Schema::vendored().map_err(|e| Failure::new(e.code(), e))?;

    let mut ours: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut wanted = vec![MANIFEST_PATH.to_owned()];
    wanted.extend(
        crosscheck::TABLES
            .iter()
            .map(|t| format!("{TABLES_DIR}/{t}.json")),
    );
    for rel in wanted {
        let bytes = read_bytes(&cmd.extract.join(&rel))?;
        ours.insert(rel, bytes);
    }
    for dir in [
        crosscheck::STAT_DESCRIPTIONS_DIR,
        crosscheck::PASSIVE_SKILL_GRAPHS_DIR,
    ] {
        for path in files_in(&cmd.extract.join(dir), ".json", true)? {
            if let Some(rel) = relative(&cmd.extract, &path) {
                ours.insert(rel, read_bytes(&path)?);
            }
        }
    }

    let mut pob: BTreeMap<String, String> = BTreeMap::new();
    for (dir, ext, deep) in crosscheck::POB_DIRS {
        for path in files_in(&cmd.pob.join(dir), ext, deep)? {
            if let Some(rel) = relative(&cmd.pob, &path) {
                pob.insert(rel, read_text(&path)?);
            }
        }
    }
    let tree_dir = cmd.pob.join("src/TreeData");
    let mut versions = Vec::new();
    for entry in std::fs::read_dir(&tree_dir).map_err(|e| {
        Failure::new(
            "EXTRACT_CLI_READ",
            format!("cannot read {}: {e}", tree_dir.display()),
        )
    })? {
        let entry = entry.map_err(|e| Failure::new("EXTRACT_CLI_READ", e))?;
        if entry.path().is_dir() {
            versions.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    let tree_version = crosscheck::newest_tree_version(versions.iter().map(String::as_str))
        .ok_or_else(|| {
            Failure::new(
                "EXTRACT_CLI_NO_TREE",
                format!("no <major>_<minor> folder in {}", tree_dir.display()),
            )
        })?
        .to_owned();
    let tree = crosscheck::tree_path(&tree_version);
    pob.insert(tree.clone(), read_text(&cmd.pob.join(&tree))?);

    let manifest_sha256 = ours
        .get(MANIFEST_PATH)
        .map(|m| hex(&Sha256::digest(m)))
        .unwrap_or_default();
    let report = crosscheck::cross_check(
        crosscheck::Inputs {
            game: cmd.game,
            schema: &schema,
            ours: &ours,
            pob: &pob,
            tree_version: &tree_version,
        },
        crosscheck::Provenance {
            extract_manifest_sha256: manifest_sha256,
            pob_commit: checkout_commit(&cmd.pob),
            tool_commit: git(&["rev-parse", "HEAD"]),
            tool_dirty: git(&["status", "--porcelain"]).map(|s| !s.is_empty()),
            tool_version: env!("CARGO_PKG_VERSION").to_owned(),
        },
    )
    .map_err(|e| Failure::new(e.code(), e))?;
    let json = |r: &Report| r.to_json().map_err(|e| Failure::new("EXTRACT_CLI_JSON", e));
    let (full, summary) = (json(&report)?, json(&report.summary())?);
    write_files(
        &cmd.out,
        [
            ("report.json", full.as_slice()),
            ("summary.json", summary.as_slice()),
        ],
    )?;
    print_report(&report);
    Ok(report.gate.met)
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
        Command::CrossCheck(cmd) => run_cross_check(cmd).map_err(|Failure(message)| message),
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
                // Stat descriptions: every file in the game's folder.
                ("metadata/statdescriptions/a.txt", 4),
                ("Metadata/StatDescriptions/B_filters.txt", 5),
                ("metadata/statdescriptions/c.dat", 6),
                ("data/statdescriptions/d.csd", 7),
                // Passive skill graphs: every .psg file.
                ("Metadata/PassiveSkillGraph.psg", 8),
            ]),
        )
        .unwrap();
        assert_eq!(
            paths,
            [
                "data/stats.datc64",
                "metadata/passiveskillgraph.psg",
                "metadata/statdescriptions/a.txt",
                "metadata/statdescriptions/b_filters.txt",
            ]
        );
        assert_eq!(records, [record(1), record(8), record(4), record(5)]);

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
            Command::Extract(_) | Command::CrossCheck(_) => panic!("not list-files"),
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
    fn parse_args_should_read_cross_check() {
        let cmd = parse_args(&args(&[
            "cross-check",
            "local/extract/poe1/1",
            "local/pob/PathOfBuilding",
            "--game",
            "poe1",
            "--out",
            "local/cross-check/poe1/1",
        ]))
        .unwrap();
        assert_eq!(
            cmd,
            Command::CrossCheck(CrossCheck {
                extract: PathBuf::from("local/extract/poe1/1"),
                pob: PathBuf::from("local/pob/PathOfBuilding"),
                game: Game::Poe1,
                out: PathBuf::from("local/cross-check/poe1/1"),
            })
        );
    }

    #[test]
    fn checkout_commit_should_read_detached_and_ref_heads() {
        let dir = TestDir::new("commit");
        let git = dir.0.join(".git");
        std::fs::create_dir_all(git.join("refs/heads")).unwrap();
        let sha = "1b4078dc4dcd58f7308df7e779974737badee894";
        std::fs::write(git.join("HEAD"), format!("{sha}\n")).unwrap();
        assert_eq!(checkout_commit(&dir.0).as_deref(), Some(sha));
        std::fs::write(git.join("HEAD"), "ref: refs/heads/dev\n").unwrap();
        std::fs::write(git.join("refs/heads/dev"), format!("{sha}\n")).unwrap();
        assert_eq!(checkout_commit(&dir.0).as_deref(), Some(sha));
        std::fs::write(git.join("refs/heads/dev"), "not a commit\n").unwrap();
        assert_eq!(checkout_commit(&dir.0), None);
    }

    #[test]
    fn files_in_should_list_sorted_and_recurse_only_when_asked() {
        let dir = TestDir::new("files");
        std::fs::create_dir_all(dir.0.join("sub")).unwrap();
        for name in ["b.lua", "a.lua", "c.txt", "sub/d.lua"] {
            std::fs::write(dir.0.join(name), "").unwrap();
        }
        let rel = |deep| -> Vec<String> {
            files_in(&dir.0, ".lua", deep)
                .unwrap()
                .iter()
                .map(|p| relative(&dir.0, p).unwrap())
                .collect()
        };
        assert_eq!(rel(false), ["a.lua", "b.lua"]);
        assert_eq!(rel(true), ["a.lua", "b.lua", "sub/d.lua"]);
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
            &["cross-check", "a", "--game", "poe1", "--out", "o"][..],
            &["cross-check", "a", "b", "c", "--game", "poe1", "--out", "o"][..],
            &["cross-check", "a", "b", "--out", "o"][..],
            &[
                "cross-check",
                "a",
                "b",
                "--game",
                "poe1",
                "--decoder",
                "u",
                "--out",
                "o",
            ][..],
            &["bogus"][..],
        ] {
            assert!(parse_args(&args(bad)).is_err(), "{bad:?}");
        }
    }
}
