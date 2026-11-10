//! Oracle and corpus harnesses, maintainer-run. The only workspace code
//! besides the extractor that reads disk: core crates take bytes
//! (ADR-016), and every extract- or corpus-backed check is a subcommand
//! here (Spike B plan, `dev/completed/spike-b-text-stat-resolution/`).
//!
//! The PoB oracle harness (drives legacy `HeadlessWrapper.lua` on pinned
//! LuaJIT at a pinned public PoB tag, ADR-006) is Spike C's
//! (`dev/completed/spike-c-oracle-harness/`): `oracle-verify`, `oracle-run`,
//! `oracle-compare`, and `divergence-check` (the divergence register,
//! `docs/parity/`).

mod categorize;
mod divergence;
mod extract;
mod grafts;
mod item_tags;
mod load_statdesc;
mod lookup;
mod mod_filter;
mod oracle;
mod oracle_compare;
mod oracle_fixtures;
mod oracle_output;
mod oracle_process;
mod oracle_run;
mod oracle_verify;
mod properties;
mod render_check;
mod resolve_corpus;
mod round_trip;

use std::path::Path;
use std::process::ExitCode;

use serde::Serialize;

const USAGE: &str = "usage:
  loom-parity-cli load-statdesc <extract-dir>
      Load every stat-description file of a `loom-extractor extract` output
      through loom-data; print counts as JSON. Exit 1 if a check fails.
  loom-parity-cli round-trip <extract-dir> --out <dir>
      Render every variant with values inside its conditions and resolve
      the lines back; write summary.json and report.json (every sample
      that did not read back) to <dir>, which must not exist. Exit 1 if a
      line is read wrongly (value-mismatch, wrong-description, split).
  loom-parity-cli render-check <extract-dir> <pob-dir> --out <dir>
      Render every PoE1 Mods row and compare with PoB's exported mod text
      (<pob-dir>/src/Data/Mod*.lua); write summary.json and report.json to
      <dir>, which must not exist. Exit 1 if a difference is uncategorized.
  loom-parity-cli resolve-corpus <builds-dir> <extract-dir> --out <dir> [--pob <tag>=<dir>]...
      Read every <Item> of the PoB builds (*.xml directly in <builds-dir>;
      subfolders are not read; identical files count once), resolve
      each modifier line in its item's scope (PoE1 extract), and
      categorize every line that is not resolved; write summary.json and
      report.json (builds by content hash, no item names) to <dir>, which
      must not exist. Each --pob names a `git archive` of a PoB tag's
      src/Data (<dir>/src/Data), the legacy-text evidence. Exit 1 while
      the Spike B gate is not met or a build is unreadable.
  loom-parity-cli oracle-verify <manifest> <oracle-dir>
      Check an oracle directory written by tools/oracle/acquire.ps1 or
      acquire.sh against the manifest (tools/oracle/manifest.json): PoB
      checkout commit, tree, and no untracked, ignored, or modified file;
      runtime source commits; built files against build.json; the
      runtime's version and build flags. Offline. Print a JSON report.
      Exit 10 (ORACLE_ACQUISITION) or 11 (ORACLE_ENVIRONMENT) on failure.
  loom-parity-cli oracle-run <manifest> <oracle-dir> --out <dir> [--runs <n>] [--timeout <s>]
                  [--adapter <file>] [--anonymous] <fixture>...
      Verify the oracle directory as oracle-verify does, then run each
      fixture (a PoB build XML file, every *.xml directly in a directory,
      or the fixtures of a fixture manifest *.json such as
      tools/oracle/fixtures/fixtures.json, checked against their pins and
      with the outcome each must have) <n> times (default 3) through the
      adapter (default <manifest dir>/adapter/oracle.lua) with a time
      limit of <s> seconds per run (default 120). --anonymous names each
      fixture by its SHA-256 instead of its file name (corpus builds).
      Offline. Write report.json, summary.md, the canonical output per
      fixture (outputs/), and each run's raw document (runs/) to <dir>,
      which must not exist. The checkout is checked after each fixture; a
      change stops the run. Exit 0 when every fixture is ok with identical
      runs (or has the outcome its manifest names) and the checkout is
      still clean; else the exit code of the smallest failing class (10
      to 18); else 4 when a fixture did not have the outcome its manifest
      names; else 3 when a fixture is partial (a shim it needed failed).
  loom-parity-cli oracle-compare <left> <right> --out <dir> [--register <file>] [--fixture <id>]
      Compare two canonical output documents (oracle-run's outputs/*.json;
      the fixture id is --fixture, or the right file name without .json),
      or every fixture of two oracle-run output directories (paired by
      id), field by field: stage, partial, main_output, breakdowns. A
      difference is expected when an accepted entry of the divergence
      register (docs/parity/divergence-register.json) covers it, else
      unexpected. Write report.json and summary.md to <dir>, which must
      not exist. Exit 0 when identical, 19 (ORACLE_EXPECTED_DIFFERENCE)
      when every difference is registered, 20
      (ORACLE_UNEXPECTED_DIFFERENCE) otherwise, or the class of a fixture
      that has no output on a side when that is smaller.
  loom-parity-cli divergence-check <register>
      Validate a divergence register (docs/parity/divergence-register.json)
      against its format (docs/parity/README.md). Print a JSON report.
      Exit 1 if it is invalid.";

/// Runs `git` in this crate's source tree; `None` if it fails.
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

fn reader(dir: &Path) -> impl Fn(&str) -> Result<Vec<u8>, String> + '_ {
    move |relative| {
        let path = dir.join(relative);
        std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))
    }
}

fn load_statdesc(extract: &Path) -> Result<bool, String> {
    let read = reader(extract);
    let manifest = read("manifest.json")?;
    let summary = load_statdesc::run(&manifest, &read)?;
    let json = serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?;
    println!("{json}");
    for problem in &summary.problems {
        eprintln!("problem: {problem}");
    }
    Ok(summary.problems.is_empty())
}

/// Creates `out` (which must not exist) and writes both files.
fn write_out(out: &Path, summary: &impl Serialize, report: &impl Serialize) -> Result<(), String> {
    if out.exists() {
        return Err(format!("{}: already exists", out.display()));
    }
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    for (name, text) in [
        ("summary.json", serde_json::to_string_pretty(summary)),
        ("report.json", serde_json::to_string_pretty(report)),
    ] {
        let mut text = text.map_err(|e| e.to_string())?;
        text.push('\n');
        let path = out.join(name);
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

fn round_trip(extract: &Path, out: &Path) -> Result<bool, String> {
    let read = reader(extract);
    let loaded = extract::load(&read)?;
    let (lookup, lookup_report) = lookup::TableLookup::load(&read, &loaded)?;
    let (summary, report) = round_trip::run(&loaded, &lookup, lookup_report)?;
    write_out(out, &summary, &report)?;
    let json = serde_json::to_string_pretty(&summary.outcomes).map_err(|e| e.to_string())?;
    println!("{json}");
    Ok(!round_trip::WRONG_READINGS
        .iter()
        .any(|o| summary.outcomes.contains_key(*o)))
}

/// `*.lua` files directly in `dir`, sorted by name.
fn lua_files(dir: &Path) -> Result<Vec<(String, std::path::PathBuf)>, String> {
    let mut files: Vec<(String, std::path::PathBuf)> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().into_string().ok()?;
            (name.ends_with(".lua") && entry.file_type().ok()?.is_file())
                .then(|| (name, entry.path()))
        })
        .collect();
    files.sort();
    Ok(files)
}

fn read_text(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn render_check(extract: &Path, pob: &Path, out: &Path) -> Result<bool, String> {
    let read = reader(extract);
    let loaded = extract::load(&read)?;
    let data = pob.join("src").join("Data");
    let mut theirs = render_check::Pob::default();
    for (name, path) in lua_files(&data)? {
        if name.starts_with("Mod") && !render_check::NON_MOD_ID_FILES.contains(&name.as_str()) {
            theirs.add_mod_file(&name, &read_text(&path)?);
        }
    }
    for (name, path) in lua_files(&data.join("StatDescriptions"))? {
        theirs.add_stat_file(&name, &read_text(&path)?);
    }
    let (summary, report) = render_check::run(&loaded, &read, &theirs)?;
    write_out(out, &summary, &report)?;
    let json = serde_json::to_string_pretty(&summary.outcomes).map_err(|e| e.to_string())?;
    println!("{json}");
    Ok(!summary.outcomes.contains_key("differs/uncategorized"))
}

/// Every `*.lua` file under `dir`, as (path relative to `dir` with `/`,
/// text), sorted.
fn lua_tree(dir: &Path) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).map_err(|e| format!("{}: {e}", d.display()))? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "lua") {
                let relative = path
                    .strip_prefix(dir)
                    .map_err(|e| e.to_string())?
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                out.push((relative, read_text(&path)?));
            }
        }
    }
    out.sort();
    Ok(out)
}

/// `<tag>=<dir>` arguments to PoB text sources.
fn pob_sources(args: &[String]) -> Result<Vec<categorize::PobText>, String> {
    let mut out = Vec::new();
    let mut rest = args;
    while let [flag, value, tail @ ..] = rest {
        if flag != "--pob" {
            return Err(format!("unknown argument {flag}"));
        }
        let (tag, dir) = value
            .split_once('=')
            .ok_or_else(|| format!("--pob {value}: expected <tag>=<dir>"))?;
        let data = Path::new(dir).join("src").join("Data");
        out.push(categorize::PobText::read(tag, lua_tree(&data)?));
        rest = tail;
    }
    if !rest.is_empty() {
        return Err(format!("unexpected argument {}", rest.join(" ")));
    }
    Ok(out)
}

fn resolve_corpus(
    builds_dir: &Path,
    extract: &Path,
    out: &Path,
    pob: &[String],
) -> Result<bool, String> {
    let pob = pob_sources(pob)?;
    let read = reader(extract);
    let loaded = extract::load(&read)?;
    let (lookup, lookup_report) = lookup::TableLookup::load(&read, &loaded)?;
    let bases = resolve_corpus::Bases::load(&read)?;
    let mods = mod_filter::Mods::load(&read)?;
    let game = categorize::GameText::load(&loaded.set, &read, pob)?;
    let mut builds = Vec::new();
    for entry in
        std::fs::read_dir(builds_dir).map_err(|e| format!("{}: {e}", builds_dir.display()))?
    {
        let path = entry.map_err(|e| e.to_string())?.path();
        let is_xml = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("xml"));
        if is_xml && path.is_file() {
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            builds.push(resolve_corpus::Build::new(bytes));
        }
    }
    let tool = resolve_corpus::Tool {
        commit: git(&["rev-parse", "HEAD"]),
        dirty: git(&["status", "--porcelain"]).map(|s| !s.is_empty()),
    };
    let (summary, report) = resolve_corpus::run(
        &loaded,
        &lookup,
        lookup_report,
        &bases,
        &mods,
        &game,
        &builds,
        tool,
    )?;
    write_out(out, &summary, &report)?;
    let json =
        serde_json::to_string_pretty(&(&summary.rate, &summary.gate)).map_err(|e| e.to_string())?;
    println!("{json}");
    Ok(summary.gate.met && summary.unreadable_builds.is_empty())
}

fn oracle_verify(manifest: &Path, oracle_dir: &Path) -> ExitCode {
    let bytes = std::fs::read(manifest).map_err(|e| format!("manifest file: {}", e.kind()));
    let report = oracle_verify::verify(
        bytes.as_deref().map_err(Clone::clone),
        oracle_dir,
        &oracle::platform(),
        oracle_verify::run_probe,
    );
    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("loom-parity-cli: {e}");
            return ExitCode::FAILURE;
        }
    }
    match report.class {
        None => ExitCode::SUCCESS,
        Some(class) => {
            for check in report.checks.iter().filter(|c| !c.ok) {
                eprintln!(
                    "{}: expected {}, found {}",
                    check.name, check.expected, check.actual
                );
            }
            ExitCode::from(class.exit_code())
        }
    }
}

/// Parses the arguments after `oracle-run <manifest> <oracle-dir>`.
fn oracle_run_request(
    manifest: &Path,
    oracle_dir: &Path,
    rest: &[String],
) -> Result<oracle_run::Request, String> {
    let mut options = oracle_run::Options {
        runs: 3,
        timeout: std::time::Duration::from_secs(120),
    };
    let mut adapter = manifest
        .parent()
        .unwrap_or(Path::new("."))
        .join("adapter")
        .join("oracle.lua");
    let mut out = None;
    let mut anonymous = false;
    let mut fixtures = Vec::new();
    let mut rest = rest;
    while let [first, tail @ ..] = rest {
        match (first.as_str(), tail) {
            ("--out", [value, tail @ ..]) => {
                out = Some(value.into());
                rest = tail;
            }
            ("--runs", [value, tail @ ..]) => {
                options.runs = value
                    .parse()
                    .map_err(|_| format!("--runs {value}: not a count"))?;
                rest = tail;
            }
            ("--timeout", [value, tail @ ..]) => {
                let seconds: u64 = value
                    .parse()
                    .map_err(|_| format!("--timeout {value}: not a number of seconds"))?;
                options.timeout = std::time::Duration::from_secs(seconds);
                rest = tail;
            }
            ("--adapter", [value, tail @ ..]) => {
                adapter = value.into();
                rest = tail;
            }
            ("--anonymous", tail) => {
                anonymous = true;
                rest = tail;
            }
            (flag, _) if flag.starts_with("--") => {
                return Err(format!("{flag}: unknown option or missing value"));
            }
            (fixture, tail) => {
                fixtures.push(fixture.into());
                rest = tail;
            }
        }
    }
    let out = out.ok_or_else(|| "--out <dir> is required".to_owned())?;
    if fixtures.is_empty() {
        return Err("at least one fixture is required".to_owned());
    }
    Ok(oracle_run::Request {
        manifest: manifest.to_path_buf(),
        oracle_dir: oracle_dir.to_path_buf(),
        adapter,
        fixtures,
        out,
        options,
        tool: oracle_run::Tool {
            commit: git(&["rev-parse", "HEAD"]),
            dirty: git(&["status", "--porcelain"]).map(|s| !s.is_empty()),
        },
        anonymous,
    })
}

fn oracle_run(manifest: &Path, oracle_dir: &Path, rest: &[String]) -> ExitCode {
    let request = match oracle_run_request(manifest, oracle_dir, rest) {
        Ok(request) => request,
        Err(message) => {
            eprintln!("loom-parity-cli: {message}");
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    let report = match oracle_run::run(&request) {
        Ok(report) => report,
        Err(message) => {
            eprintln!("loom-parity-cli: {message}");
            return ExitCode::FAILURE;
        }
    };
    print!("{}", report.summary_markdown());
    ExitCode::from(report.exit_code())
}

/// Parses the arguments after `oracle-compare <left> <right>`.
fn oracle_compare_request(
    left: &Path,
    right: &Path,
    rest: &[String],
) -> Result<oracle_compare::Request, String> {
    let mut out = None;
    let mut register = None;
    let mut fixture = None;
    let mut rest = rest;
    while let [flag, value, tail @ ..] = rest {
        match flag.as_str() {
            "--out" => out = Some(value.into()),
            "--register" => register = Some(value.into()),
            "--fixture" => fixture = Some(value.clone()),
            _ => return Err(format!("{flag}: unknown option")),
        }
        rest = tail;
    }
    if !rest.is_empty() {
        return Err(format!("unexpected argument {}", rest.join(" ")));
    }
    Ok(oracle_compare::Request {
        left: left.to_path_buf(),
        right: right.to_path_buf(),
        register,
        fixture,
        out: out.ok_or_else(|| "--out <dir> is required".to_owned())?,
    })
}

fn oracle_compare(left: &Path, right: &Path, rest: &[String]) -> ExitCode {
    let request = match oracle_compare_request(left, right, rest) {
        Ok(request) => request,
        Err(message) => {
            eprintln!("loom-parity-cli: {message}");
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match oracle_compare::run(&request) {
        Ok(report) => {
            print!("{}", report.summary_markdown());
            ExitCode::from(report.class.map_or(0, oracle::FailureClass::exit_code))
        }
        Err(message) => {
            eprintln!("loom-parity-cli: {message}");
            ExitCode::FAILURE
        }
    }
}

fn divergence_check(register: &Path) -> ExitCode {
    let report = match std::fs::read(register) {
        Ok(bytes) => divergence::Register::check(&bytes),
        Err(e) => divergence::CheckReport {
            status: "invalid",
            format: None,
            entries: None,
            accepted: None,
            problems: vec![format!("{}: {e}", register.display())],
        },
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("loom-parity-cli: {e}");
            return ExitCode::FAILURE;
        }
    }
    for problem in &report.problems {
        eprintln!("problem: {problem}");
    }
    if report.problems.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let [cmd, manifest, oracle_dir] = args.as_slice()
        && cmd == "oracle-verify"
    {
        return oracle_verify(Path::new(manifest), Path::new(oracle_dir));
    }
    if let [cmd, manifest, oracle_dir, rest @ ..] = args.as_slice()
        && cmd == "oracle-run"
    {
        return oracle_run(Path::new(manifest), Path::new(oracle_dir), rest);
    }
    if let [cmd, left, right, rest @ ..] = args.as_slice()
        && cmd == "oracle-compare"
    {
        return oracle_compare(Path::new(left), Path::new(right), rest);
    }
    if let [cmd, register] = args.as_slice()
        && cmd == "divergence-check"
    {
        return divergence_check(Path::new(register));
    }
    let result = match args.as_slice() {
        [cmd, extract] if cmd == "load-statdesc" => load_statdesc(Path::new(extract)),
        [cmd, extract, flag, out] if cmd == "round-trip" && flag == "--out" => {
            round_trip(Path::new(extract), Path::new(out))
        }
        [cmd, extract, pob, flag, out] if cmd == "render-check" && flag == "--out" => {
            render_check(Path::new(extract), Path::new(pob), Path::new(out))
        }
        [cmd, builds, extract, flag, out, pob @ ..]
            if cmd == "resolve-corpus" && flag == "--out" =>
        {
            resolve_corpus(Path::new(builds), Path::new(extract), Path::new(out), pob)
        }
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(message) => {
            eprintln!("loom-parity-cli: {message}");
            ExitCode::FAILURE
        }
    }
}
