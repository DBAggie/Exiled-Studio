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

mod build_round_trip;
mod bundle;
mod bundle_equal;
mod calc_survey;
mod categorize;
mod data_parity;
mod divergence;
mod dto_hashes;
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
mod poe1;
mod poe2;
mod properties;
mod render_check;
mod resolve_corpus;
mod round_trip;

use std::path::Path;
use std::process::ExitCode;

use serde::Serialize;

const USAGE: &str = "usage:
  loom-parity-cli bundle <extract-dir> --out <dir>
      Convert a `loom-extractor extract` output (PoE1 or PoE2) into a data
      bundle of format 2 (loom_data::bundle) in <dir>, which must not
      exist; load it back with Bundle::load; print a summary (row counts,
      hashes) as JSON. Two runs on one extract give the same bytes.
  loom-parity-cli bundle-check <bundle-dir>
      Validate a data bundle (loom_data::bundle::validate: hashes, sizes,
      unlisted files, schema, references, unique IDs, item rules, scopes);
      print the findings as JSON. Exit 0 when valid, 1 when not.
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
  loom-parity-cli bundle-equal <extract-dir> <bundle-dir>
      Check that the item pipeline's data built from the bundle (validated
      first) equal those the corpus harness reads from the extract: mod
      rows, local stats, base rows, granted skills, properties (plan
      step 6). Print a JSON report; exit 1 on any mismatch, or with a
      message when the extract or the bundle does not load or validate.
  loom-parity-cli resolve-corpus <builds-dir> <extract-dir> --out <dir> [--bundle <dir>] [--pob <tag>=<dir>]...
      Read every <Item> of the PoB builds (*.xml directly in <builds-dir>;
      subfolders are not read; identical files count once), resolve
      each modifier line in its item's scope (PoE1 extract), and
      categorize every line that is not resolved; write summary.json and
      report.json (builds by content hash, no item names) to <dir>, which
      must not exist. Each --pob names a `git archive` of a PoB tag's
      src/Data (<dir>/src/Data), the legacy-text evidence. Exit 1 while
      the Spike B gate is not met or a build is unreadable. With
      --bundle, the bases, mod rows, granted skills, properties,
      compositions, and stat-description indexes come from that data
      bundle (validated first; plan step 6) instead of the extract; the
      categories still read the extract.
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
  loom-parity-cli dto-hashes <dir>...
      Import every *.xml PoB build directly in each <dir> (in name order),
      write its v1 build document twice and read it back (loom_serialization::
      dto::v1); print each document's SHA-256 and size as JSON. CI compares
      the output of Windows and Linux. Exit 1 if a document differs on a
      second run or does not read back to the same build.
  loom-parity-cli data-parity <bundle-dir> <pob-export> --rules <file> --out <dir>
      Compare the values a calculation reads (skills and their levels,
      gems, item bases, classes, character and monster constants, game
      constants, monster levels, passive and mastery text) in a PoE1
      bundle of format 2 (validated first) with PoB v2.67.2's data, as
      tools/oracle/data/pob_data.lua exports it (plan step 3, step 2).
      Each bundle value is put into PoB's form by the conversion PoB's
      exporter made, then compared exactly. Each difference gets the first
      matching rule of <file> (docs/phase-3/data-parity-rules-poe1.json:
      patch-delta, pob-derivation, representation, with evidence) or is
      uncategorized. Write summary.json, report.json (every difference),
      and overlay.json (PoB's values for patch-delta and pob-derivation
      differences, for calc-run --data-overlay) to <dir>, which must not
      exist. Exit 0 when nothing is uncategorized and every rule matched
      exactly its count with every key pattern, 1 otherwise, 2 on a usage error, 3 when an input does not read.
  loom-parity-cli build-round-trip <input>... --out <dir> [--bundle <dir>] [--anonymous]
                  [--expect-import-error <file name>]...
      Run every build through import (A), resolution with the bundle if
      given (validated first), the v1 JSON document (B), and PoB XML export
      and re-import (C, resolved again; PoE1 builds only), and check
      (a) export(B) xml_equal to the input, (b) A, B, and C equal with
      resolutions, (c) the same JSON bytes on a second run, (d) every
      item's source lines in equal its lines out; PoB input also runs the
      unknown-content mutation set (plan step 8). An input is a folder
      (every *.xml directly in it, in name order), a PoB build *.xml, or a
      v1 build document *.json; identical files count once. A file named
      by --expect-import-error must not import, and then passes.
      A name that is no input's file name stops the run (exit 3).
      --anonymous names builds by SHA-256. Write summary.json (scrubbed)
      and report.json (failures) to <dir>, which must not exist. Exit 0
      when every build passes, 1 when one fails (or no build was given),
      2 on a usage error, 3 when the run cannot start (an input or the
      bundle does not read or validate, or <dir> exists).
  loom-parity-cli calc-survey <input>... --bundle <dir> --scope <file> --data-rules <file> --out <dir>
                  [--anonymous]
      Decide which builds are in the calc slice's scope from game data
      (Phase 3 plan, step 3): import each PoB build (a folder: every *.xml
      directly in it, in name order; or a *.xml file; identical files count
      once), resolve it with the bundle (format 2, PoE1, validated first),
      find its main skill as PoB does, and apply the archetype rules, the
      tree-version rule, unknown passives, and the excluded mechanics of
      the scope file <file> (docs/phase-3/calc-scope-poe1.json); the
      data-parity rules (docs/phase-3/data-parity-rules-poe1.json) name the
      passives the bundle does not fully carry. Write summary.json (per
      build: main skill, verdict and reasons; candidate skills per kind,
      ranked; the chosen skills' in-scope builds) and report.json (each
      build's mechanic inventory; how many builds reach each stat,
      support, skill, and config input) to <dir>, which must not exist.
      --anonymous names builds by SHA-256. Exit 0 when every chosen skill
      has the scope file's minimum of in-scope builds (or none is chosen),
      1 when one has fewer, 2 on a usage error, 3 when the run cannot
      start.
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

/// A bundle in a folder, for the validator.
struct DirSource<'d>(&'d Path);

impl loom_data::bundle::BundleSource for DirSource<'_> {
    fn list(&self) -> Result<Vec<String>, loom_data::bundle::SourceError> {
        fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                // A link could point out of the bundle: refused.
                if entry.file_type()?.is_symlink() {
                    return Err(std::io::Error::other(format!(
                        "{}: a symbolic link",
                        path.display()
                    )));
                }
                if path.is_dir() {
                    walk(root, &path, out)?;
                } else if let Ok(rel) = path.strip_prefix(root) {
                    out.push(rel.to_string_lossy().replace('\\', "/"));
                }
            }
            Ok(())
        }
        let mut out = Vec::new();
        walk(self.0, self.0, &mut out).map_err(|e| loom_data::bundle::SourceError::Unreadable {
            path: self.0.display().to_string(),
            message: e.to_string(),
        })?;
        out.sort();
        Ok(out)
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, loom_data::bundle::SourceError> {
        // The loader refuses paths that leave the bundle before reading;
        // this source refuses them again.
        if path.split('/').any(|s| s.is_empty() || s == "..") || path.contains(['\\', ':']) {
            return Err(loom_data::bundle::SourceError::NotFound(path.to_owned()));
        }
        let full = self.0.join(path);
        match std::fs::read(&full) {
            Ok(b) => Ok(b),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(loom_data::bundle::SourceError::NotFound(path.to_owned()))
            }
            Err(e) => Err(loom_data::bundle::SourceError::Unreadable {
                path: path.to_owned(),
                message: e.to_string(),
            }),
        }
    }
}

/// `bundle-check`: validates a bundle folder, prints the findings.
fn bundle_check(dir: &Path) -> Result<bool, String> {
    let report = loom_data::bundle::validate::validate(&DirSource(dir));
    let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    println!("{json}");
    Ok(report.is_valid())
}

/// `bundle`: converts, writes, loads back, prints the summary.
fn bundle_cmd(extract: &Path, out: &Path) -> Result<bool, String> {
    if out.exists() {
        return Err(format!("{}: already exists", out.display()));
    }
    let read = reader(extract);
    let loaded = extract::load(&read)?;
    let (files, summary) = bundle::convert(&read, &loaded)?;
    // Written to a sibling folder and renamed at the end, so a failed run
    // leaves no partial <dir>.
    let partial = out.with_extension("partial");
    if partial.exists() {
        return Err(format!(
            "{}: already exists (an earlier run?)",
            partial.display()
        ));
    }
    let mut source = loom_data::bundle::MemorySource::new();
    for (path, bytes) in &files {
        let target = partial.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(&target, bytes).map_err(|e| format!("{}: {e}", target.display()))?;
        source.insert(path.clone(), bytes.clone());
    }
    let back = loom_data::bundle::Bundle::load(&source)
        .map_err(|e| format!("the written bundle does not load: {} ({e})", e.code()))?;
    std::fs::rename(&partial, out).map_err(|e| format!("{}: {e}", out.display()))?;
    let json = serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?;
    println!("{json}");
    Ok(back.game.name() == summary.game)
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

/// `--pob <tag>=<dir>` sources, and the `--bundle <dir>`, if given.
fn corpus_args(args: &[String]) -> Result<(Vec<categorize::PobText>, Option<&Path>), String> {
    let mut out = Vec::new();
    let mut bundle = None;
    let mut rest = args;
    while let [flag, value, tail @ ..] = rest {
        if flag == "--bundle" && bundle.is_none() {
            bundle = Some(Path::new(value.as_str()));
            rest = tail;
            continue;
        }
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
    Ok((out, bundle))
}

/// Loads a bundle folder after the validator accepts it.
fn load_bundle(dir: &Path) -> Result<loom_data::bundle::Bundle, String> {
    let source = DirSource(dir);
    let report = loom_data::bundle::validate::validate(&source);
    if !report.is_valid() {
        return Err(format!(
            "{}: the bundle is not valid ({})",
            dir.display(),
            report.codes().join(", ")
        ));
    }
    loom_data::bundle::Bundle::load(&source)
        .map_err(|e| format!("{}: {} ({e})", dir.display(), e.code()))
}

/// `bundle-equal`: the pipeline's data from a bundle against the
/// extract's (`bundle_equal`).
fn bundle_equal(extract: &Path, bundle_dir: &Path) -> Result<bool, String> {
    let read = reader(extract);
    let loaded = extract::load(&read)?;
    let bases = resolve_corpus::Bases::load(&read)?;
    let mods = mod_filter::load(&read)?;
    let game = categorize::GameText::load(&loaded.set, &read, Vec::new())?;
    let bundle = load_bundle(bundle_dir)?;
    let data = loom_parser::import::BundleData::new(&bundle);
    let report = bundle_equal::compare(&bases, &mods, &game.compositions, &data);
    let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    println!("{json}");
    Ok(report.equal())
}

fn resolve_corpus(
    builds_dir: &Path,
    extract: &Path,
    out: &Path,
    pob: &[String],
) -> Result<bool, String> {
    let (pob, bundle_dir) = corpus_args(pob)?;
    let read = reader(extract);
    let loaded = extract::load(&read)?;
    let (lookup, lookup_report) = lookup::TableLookup::load(&read, &loaded)?;
    // `--bundle`: the pipeline's data come from the bundle (plan step 6);
    // the extract still gives the categories' evidence.
    let bundle = bundle_dir.map(load_bundle).transpose()?;
    let data = bundle.as_ref().map(loom_parser::import::BundleData::new);
    let (bases, extract_mods) = match &data {
        Some(d) => (resolve_corpus::Bases::from_bundle(d), None),
        None => (
            resolve_corpus::Bases::load(&read)?,
            Some(mod_filter::load(&read)?),
        ),
    };
    let mods = match (&data, &extract_mods) {
        (Some(d), _) => d.mods(),
        (None, Some(m)) => m,
        (None, None) => return Err("no mod rows".to_owned()),
    };
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
    let (summary, report, check) = resolve_corpus::run(
        &loaded,
        &lookup,
        lookup_report,
        &bases,
        mods,
        &game,
        &builds,
        tool,
        data.as_ref(),
    )?;
    write_out(out, &summary, &report)?;
    let json =
        serde_json::to_string_pretty(&(&summary.rate, &summary.gate)).map_err(|e| e.to_string())?;
    println!("{json}");
    // `--bundle`: the core's `resolve_build` against the harness, per line.
    let check_equal = match &check {
        Some(c) => {
            let path = out.join("resolve-build-check.json");
            let mut text = serde_json::to_string_pretty(c).map_err(|e| e.to_string())?;
            text.push('\n');
            std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
            println!(
                "resolve_build: {} of {} modifier lines equal the harness",
                c.equal, c.lines
            );
            c.differences.is_empty()
        }
        None => true,
    };
    Ok(summary.gate.met && summary.unreadable_builds.is_empty() && check_equal)
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

/// `data-parity <bundle-dir> <pob-export> --rules <file> --out <dir>`.
fn data_parity_cmd(args: &[String]) -> ExitCode {
    let mut positional = Vec::new();
    let (mut rules, mut out) = (None, None);
    let mut rest = args;
    while let [arg, tail @ ..] = rest {
        match (arg.as_str(), tail) {
            ("--rules", [value, tail @ ..]) if rules.is_none() => {
                rules = Some(value.clone());
                rest = tail;
            }
            ("--out", [value, tail @ ..]) if out.is_none() => {
                out = Some(value.clone());
                rest = tail;
            }
            (flag, _) if flag.starts_with("--") => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
            (value, tail) => {
                positional.push(value.to_owned());
                rest = tail;
            }
        }
    }
    let ([bundle_dir, export], Some(rules), Some(out)) = (positional.as_slice(), rules, out) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let out = Path::new(&out);
    let start = || -> Result<data_parity::Outputs, String> {
        if out.exists() {
            return Err(format!("{}: already exists", out.display()));
        }
        let bundle_dir = Path::new(bundle_dir);
        let bundle = load_bundle(bundle_dir)?;
        let read =
            |path: &Path| std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()));
        let manifest = read(&bundle_dir.join("bundle.json"))?;
        let export = read(Path::new(export))?;
        let rules = read(Path::new(&rules))?;
        data_parity::run(&data_parity::Request {
            bundle: &bundle,
            bundle_manifest: &manifest,
            export: &export,
            rules: &rules,
            passive_scope: data_parity::compare::PASSIVE_SCOPE,
            tool: resolve_corpus::Tool {
                commit: git(&["rev-parse", "HEAD"]),
                dirty: git(&["status", "--porcelain"]).map(|s| !s.is_empty()),
            },
        })
    };
    let outputs = match start() {
        Ok(outputs) => outputs,
        Err(message) => {
            eprintln!("data-parity: {message}");
            return ExitCode::from(3);
        }
    };
    let written = write_out(out, &outputs.summary, &outputs.report).and_then(|()| {
        let mut text = serde_json::to_string_pretty(&outputs.overlay).map_err(|e| e.to_string())?;
        text.push('\n');
        let path = out.join("overlay.json");
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
    });
    if let Err(message) = written {
        eprintln!("data-parity: {message}");
        return ExitCode::from(3);
    }
    let summary = &outputs.summary;
    println!(
        "data-parity: {} differences ({}), {} overlay rows, {} failed rules",
        outputs.report.differences.len(),
        summary
            .categories
            .iter()
            .map(|(k, v)| format!("{k} {v}"))
            .collect::<Vec<_>>()
            .join(", "),
        summary.overlay_rows,
        summary.failed_rules.len()
    );
    for rule in summary
        .rules
        .iter()
        .filter(|r| summary.failed_rules.contains(&r.id))
    {
        eprintln!(
            "failed rule {}: matched {} of {}, stale keys {:?}",
            rule.id, rule.matched, rule.expected, rule.stale_keys
        );
    }
    if summary.pass {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn dto_hashes(dirs: &[String]) -> Result<bool, String> {
    let report = dto_hashes::run(dirs)?;
    let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    println!("{json}");
    for failure in &report.failures {
        eprintln!("failure: {failure}");
    }
    Ok(report.failures.is_empty())
}

/// `build-round-trip`'s arguments.
#[derive(Debug, Default, PartialEq, Eq)]
struct RoundTripArgs {
    inputs: Vec<String>,
    bundle: Option<String>,
    out: Option<String>,
    anonymous: bool,
    expect_import_error: std::collections::BTreeSet<String>,
}

/// Parses the arguments after `build-round-trip`; `None` is a usage error.
fn round_trip_args(args: &[String]) -> Option<RoundTripArgs> {
    let mut out = RoundTripArgs::default();
    let mut rest = args;
    while let [arg, tail @ ..] = rest {
        rest = tail;
        match arg.as_str() {
            "--anonymous" => out.anonymous = true,
            "--bundle" | "--out" | "--expect-import-error" => {
                let [value, tail @ ..] = rest else {
                    return None;
                };
                rest = tail;
                match arg.as_str() {
                    "--bundle" if out.bundle.is_none() => out.bundle = Some(value.clone()),
                    "--out" if out.out.is_none() => out.out = Some(value.clone()),
                    "--expect-import-error" => {
                        out.expect_import_error.insert(value.clone());
                    }
                    _ => return None,
                }
            }
            flag if flag.starts_with("--") => return None,
            input => out.inputs.push(input.to_owned()),
        }
    }
    (!out.inputs.is_empty() && out.out.is_some()).then_some(out)
}

/// The inputs: each `*.xml` directly in a folder (in name order), or a
/// file (`*.xml` or `*.json`).
fn round_trip_inputs(args: &[String]) -> Result<Vec<build_round_trip::Input>, String> {
    use build_round_trip::{Format, Input};
    let format_of = |path: &Path| match path.extension().and_then(|e| e.to_str()) {
        Some(e) if e.eq_ignore_ascii_case("xml") => Some(Format::PobXml),
        Some(e) if e.eq_ignore_ascii_case("json") => Some(Format::BuildDocument),
        _ => None,
    };
    let mut inputs = Vec::new();
    for arg in args {
        let path = Path::new(arg);
        let mut files = Vec::new();
        if path.is_dir() {
            for entry in std::fs::read_dir(path).map_err(|e| format!("{arg}: {e}"))? {
                let p = entry.map_err(|e| format!("{arg}: {e}"))?.path();
                if p.is_file() && format_of(&p) == Some(Format::PobXml) {
                    files.push(p);
                }
            }
            files.sort();
        } else {
            files.push(path.to_path_buf());
        }
        for p in files {
            let format =
                format_of(&p).ok_or_else(|| format!("{}: not *.xml or *.json", p.display()))?;
            let file = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let shown = if path.is_dir() {
                format!("{}/{file}", arg.trim_end_matches(['/', '\\']))
            } else {
                arg.clone()
            };
            inputs.push(Input {
                path: shown.replace('\\', "/"),
                file,
                format,
                bytes: std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?,
            });
        }
    }
    Ok(inputs)
}

/// `build-round-trip` (module `build_round_trip`). Exit 0 when every build
/// passes, 1 when one fails, 2 on a usage error, 3 when the run cannot
/// start (an input or the bundle does not read or validate, or `--out`
/// exists).
fn build_round_trip_cmd(args: &[String]) -> ExitCode {
    let Some(args) = round_trip_args(args) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match build_round_trip_run(&args) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(message) => {
            eprintln!("loom-parity-cli: {message}");
            ExitCode::from(3)
        }
    }
}

fn build_round_trip_run(args: &RoundTripArgs) -> Result<bool, String> {
    let out = Path::new(args.out.as_deref().unwrap_or_default());
    if out.exists() {
        return Err(format!("{}: already exists", out.display()));
    }
    let inputs = round_trip_inputs(&args.inputs)?;
    if let Some(missing) = args
        .expect_import_error
        .iter()
        .find(|name| !inputs.iter().any(|i| &i.file == *name))
    {
        return Err(format!("--expect-import-error {missing}: no such input"));
    }
    let bundle = args
        .bundle
        .as_deref()
        .map(|d| load_bundle(Path::new(d)))
        .transpose()?;
    let data = bundle.as_ref().map(loom_parser::import::BundleData::new);
    let resolver = data
        .as_ref()
        .map(loom_parser::import::Resolver::new)
        .transpose()
        .map_err(|e| format!("the bundle's pipeline does not build: {} ({e})", e.code()))?;
    let options = build_round_trip::Options {
        data: resolver
            .as_ref()
            .zip(data.as_ref())
            .map(|(resolver, data)| build_round_trip::Data { resolver, data }),
        anonymous: args.anonymous,
        expect_import_error: &args.expect_import_error,
        tool: resolve_corpus::Tool {
            commit: git(&["rev-parse", "HEAD"]),
            dirty: git(&["status", "--porcelain"]).map(|s| !s.is_empty()),
        },
    };
    let (summary, report) = build_round_trip::run(&inputs, &options);
    write_out(out, &summary, &report)?;
    println!(
        "build-round-trip: {} builds, {} passed, {} failed ({} duplicates skipped)",
        summary.builds, summary.passed, summary.failed, summary.duplicates
    );
    for build in &report.builds {
        for failure in &build.failures {
            eprintln!("{}: {failure}", build.name);
        }
    }
    Ok(summary.failed == 0 && summary.builds > 0)
}

/// `calc-survey`'s arguments.
#[derive(Debug, Default, PartialEq, Eq)]
struct SurveyArgs {
    inputs: Vec<String>,
    bundle: Option<String>,
    scope: Option<String>,
    data_rules: Option<String>,
    out: Option<String>,
    anonymous: bool,
}

/// Parses the arguments after `calc-survey`; `None` is a usage error.
fn survey_args(args: &[String]) -> Option<SurveyArgs> {
    let mut out = SurveyArgs::default();
    let mut rest = args;
    while let [arg, tail @ ..] = rest {
        rest = tail;
        let slot = match arg.as_str() {
            "--anonymous" => {
                out.anonymous = true;
                continue;
            }
            "--bundle" => &mut out.bundle,
            "--scope" => &mut out.scope,
            "--data-rules" => &mut out.data_rules,
            "--out" => &mut out.out,
            flag if flag.starts_with("--") => return None,
            input => {
                out.inputs.push(input.to_owned());
                continue;
            }
        };
        let [value, tail @ ..] = rest else {
            return None;
        };
        if slot.replace(value.clone()).is_some() {
            return None;
        }
        rest = tail;
    }
    let complete = !out.inputs.is_empty()
        && out.bundle.is_some()
        && out.scope.is_some()
        && out.data_rules.is_some()
        && out.out.is_some();
    complete.then_some(out)
}

/// `calc-survey` (module `calc_survey`).
fn calc_survey_cmd(args: &[String]) -> ExitCode {
    let Some(args) = survey_args(args) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match calc_survey_run(&args) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(message) => {
            eprintln!("calc-survey: {message}");
            ExitCode::from(3)
        }
    }
}

fn calc_survey_run(args: &SurveyArgs) -> Result<bool, String> {
    let path = |p: &Option<String>| Path::new(p.as_deref().unwrap_or_default()).to_path_buf();
    let read = |p: &Path| std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()));
    let sha = |b: &[u8]| {
        use sha2::Digest as _;
        sha2::Sha256::digest(b)
            .iter()
            .map(|x| format!("{x:02x}"))
            .collect::<String>()
    };
    let out = path(&args.out);
    if out.exists() {
        return Err(format!("{}: already exists", out.display()));
    }
    let scope_bytes = read(&path(&args.scope))?;
    let scope = calc_survey::rules::Scope::read(&scope_bytes)?;
    let rules_bytes = read(&path(&args.data_rules))?;
    let not_carried = calc_survey::not_carried_passives(&rules_bytes, scope.steam_build)?;
    let inputs: Vec<calc_survey::Input> = round_trip_inputs(&args.inputs)?
        .into_iter()
        .map(|i| match i.format {
            build_round_trip::Format::PobXml => Ok(calc_survey::Input {
                path: i.path,
                bytes: i.bytes,
            }),
            build_round_trip::Format::BuildDocument => {
                Err(format!("{}: calc-survey reads PoB builds (*.xml)", i.path))
            }
        })
        .collect::<Result<_, _>>()?;
    let bundle = load_bundle(&path(&args.bundle))?;
    let m = &bundle.manifest;
    if m.game != "poe1" || m.source.steam_build != Some(scope.steam_build) {
        return Err(format!(
            "the bundle is {} Steam build {:?}; the scope file is for poe1 {}",
            m.game, m.source.steam_build, scope.steam_build
        ));
    }
    let tables = calc_survey::Tables::new(&bundle)?;
    let data = loom_parser::import::BundleData::new(&bundle);
    let resolver = loom_parser::import::Resolver::new(&data)
        .map_err(|e| format!("the bundle's pipeline does not build: {} ({e})", e.code()))?;
    let request = calc_survey::Request {
        resolver: &resolver,
        data: &data,
        tables: &tables,
        scope: &scope,
        not_carried: &not_carried,
        anonymous: args.anonymous,
        inputs: calc_survey::Inputs {
            tool: resolve_corpus::Tool {
                commit: git(&["rev-parse", "HEAD"]),
                dirty: git(&["status", "--porcelain"]).map(|s| !s.is_empty()),
            },
            bundle: resolve_corpus::BundleInputs {
                game: m.game.clone(),
                steam_build: m.source.steam_build,
                index_sha256: m.source.index_sha256.clone(),
            },
            scope_sha256: sha(&scope_bytes),
            data_rules_sha256: sha(&rules_bytes),
            anonymous: args.anonymous,
        },
    };
    let (summary, report) = calc_survey::run(&inputs, &request);
    write_out(&out, &summary, &report)?;
    println!(
        "calc-survey: {} builds, {} in scope ({} duplicates skipped)",
        summary.builds, summary.in_scope, summary.duplicates
    );
    for (kind, list) in &summary.candidates {
        for c in list.iter().take(3) {
            println!(
                "  {kind} {}: {} in scope of {} archetype builds",
                c.skill,
                c.in_scope.len(),
                c.archetype_builds
            );
        }
    }
    for (kind, c) in &summary.chosen {
        println!(
            "  chosen {kind} {}: {} in scope (minimum {}){}",
            c.skill,
            c.in_scope.len(),
            summary.scope.minimum_per_skill,
            if c.meets_minimum {
                ""
            } else {
                ", below the minimum"
            }
        );
    }
    Ok(summary.pass)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let [cmd, rest @ ..] = args.as_slice()
        && cmd == "calc-survey"
    {
        return calc_survey_cmd(rest);
    }
    if let [cmd, rest @ ..] = args.as_slice()
        && cmd == "build-round-trip"
    {
        return build_round_trip_cmd(rest);
    }
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
    if let [cmd, rest @ ..] = args.as_slice()
        && cmd == "data-parity"
    {
        return data_parity_cmd(rest);
    }
    if let [cmd, register] = args.as_slice()
        && cmd == "divergence-check"
    {
        return divergence_check(Path::new(register));
    }
    let result = match args.as_slice() {
        [cmd, extract, flag, out] if cmd == "bundle" && flag == "--out" => {
            bundle_cmd(Path::new(extract), Path::new(out))
        }
        [cmd, dir] if cmd == "bundle-check" => bundle_check(Path::new(dir)),
        [cmd, dirs @ ..] if cmd == "dto-hashes" && !dirs.is_empty() => dto_hashes(dirs),
        [cmd, extract] if cmd == "load-statdesc" => load_statdesc(Path::new(extract)),
        [cmd, extract, flag, out] if cmd == "round-trip" && flag == "--out" => {
            round_trip(Path::new(extract), Path::new(out))
        }
        [cmd, extract, pob, flag, out] if cmd == "render-check" && flag == "--out" => {
            render_check(Path::new(extract), Path::new(pob), Path::new(out))
        }
        [cmd, extract, bundle] if cmd == "bundle-equal" => {
            bundle_equal(Path::new(extract), Path::new(bundle))
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
