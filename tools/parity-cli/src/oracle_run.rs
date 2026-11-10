//! `oracle-run`: runs fixtures through the adapter on the pinned runtime,
//! several times each, and reports every outcome (Spike C plan, step 2a).
//!
//! A run's outcome is either a checked, canonical output document (status
//! `ok`, or `partial` when the adapter names reasons) or a failure class
//! with the stage and message. Only this module assigns classes; the
//! adapter reports facts. Runs of one fixture must have byte-identical
//! outcomes, otherwise the fixture is `ORACLE_NONDETERMINISM`. A failure
//! that repeats identically is deterministic but still a failure, never
//! `ok`.
//!
//! Classification of one run, first match wins:
//! - the process cannot start: `ORACLE_ENVIRONMENT`;
//! - the time limit is reached: `ORACLE_TIMEOUT`;
//! - no output document: `ORACLE_LAUNCH` if the exit code is not 0 (the
//!   adapter did not get to write), else `ORACLE_PROTOCOL`;
//! - a document that is not JSON, breaks the schema, or disagrees with the
//!   exit code (0 for `done`, 1 otherwise): `ORACLE_PROTOCOL`;
//! - stage `startup`: `ORACLE_LAUNCH`; `import`: `ORACLE_IMPORT`;
//!   `calculate`: `ORACLE_CALCULATION`; `normalize`:
//!   `ORACLE_NORMALIZATION`;
//! - done, but PoB found a user settings file or loaded a network module:
//!   `ORACLE_ENVIRONMENT` (the result depends on something outside the
//!   pinned inputs);
//! - done with partial reasons (a shim the build needed failed, such as
//!   `Inflate` without zlib): status `partial`, no class.
//!
//! The checkout is checked after each fixture (including ignored files,
//! so a `.bin` cache the write guard missed is caught); a dirty checkout
//! stops the run, since later fixtures would run on changed inputs.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use crate::oracle::{self, Adapter, FailureClass};
use crate::oracle_output::{self, Probe, Stage};
use crate::oracle_process::{self, Finished};
use crate::oracle_verify::{self, Check};

/// Modules whose loading means PoB could reach the network. The wrapper
/// answers `lcurl.safe` itself without loading it, so it never appears.
const NETWORK_MODULES: &[&str] = &["lcurl", "socket", "ssl", "http"];

/// Exit code of `oracle-run` when nothing failed but a fixture is
/// `partial`: distinct from success, from file (1) and usage (2) errors,
/// and from the failure classes (10 to 20).
pub const EXIT_PARTIAL: u8 = 3;

/// How many characters of a failed run's standard output and error the
/// report keeps.
const STREAM_TAIL: usize = 2000;

/// A fixture: a PoB build XML file.
#[derive(Debug, Clone)]
pub struct Fixture {
    /// File name (the fixture's id in the report and output file name).
    pub id: String,
    /// Where it is read from.
    pub path: PathBuf,
    /// SHA-256 of its bytes.
    pub sha256: String,
}

/// The outcome of one run: what repeated runs must reproduce.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Outcome {
    /// `ok`, `partial`, or `failed`.
    pub status: &'static str,
    /// Failure class; `None` for `ok` and `partial`.
    pub class: Option<FailureClass>,
    /// Stage the adapter reached, when it wrote a document.
    pub stage: Option<&'static str>,
    /// Failure message with machine paths replaced (see [`Scrub`]).
    pub message: Option<String>,
    /// SHA-256 of the canonical output document (`ok` and `partial`).
    pub output_sha256: Option<String>,
}

/// One run as reported.
#[derive(Debug, Serialize)]
pub struct Run {
    /// Exit code, if the process exited with one.
    pub exit_code: Option<i32>,
    /// Wall time in milliseconds (not part of the outcome).
    pub elapsed_ms: u128,
    /// Whether the outcome equals the first run's.
    pub same_as_first: bool,
    /// The outcome, when it differs from the first run's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<Outcome>,
    /// Standard output: size and SHA-256 (PoB prints to it; recorded, not
    /// part of the outcome).
    pub stdout: Stream,
    /// Standard error: size and SHA-256.
    pub stderr: Stream,
    /// The end of standard output, scrubbed, for a failed run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout_tail: Option<String>,
    /// The end of standard error, scrubbed, for a failed run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr_tail: Option<String>,
}

/// A captured output stream, recorded without its text.
#[derive(Debug, Default, Serialize)]
pub struct Stream {
    /// Bytes captured (after lossy UTF-8 decoding).
    pub bytes: usize,
    /// SHA-256 of the captured text.
    pub sha256: Option<String>,
}

impl Stream {
    fn of(text: &str) -> Self {
        Self {
            bytes: text.len(),
            sha256: Some(oracle::sha256_hex(text.as_bytes())),
        }
    }
}

/// Repeated-run agreement.
#[derive(Debug, Serialize)]
pub struct Determinism {
    /// Runs made.
    pub runs: usize,
    /// Runs whose outcome equals the first run's (including the first).
    pub identical: usize,
}

/// One fixture's result.
#[derive(Debug, Serialize)]
pub struct FixtureReport {
    /// File name.
    pub id: String,
    /// SHA-256 of the fixture bytes.
    pub sha256: String,
    /// `ok`, `partial`, or `failed`.
    pub status: &'static str,
    /// Failure class; `ORACLE_NONDETERMINISM` when runs disagree.
    pub class: Option<FailureClass>,
    /// Agreement of the runs.
    pub determinism: Determinism,
    /// The first run's outcome.
    pub outcome: Outcome,
    /// What the shims did in the first run (when it wrote a valid
    /// document); the document's `probe` itself is part of the outcome.
    pub probe: Option<Probe>,
    /// Every run.
    pub runs: Vec<Run>,
}

/// One fixture's result plus the canonical output text to write.
pub struct FixtureResult {
    /// The report entry.
    pub report: FixtureReport,
    /// Pretty canonical document of the first run (`ok` and `partial`).
    pub output: Option<String>,
}

/// Replaces machine-specific paths in messages with fixed tokens, so an
/// outcome does not depend on where the oracle or the fixture lives.
pub struct Scrub {
    replacements: Vec<(String, &'static str)>,
}

impl Scrub {
    /// Scrubs each path (as given and with either separator) to its token.
    pub fn new(paths: &[(&Path, &'static str)]) -> Self {
        let mut replacements = Vec::new();
        for (path, token) in paths {
            let text = path.to_string_lossy().into_owned();
            for variant in [
                text.clone(),
                text.replace('\\', "/"),
                text.replace('/', "\\"),
            ] {
                if !variant.is_empty() {
                    replacements.push((variant, *token));
                }
            }
        }
        // Longest first, so a path wins over its parent directory.
        replacements.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(&b.0)));
        replacements.dedup();
        Self { replacements }
    }

    /// `text` with every known path replaced, every `\` written as `/`
    /// (also a backslash that was part of PoB's own text: harmless, since
    /// it happens the same way every run), and every hex number after
    /// `0x` written as `0x?` (Lua writes addresses in messages, such as
    /// `table: 0x...` or `[C]: at 0x...`, and they change per run).
    pub fn apply(&self, text: &str) -> String {
        let mut text = text.to_owned();
        for (path, token) in &self.replacements {
            text = text.replace(path.as_str(), token);
        }
        hide_addresses(&text.replace('\\', "/"))
    }
}

/// `text` with `0x` followed by hex digits written as `0x?`.
fn hide_addresses(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("0x") {
        let (before, after) = rest.split_at(at);
        out.push_str(before);
        let digits = after[2..].bytes().take_while(u8::is_ascii_hexdigit).count();
        if digits == 0 {
            out.push_str("0x");
        } else {
            out.push_str("0x?");
        }
        rest = &after[2 + digits..];
    }
    out.push_str(rest);
    out
}

fn failed(class: FailureClass, stage: Option<Stage>, message: String) -> Outcome {
    Outcome {
        status: "failed",
        class: Some(class),
        stage: stage.map(Stage::name),
        message: Some(message),
        output_sha256: None,
    }
}

/// One classified run: the outcome; for `ok` and `partial`, the pretty
/// canonical document; and the probe of any valid document.
pub type Classified = (Outcome, Option<String>, Option<Probe>);

/// Classifies one run from what the process did and the output file:
/// its bytes, `None` when it was not written, or the error reading it.
/// A document left behind by a run that hit the time limit is ignored.
pub fn classify(
    started: &Result<Finished, String>,
    document: Result<Option<&[u8]>, &str>,
    adapter: &Adapter,
    scrub: &Scrub,
) -> Classified {
    let finished = match started {
        Err(error) => {
            return (
                failed(FailureClass::Environment, None, scrub.apply(error)),
                None,
                None,
            );
        }
        Ok(finished) => finished,
    };
    if finished.timed_out {
        // No elapsed time in the message: it differs per run.
        let message = "time limit reached".to_owned();
        return (failed(FailureClass::Timeout, None, message), None, None);
    }
    let document = match document {
        Ok(document) => document,
        Err(error) => {
            let message = format!("output document unreadable: {error}");
            return (failed(FailureClass::Protocol, None, message), None, None);
        }
    };
    let Some(bytes) = document else {
        let outcome = match finished.code {
            Some(0) => failed(
                FailureClass::Protocol,
                None,
                "exit code 0 without an output document".to_owned(),
            ),
            code => failed(
                FailureClass::Launch,
                None,
                format!(
                    "{} without an output document",
                    code.map_or_else(|| "no exit code".to_owned(), |c| format!("exit code {c}"))
                ),
            ),
        };
        return (outcome, None, None);
    };
    let document = match oracle_output::parse(bytes, adapter) {
        Ok(document) => document,
        Err(error) => {
            let message = format!("output document: {error}");
            return (failed(FailureClass::Protocol, None, message), None, None);
        }
    };
    let probe = Some(document.probe.clone());
    let stage = Some(document.stage);
    let expected_code = if document.stage == Stage::Done { 0 } else { 1 };
    let problem = if finished.code != Some(expected_code) {
        Some((
            FailureClass::Protocol,
            format!(
                "stage {} with exit code {:?} (expected {expected_code})",
                document.stage.name(),
                finished.code
            ),
        ))
    } else if let Some((source, message)) = &document.error {
        let class = match document.stage {
            Stage::Startup => FailureClass::Launch,
            Stage::Import => FailureClass::Import,
            Stage::Calculate => FailureClass::Calculation,
            Stage::Normalize | Stage::Done => FailureClass::Normalization,
        };
        Some((class, format!("{source}: {}", scrub.apply(message))))
    } else if document.settings_found == Some(true) {
        Some((
            FailureClass::Environment,
            "PoB found a user settings file".to_owned(),
        ))
    } else {
        document
            .modules
            .iter()
            .find(|m| is_network_module(m))
            .map(|module| {
                (
                    FailureClass::Environment,
                    format!("PoB loaded the network module {module}"),
                )
            })
    };
    if let Some((class, message)) = problem {
        return (failed(class, stage, message), None, probe);
    }
    let mut text = match serde_json::to_string_pretty(&document.canonical) {
        Ok(text) => text,
        Err(error) => {
            let outcome = failed(FailureClass::Normalization, stage, error.to_string());
            return (outcome, None, probe);
        }
    };
    text.push('\n');
    let outcome = Outcome {
        status: if document.partial.is_empty() {
            "ok"
        } else {
            "partial"
        },
        class: None,
        stage: Some(document.stage.name()),
        message: None,
        output_sha256: Some(oracle::sha256_hex(text.as_bytes())),
    };
    (outcome, Some(text), probe)
}

fn is_network_module(name: &str) -> bool {
    let root = name.split(['.', '/']).next().unwrap_or(name);
    NETWORK_MODULES.contains(&root)
}

/// One launch: runs the adapter for `fixture`, which must write its
/// document to `output` (a path that does not exist yet).
pub trait Launcher {
    /// Starts the adapter and waits for it.
    fn launch(&self, fixture: &Path, output: &Path) -> Result<Finished, String>;
}

/// Runs `fixture` `runs` times. Each run's raw document is kept in
/// `raw_dir` as `<n>.json`.
pub fn run_fixture(
    fixture: &Fixture,
    runs: usize,
    raw_dir: &Path,
    adapter: &Adapter,
    launcher: &impl Launcher,
    scrub: &Scrub,
) -> Result<FixtureResult, String> {
    std::fs::create_dir_all(raw_dir).map_err(|e| format!("{}: {e}", raw_dir.display()))?;
    let mut first: Option<Classified> = None;
    let mut reports = Vec::new();
    for n in 1..=runs {
        let output = raw_dir.join(format!("{n}.json"));
        // Cannot happen under a fresh --out; guards against overwriting.
        if output.exists() {
            return Err(format!("{}: already exists", output.display()));
        }
        let started = launcher.launch(&fixture.path, &output);
        // Only a missing file means "no document"; any other read error is
        // reported as such.
        let bytes = match std::fs::read(&output) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.kind().to_string()),
        };
        let document = bytes.as_ref().map(Option::as_deref).map_err(String::as_str);
        let (outcome, text, probe) = classify(&started, document, adapter, scrub);
        let (exit_code, elapsed_ms) = match &started {
            Ok(f) => (f.code, f.elapsed.as_millis()),
            Err(_) => (None, 0),
        };
        let same_as_first = first.as_ref().is_none_or(|(o, _, _)| *o == outcome);
        let failed = outcome.status == "failed";
        let tail_of = |text: &str| scrub.apply(tail(text.trim_end(), STREAM_TAIL));
        let (stdout, stderr, stdout_tail, stderr_tail) = match &started {
            Ok(f) => (
                Stream::of(&f.stdout),
                Stream::of(&f.stderr),
                failed.then(|| tail_of(&f.stdout)),
                failed.then(|| tail_of(&f.stderr)),
            ),
            Err(_) => Default::default(),
        };
        reports.push(Run {
            exit_code,
            elapsed_ms,
            same_as_first,
            outcome: (!same_as_first).then(|| outcome.clone()),
            stdout,
            stderr,
            stdout_tail,
            stderr_tail,
        });
        if first.is_none() {
            first = Some((outcome, text, probe));
        }
    }
    let (outcome, output, probe) = first.ok_or_else(|| "at least one run is needed".to_owned())?;
    let identical = reports.iter().filter(|r| r.same_as_first).count();
    let (status, class) = if identical == runs {
        (outcome.status, outcome.class)
    } else {
        ("failed", Some(FailureClass::Nondeterminism))
    };
    Ok(FixtureResult {
        report: FixtureReport {
            id: fixture.id.clone(),
            sha256: fixture.sha256.clone(),
            status,
            class,
            determinism: Determinism { runs, identical },
            outcome,
            probe,
            runs: reports,
        },
        output,
    })
}

/// The last `max` characters of `text`.
fn tail(text: &str, max: usize) -> &str {
    let count = text.chars().count();
    if count <= max {
        return text;
    }
    let skip = count - max;
    text.char_indices()
        .nth(skip)
        .map_or(text, |(i, _)| &text[i..])
}

/// The adapter file as recorded.
#[derive(Debug, Serialize)]
pub struct AdapterRecord {
    /// File name of the script the runtime starts.
    pub file: String,
    /// SHA-256 of every `*.lua` file in the adapter's directory, by name
    /// (the adapter loads `json.lua` from there).
    pub files: BTreeMap<String, String>,
    /// Protocol version.
    pub protocol: u32,
    /// Arguments after the fixture and output paths (the `zlib` argument
    /// as the manifest names it; the launch passes it resolved).
    pub arguments: Vec<String>,
    /// The zlib library the `inflate` shim loads.
    pub zlib: ZlibRecord,
}

/// The zlib library of the `inflate` shim, as recorded.
#[derive(Debug, Serialize)]
pub struct ZlibRecord {
    /// As the manifest names it for this platform.
    pub library: String,
    /// The file it resolves to: relative to the oracle dir for a path, the
    /// system file for a library name; `None` if no file was found (the
    /// adapter then reports the shim as not installed).
    pub file: Option<String>,
    /// SHA-256 of that file.
    pub sha256: Option<String>,
}

/// Where the system loader finds a library by name on the supported Linux
/// (Debian and Ubuntu multiarch first). Searched in this order to record
/// the file a name like `libz.so.1` loads; C4 confirms it on CI.
const SYSTEM_LIBRARY_DIRS: &[&str] = &[
    "/lib/x86_64-linux-gnu",
    "/usr/lib/x86_64-linux-gnu",
    "/lib64",
    "/usr/lib64",
    "/lib",
    "/usr/lib",
];

/// Resolves the manifest's zlib library for `platform`: the record and the
/// value to pass as the adapter's `zlib=` argument (an absolute path for a
/// path, so the adapter can load it with its own folder searched for its
/// dependencies; a library name as is).
pub fn resolve_zlib(
    adapter: &Adapter,
    oracle_dir: &Path,
    platform: &str,
) -> Result<(ZlibRecord, OsString), String> {
    let library = adapter
        .zlib
        .get(platform)
        .ok_or_else(|| format!("adapter.zlib: no library for platform {platform}"))?;
    if library.contains('/') {
        let path = absolute(&oracle_dir.join(library))?;
        let bytes =
            std::fs::read(&path).map_err(|e| format!("adapter.zlib {library}: {}", e.kind()))?;
        let record = ZlibRecord {
            library: library.clone(),
            file: Some(library.clone()),
            sha256: Some(oracle::sha256_hex(&bytes)),
        };
        return Ok((record, path.into_os_string()));
    }
    let found = SYSTEM_LIBRARY_DIRS
        .iter()
        .map(|dir| Path::new(dir).join(library))
        .find(|path| path.is_file())
        .and_then(|path| std::fs::canonicalize(path).ok());
    let sha256 = found
        .as_ref()
        .and_then(|path| std::fs::read(path).ok())
        .map(|bytes| oracle::sha256_hex(&bytes));
    // The adapter loads the file that was hashed, by its canonical path;
    // only when none is found does the system loader resolve the name (the
    // record then has no file, and the shim reports what it loaded).
    let argument = found.as_ref().map_or_else(
        || OsString::from(library),
        |path| path.clone().into_os_string(),
    );
    let record = ZlibRecord {
        library: library.clone(),
        file: found.map(|path| path.to_string_lossy().into_owned()),
        sha256,
    };
    Ok((record, argument))
}

/// SHA-256 of every `*.lua` file next to `adapter`; the adapter itself
/// must be one of them.
fn adapter_files(adapter: &Path) -> Result<BTreeMap<String, String>, String> {
    let dir = adapter.parent().unwrap_or(Path::new("."));
    let dir = if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    };
    let mut files = BTreeMap::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_file() && path.extension().is_some_and(|e| e == "lua") {
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            files.insert(name, oracle::sha256_hex(&bytes));
        }
    }
    let own = adapter
        .file_name()
        .map(|n| n.to_string_lossy().into_owned());
    if !own.is_some_and(|n| files.contains_key(&n)) {
        return Err(format!("{}: not a .lua file", adapter.display()));
    }
    Ok(files)
}

/// The environment the runtime gets.
#[derive(Debug, Serialize)]
pub struct EnvironmentRecord {
    /// Every variable passed; all others are cleared.
    pub names: Vec<&'static str>,
    /// `LUA_PATH` (relative to `pob/src`, the working directory).
    pub lua_path: &'static str,
    /// The working directory, relative to the oracle directory.
    pub working_directory: &'static str,
}

/// The tool that made the report.
#[derive(Debug, Serialize)]
pub struct Tool {
    /// Commit of this repository, if known.
    pub commit: Option<String>,
    /// Whether the working tree had changes.
    pub dirty: Option<bool>,
}

/// `report.json`.
#[derive(Debug, Serialize)]
pub struct Report {
    /// Report format; 1.
    pub format: u32,
    /// `failed` when there is a class; else `partial` when a fixture is
    /// `partial` (a shim it needed failed: complete runs, incomplete
    /// result; exit 3); else `ok`.
    pub status: &'static str,
    /// The smallest failing class over the verification, the fixtures, and
    /// the checks after the runs.
    pub class: Option<FailureClass>,
    /// Platform key.
    pub platform: String,
    /// `oracle-verify` before any run: every pinned input and its check.
    pub oracle: oracle_verify::Report,
    /// The adapter.
    pub adapter: Option<AdapterRecord>,
    /// The runtime environment.
    pub environment: EnvironmentRecord,
    /// Runs per fixture.
    pub runs: usize,
    /// Time limit per run, in seconds.
    pub timeout_s: u64,
    /// This tool.
    pub tool: Tool,
    /// Every fixture.
    pub fixtures: Vec<FixtureReport>,
    /// Checks after the runs (the checkout must still be clean, including
    /// ignored files): after the last fixture, or the failed check after
    /// the fixture that changed it.
    pub after: Vec<Check>,
    /// Fixtures not run because the checkout changed before their turn.
    pub skipped: Vec<String>,
}

/// `LUA_PATH` for runs: PoB's modules relative to `pob/src` and the Lua
/// modules it ships in `pob/runtime/lua`.
pub const LUA_PATH: &str = "./?.lua;../runtime/lua/?.lua;../runtime/lua/?/init.lua";

/// The adapter's arguments after the fixture and output paths.
pub fn adapter_arguments(adapter: &Adapter) -> Vec<String> {
    adapter
        .breakdowns
        .iter()
        .map(|b| format!("breakdown={b}"))
        .chain(adapter.opaque_keys.iter().map(|k| format!("opaque={k}")))
        .collect()
}

impl Report {
    /// The overall class and status from the parts.
    pub fn finish(mut self) -> Self {
        let class = self
            .oracle
            .class
            .into_iter()
            .chain(self.fixtures.iter().filter_map(|f| f.class))
            .chain(self.after.iter().filter(|c| !c.ok).map(|c| c.class))
            .min();
        self.class = class;
        self.status = if class.is_some() {
            "failed"
        } else if self.fixtures.iter().any(|f| f.status == "partial") {
            "partial"
        } else {
            "ok"
        };
        self
    }

    /// A short Markdown summary.
    pub fn summary_markdown(&self) -> String {
        let mut text = format!(
            "# oracle-run\n\nStatus: **{}**{}. Platform `{}`, {} runs per fixture, time limit {} s.\n\n",
            self.status,
            self.class
                .map(|c| format!(" ({})", c.name()))
                .unwrap_or_default(),
            self.platform,
            self.runs,
            self.timeout_s,
        );
        if let Some(manifest) = &self.oracle.manifest {
            text.push_str(&format!(
                "PoB `{}` (`{}`), LuaJIT `{}`, adapter protocol {}.\n\n",
                manifest.pob.tag,
                manifest.pob.commit,
                manifest.luajit.version,
                manifest.adapter.protocol
            ));
        }
        text.push_str(
            "| Fixture | Status | Class | Identical runs | Inflate calls (bytes out) | Writes refused | Output SHA-256 |\n\
             |---|---|---|---|---|---|---|\n",
        );
        for f in &self.fixtures {
            let (inflate, refused) = f.probe.as_ref().map_or_else(
                || ("-".to_owned(), "-".to_owned()),
                |p| {
                    let errors = if p.inflate_errors > 0 {
                        format!(", {} failed", p.inflate_errors)
                    } else {
                        String::new()
                    };
                    (
                        format!("{} ({}{errors})", p.inflate_calls, p.inflated_bytes),
                        p.writes_refused.to_string(),
                    )
                },
            );
            text.push_str(&format!(
                "| {} | {} | {} | {}/{} | {inflate} | {refused} | {} |\n",
                f.id.replace('|', "\\|"),
                f.status,
                f.class.map_or("-", FailureClass::name),
                f.determinism.identical,
                f.determinism.runs,
                f.outcome
                    .output_sha256
                    .as_deref()
                    .map_or("-", |h| &h[..16.min(h.len())]),
            ));
        }
        let dirty: Vec<&Check> = self.after.iter().filter(|c| !c.ok).collect();
        if !dirty.is_empty() {
            text.push_str("\nAfter the runs:\n");
            for check in dirty {
                text.push_str(&format!("- {}: {}\n", check.name, check.actual));
            }
        }
        if !self.skipped.is_empty() {
            text.push_str(&format!(
                "\nNot run (the checkout changed): {}\n",
                self.skipped.join(", ")
            ));
        }
        text
    }
}

/// Fixtures from `paths`: each file as given, and the `*.xml` files
/// directly inside each directory, sorted by name. Ids (file names) must
/// be unique.
pub fn collect_fixtures(paths: &[PathBuf]) -> Result<Vec<Fixture>, String> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            let mut inside: Vec<PathBuf> = std::fs::read_dir(path)
                .map_err(|e| format!("{}: {e}", path.display()))?
                .filter_map(|entry| entry.ok().map(|e| e.path()))
                .filter(|p| {
                    p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("xml"))
                })
                .collect();
            inside.sort();
            files.extend(inside);
        } else {
            files.push(path.clone());
        }
    }
    let mut fixtures: Vec<Fixture> = Vec::new();
    for path in files {
        let id = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .ok_or_else(|| format!("{}: no file name", path.display()))?;
        if fixtures.iter().any(|f| f.id == id) {
            return Err(format!("two fixtures are named {id}"));
        }
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        fixtures.push(Fixture {
            id,
            sha256: oracle::sha256_hex(&bytes),
            path,
        });
    }
    if fixtures.is_empty() {
        return Err("no fixtures".to_owned());
    }
    Ok(fixtures)
}

/// Options of a run.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// Runs per fixture.
    pub runs: usize,
    /// Time limit per run.
    pub timeout: Duration,
}

/// Starts the adapter on the pinned runtime: `luajit <adapter> <fixture>
/// <output> <arguments> zlib=<library>` in `pob/src` with [`LUA_PATH`] and
/// `LUA_CPATH` (the runtime's own directory) only.
pub struct ProcessLauncher {
    luajit: PathBuf,
    adapter: PathBuf,
    working_directory: PathBuf,
    env: Vec<(String, OsString)>,
    arguments: Vec<String>,
    zlib: OsString,
    timeout: Duration,
}

impl Launcher for ProcessLauncher {
    fn launch(&self, fixture: &Path, output: &Path) -> Result<Finished, String> {
        let fixture = absolute(fixture)?;
        let output = absolute(output)?;
        let paths = [
            ("adapter", self.adapter.as_path()),
            ("fixture", fixture.as_path()),
            ("output", output.as_path()),
            ("runtime", self.luajit.as_path()),
            ("zlib", Path::new(&self.zlib)),
        ];
        if let Some(name) = non_ascii_path(&paths) {
            return Err(format!(
                "the {name} path is not ASCII; LuaJIT on Windows opens files through the \
                 ANSI code page, so such a path works on some machines only"
            ));
        }
        let mut args = vec![
            self.adapter.clone().into_os_string(),
            fixture.into_os_string(),
            output.into_os_string(),
        ];
        args.extend(self.arguments.iter().map(OsString::from));
        let mut zlib = OsString::from("zlib=");
        zlib.push(&self.zlib);
        args.push(zlib);
        oracle_process::run(
            &self.luajit,
            &args,
            &self.working_directory,
            &self.env,
            self.timeout,
        )
    }
}

/// On Windows, the name of the first path that is not ASCII. LuaJIT
/// passes file names to the C library's ANSI functions (`fopen`,
/// `LoadLibraryA`), so a character outside the machine's code page cannot
/// be opened (measured in C2a: `Ä` works with code page 1252, `日` fails).
/// Elsewhere paths are bytes, and every path passes.
fn non_ascii_path<'a>(paths: &[(&'a str, &Path)]) -> Option<&'a str> {
    if !cfg!(windows) {
        return None;
    }
    paths
        .iter()
        .find(|(_, path)| !path.as_os_str().to_string_lossy().is_ascii())
        .map(|(name, _)| *name)
}

fn absolute(path: &Path) -> Result<PathBuf, String> {
    std::path::absolute(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// What `oracle-run` is asked to do.
pub struct Request {
    /// `tools/oracle/manifest.json`.
    pub manifest: PathBuf,
    /// The acquired oracle directory.
    pub oracle_dir: PathBuf,
    /// The adapter file.
    pub adapter: PathBuf,
    /// Fixture files or directories.
    pub fixtures: Vec<PathBuf>,
    /// Output directory; must not exist.
    pub out: PathBuf,
    /// Runs and time limit.
    pub options: Options,
    /// This tool's commit and dirty flag.
    pub tool: Tool,
}

/// Runs `oracle-verify`, then every fixture, then the clean-checkout
/// check; writes `report.json`, `summary.md`, `outputs/<id>.json` (the
/// canonical document of each `ok` or `partial` fixture), and
/// `runs/<id>/<n>.json` (each raw document) to `out`. `Err` for a usage or
/// file error before or while writing; every oracle failure is in the
/// report instead.
pub fn run(request: &Request) -> Result<Report, String> {
    if request.options.runs == 0 {
        return Err("--runs must be at least 1".to_owned());
    }
    if request.out.exists() {
        return Err(format!("{}: already exists", request.out.display()));
    }
    let fixtures = collect_fixtures(&request.fixtures)?;
    let adapter_files = adapter_files(&request.adapter)?;
    let manifest_bytes =
        std::fs::read(&request.manifest).map_err(|e| format!("manifest file: {}", e.kind()));
    let platform = oracle::platform();
    let verified = oracle_verify::verify(
        manifest_bytes.as_deref().map_err(Clone::clone),
        &request.oracle_dir,
        &platform,
        oracle_verify::run_probe,
    );
    std::fs::create_dir_all(&request.out).map_err(|e| format!("{}: {e}", request.out.display()))?;
    let mut report = Report {
        format: 1,
        status: "failed",
        class: None,
        platform: platform.clone(),
        adapter: None,
        environment: EnvironmentRecord {
            names: oracle_verify::ENVIRONMENT.to_vec(),
            lua_path: LUA_PATH,
            working_directory: "pob/src",
        },
        runs: request.options.runs,
        timeout_s: request.options.timeout.as_secs(),
        tool: Tool {
            commit: request.tool.commit.clone(),
            dirty: request.tool.dirty,
        },
        fixtures: Vec::new(),
        after: Vec::new(),
        skipped: Vec::new(),
        oracle: verified,
    };
    // Nothing runs on an oracle that failed verification.
    if let (None, Some(manifest)) = (report.oracle.class, report.oracle.manifest.clone()) {
        let adapter = &manifest.adapter;
        let arguments = adapter_arguments(adapter);
        let runtime = oracle::runtime_files(&platform)
            .and_then(|files| files.first())
            .ok_or_else(|| format!("unsupported platform {platform}"))?;
        let oracle_dir = absolute(&request.oracle_dir)?;
        let (zlib_record, zlib) = resolve_zlib(adapter, &oracle_dir, &platform)?;
        report.adapter = Some(AdapterRecord {
            file: request
                .adapter
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            files: adapter_files,
            protocol: adapter.protocol,
            arguments: arguments
                .iter()
                .cloned()
                .chain([format!("zlib={}", zlib_record.library)])
                .collect(),
            zlib: zlib_record,
        });
        let bin = oracle_dir.join("bin");
        let launcher = ProcessLauncher {
            luajit: oracle_dir.join(runtime),
            adapter: absolute(&request.adapter)?,
            working_directory: oracle_dir.join("pob").join("src"),
            env: vec![
                ("LUA_PATH".to_owned(), OsString::from(LUA_PATH)),
                ("LUA_CPATH".to_owned(), oracle_process::cpath(&bin)),
            ],
            arguments,
            zlib,
            timeout: request.options.timeout,
        };
        let out = absolute(&request.out)?;
        let mut scrub_paths: Vec<(PathBuf, &'static str)> = vec![
            (oracle_dir.clone(), "<oracle>"),
            (launcher.adapter.clone(), "<adapter>"),
            (out.clone(), "<out>"),
        ];
        let outputs = out.join("outputs");
        std::fs::create_dir_all(&outputs).map_err(|e| format!("{}: {e}", outputs.display()))?;
        for (index, fixture) in fixtures.iter().enumerate() {
            scrub_paths.push((absolute(&fixture.path)?, "<fixture>"));
            let scrub = Scrub::new(
                &scrub_paths
                    .iter()
                    .map(|(p, t)| (p.as_path(), *t))
                    .collect::<Vec<_>>(),
            );
            scrub_paths.pop();
            let raw = out.join("runs").join(&fixture.id);
            let result = run_fixture(
                fixture,
                request.options.runs,
                &raw,
                adapter,
                &launcher,
                &scrub,
            )?;
            if let Some(text) = &result.output {
                let path = outputs.join(format!("{}.json", fixture.id));
                std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
            }
            report.fixtures.push(result.report);
            let mut clean = oracle_verify::clean_check(&request.oracle_dir);
            if !clean.ok {
                // Names the fixture after which the checkout changed; the
                // remaining fixtures do not run on changed inputs.
                clean.name = format!("pob.clean after {}", fixture.id);
                report.after.push(clean);
                report.skipped = fixtures[index + 1..].iter().map(|f| f.id.clone()).collect();
                break;
            }
            if report.fixtures.len() == fixtures.len() {
                report.after.push(clean);
            }
        }
    }
    let report = report.finish();
    let mut json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    json.push('\n');
    for (name, text) in [
        ("report.json", json),
        ("summary.md", report.summary_markdown()),
    ] {
        let path = request.out.join(name);
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(report)
}

#[cfg(test)]
mod tests;
