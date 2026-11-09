//! Oracle and corpus harnesses, maintainer-run. The only workspace code
//! besides the extractor that reads disk: core crates take bytes
//! (ADR-016), and every extract- or corpus-backed check is a subcommand
//! here (Spike B plan, `dev/active/spike-b-text-stat-resolution/`).
//!
//! The PoB oracle harness (drives legacy `HeadlessWrapper.lua` on pinned
//! LuaJIT at a pinned public PoB tag, ADR-006) comes with Spike C.

mod extract;
mod load_statdesc;
mod lookup;
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
  loom-parity-cli resolve-corpus <builds-dir> <extract-dir> --out <dir>
      Read every <Item> of the PoB builds (*.xml) in <builds-dir>, resolve
      each modifier line in its item's scope (PoE1 extract); write
      summary.json and report.json (builds by content hash, no item names)
      to <dir>, which must not exist. Exit 1 while the resolution rate of
      non-unique lines is below 98% or a build is unreadable.";

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
    let (lookup, lookup_report) = lookup::TableLookup::load(&read)?;
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

fn resolve_corpus(builds_dir: &Path, extract: &Path, out: &Path) -> Result<bool, String> {
    let read = reader(extract);
    let loaded = extract::load(&read)?;
    let (lookup, lookup_report) = lookup::TableLookup::load(&read)?;
    let bases = resolve_corpus::Bases::load(&read)?;
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
    let (summary, report) =
        resolve_corpus::run(&loaded, &lookup, lookup_report, &bases, &builds, tool)?;
    write_out(out, &summary, &report)?;
    let json = serde_json::to_string_pretty(&summary.rate).map_err(|e| e.to_string())?;
    println!("{json}");
    Ok(summary.rate.meets_rate && summary.unreadable_builds.is_empty())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [cmd, extract] if cmd == "load-statdesc" => load_statdesc(Path::new(extract)),
        [cmd, extract, flag, out] if cmd == "round-trip" && flag == "--out" => {
            round_trip(Path::new(extract), Path::new(out))
        }
        [cmd, extract, pob, flag, out] if cmd == "render-check" && flag == "--out" => {
            render_check(Path::new(extract), Path::new(pob), Path::new(out))
        }
        [cmd, builds, extract, flag, out] if cmd == "resolve-corpus" && flag == "--out" => {
            resolve_corpus(Path::new(builds), Path::new(extract), Path::new(out))
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
