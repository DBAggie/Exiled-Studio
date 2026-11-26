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
mod calc_parity;
mod calc_run;
mod calc_survey;
mod categorize;
mod cluster_jewels;
mod custom_lines;
mod data_parity;
mod divergence;
mod dto_hashes;
mod extract;
mod flasks;
mod flavour;
mod grafts;
mod item_rewordings;
mod item_tags;
mod jewel_radii;
mod load_statdesc;
mod lookup;
mod minions;
mod mod_filter;
mod oracle;
mod oracle_compare;
mod oracle_fixtures;
mod oracle_output;
mod oracle_process;
mod oracle_run;
mod oracle_verify;
mod pantheons;
mod pob_skills;
mod poe1;
mod poe2;
mod properties;
mod render_check;
mod resolve_corpus;
mod round_trip;
mod tattoos;
mod timeless;
mod tree_flavour;
mod tree_layout;
mod tree_pack;
mod tree_parity;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::ExitCode;

use serde::Serialize;

const USAGE: &str = "usage:
  loom-parity-cli tree-parity layout <pob-layout.json> <bundle-dir>
  loom-parity-cli tree-parity alloc-requests <bundle-dir> <count> <seed> [<tree-version>]
  loom-parity-cli tree-parity alloc <requests.txt> <pob-alloc.json> <bundle-dir>
  loom-parity-cli tree-parity readback <build.xml> <pob-readback.json>
  loom-parity-cli tree-parity convert <requests.txt> <pob-alloc.json> <bundle-dir> <from-version>
      The passive tree against PoB (docs/tree/parity.md): PoB's side from
      tools/oracle/data/pob_tree.lua. Prints a JSON record; exit 0 when
      everything matches, 1 otherwise. alloc-requests prints seeded
      requests for pob_tree.lua alloc. convert compares converting specs
      of <from-version>'s pack (requests made on it) to the bundle's own
      tree with PoB's import of them on its latest tree.
  loom-parity-cli tree-pack <tools/tree/manifest.json> <version> <export-dir> <bundle-dir> [--accept-unread] --out <dir>
      Copy a bundle into <dir> (which must not exist) with an older tree
      version added as a tree pack (trees/<version>.json; passive tree plan,
      T4), converted from GGG's tree export at the commit pinned for the
      version (fetch it with tools/tree/acquire <version>): the lines read
      as stat IDs through the bundle's passive descriptions, the ones that
      do not kept as text. Refuses another commit or other orbits. Prints a
      JSON summary.
  loom-parity-cli timeless-pack <bundle-dir> <pob-src-dir> <pob-version> --out <dir>
      Copy a bundle into <dir> (which must not exist) with PoB's timeless
      jewel results added (poe1/timeless_luts.json and poe1/timeless/<Type>.bin;
      Phase 6 P6-10a): PoB's NodeIndexMapping.lua, its jewel types and seed
      ranges from Modules/Data.lua, and each Data/TimelessJewelData zip
      inflated byte for byte. Needs the bundle's poe1/timeless.json (the
      extract's tables). Prints a JSON summary.
  loom-parity-cli tree-layout <tools/tree/manifest.json> <export-dir> <bundle-dir> --out <dir>
      Copy a bundle into <dir> (which must not exist) with the passive
      tree's layout constants (data/passive_tree_layout.json) and sprites
      (data/passive_tree_sprites.json, the sheets in trees/assets/) added,
      read from GGG's tree export at the pinned commit (ADR-005 Amendment 2;
      fetch it with tools/tree/acquire). Refuses another commit, orbits
      that differ from the bundle's, or a README angle table that differs
      from the rule; validates before and after; prints a summary.
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
  loom-parity-cli pob-skills <pob-data.json> --out <file.rs>
      Write PoB v2.67.2's hand-written skill data (base flags, parts, base
      mods, skill function names) from the PoB data export (format 10) as
      the Rust table crates/loom-calc/src/poe1/pob_skills.rs (Phase 6 plan,
      decision 5); from format 13, each skill's minion lists. Exit 0 when
      written.
  loom-parity-cli pob-minions <pob-data.json> --out <file.rs>
      Write what PoB v2.67.2's minion templates (Export/Minions/*.txt) add
      to each game variety (the variety, #limit, #hostile, extra skills,
      #mod extras by line) from the PoB data export (format 13) as the Rust
      table crates/loom-calc/src/poe1/pob_minions.rs (P6-7b1). Exit 0 when
      written.
  loom-parity-cli custom-lines <bundle-dir> <pob-parse.json> --out <dir>
      Read every custom modifier line of <pob-parse.json> (the output of
      tools/oracle/data/pob_parse_lines.lua: each line with PoB v2.67.2's
      parse) as a build's custom line is read (stat descriptions, then the
      curated patterns, then the note rule; ADR-015 Amendment 2) and compare
      with PoB's parse; write report.json to <dir>, which must not exist
      (docs/phase-3/custom-lines.md). Exit 1 when a note is a line PoB
      applies, or a pattern's effect differs from PoB's mods.
  loom-parity-cli item-rewordings lines <table.json> --out <lines.txt> [<extra.txt>...]
      Write, one per line, each pair of the item rewording table
      (tools/rewordings/items.json) in its old and current wording at
      sample numbers, then the lines of each <extra.txt> ({...} prefixes
      removed, each once), for tools/oracle/data/pob_parse_lines.lua
      (ADR-015 Amendment 3; docs/phase-6/item-rewordings.md).
  loom-parity-cli item-rewordings check <bundle-dir> <table.json> <pob-parse.json> --out <dir>
      Compare the table and the further lines with PoB v2.67.2's parse;
      write report.json to <dir>, which must not exist. Exit 1 when an
      accepted pair's two wordings parse differently in PoB, its current
      wording reads through no description, or an item note is a line PoB
      applies.
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
  loom-parity-cli calc-run <input>... --bundle <dir> --out <dir> [--anonymous] [--data-overlay <file>]
                  [--scope <file>]
      Calculate every build (Phase 3 plan, step 5): import (a folder: every
      *.xml directly in it, in name order; a *.xml file; or a v1 build
      document *.json), resolve with the bundle (validated first), and run
      the engine; write each result as the calc result document v1
      (documents/<id>.json), report.json (per build: outcome, error code,
      document SHA-256, state counts), summary.json (counts of states,
      reasons, failure codes; calculation time), and timing.json (time per
      build) to <dir>, which must not exist. A build's id is its file
      name, or its SHA-256 with --anonymous, as oracle-run names fixtures.
      --data-overlay reads data-parity's overlay.json for this bundle and
      calculates with PoB's values for its rows (for calc-parity
      --overlay-run). --scope reads calc-survey's scope file for this
      bundle's Steam build; its tree_version is the data's passive tree
      version, against which the engine checks each build's (without it,
      a build that allocates passives is partial, unchecked-tree-version).
      Exit 0 when every build has a document, 1 when one failed, 2 on a
      usage error, 3 when the run cannot start.
  loom-parity-cli calc-parity <oracle-run-dir> <calc-run-dir> --slice <file> --out <dir>
                  [--register <file>] [--overlay-run <dir>] [--manifest <file>]
      Compare the successor's documents with the oracle's outputs, paired
      by id, per slice field (docs/phase-3/slice-fields.json): complete
      fields with their rule; partial, indeterminate, and not computed
      fields listed, not compared; the ten breakdowns term by term at each
      line's printed decimals (not when PoB's CALCS and main views
      differ). Differences are classified with the divergence register; a
      data-version entry covers one only when the --overlay-run (calc-run
      --data-overlay) matches the oracle there. --manifest: each mechanic
      fixture's targets must match or be registered, and each must be in
      both runs. The overlay run must hold the same builds (ids, SHA-256)
      and tool commit; a build whose bytes differ between the two runs is
      not compared (ORACLE_PROTOCOL). Write report.json and summary.md to
      <dir>, which must not exist. Exit: the class of a build without
      output on a side (10 to 18; 17 for a breakdown line the reader does
      not know); else 20 for an unregistered difference; else 5 when a
      build is incomplete (no complete field, or a complete field's
      breakdown not compared): not a pass; else 19 when every difference
      is registered; else 0. 2 on a usage error, 3 when an input does not
      read.
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

/// `custom-lines`: custom modifier lines against PoB's parse
/// ([`custom_lines`]).
fn custom_lines_cmd(bundle_dir: &Path, pob: &Path, out: &Path) -> Result<bool, String> {
    if out.exists() {
        return Err(format!("{}: already exists", out.display()));
    }
    let bytes = std::fs::read(pob).map_err(|e| format!("{}: {e}", pob.display()))?;
    let lines: Vec<custom_lines::PobLine> =
        serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", pob.display()))?;
    let bundle = load_bundle(bundle_dir)?;
    let data = loom_parser::import::BundleData::new(&bundle);
    let resolver = loom_parser::import::Resolver::new(&data).map_err(|e| e.to_string())?;
    let report = custom_lines::compare(&resolver, &lines);
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())? + "\n";
    std::fs::write(out.join("report.json"), json).map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::to_string(&report.counts).map_err(|e| e.to_string())?
    );
    Ok(report.passed())
}

/// The item rewording table at `path`.
fn rewording_table(path: &Path) -> Result<loom_parser::import::poe1::reword::Table, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    loom_parser::import::poe1::reword::Table::parse(&text)
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// `item-rewordings lines` ([`item_rewordings`]).
fn item_rewordings_lines(table: &Path, out: &Path, extra: &[String]) -> Result<bool, String> {
    let table = rewording_table(table)?;
    let mut more = Vec::new();
    for path in extra {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        more.extend(text.lines().map(str::to_owned));
    }
    let lines = item_rewordings::lines(&table, &more);
    let text: String = lines.iter().map(|l| format!("{l}\n")).collect();
    std::fs::write(out, text).map_err(|e| format!("{}: {e}", out.display()))?;
    println!("{} lines", lines.len());
    Ok(true)
}

/// `item-rewordings check` ([`item_rewordings`]).
fn item_rewordings_check(
    bundle_dir: &Path,
    table: &Path,
    pob: &Path,
    out: &Path,
) -> Result<bool, String> {
    if out.exists() {
        return Err(format!("{}: already exists", out.display()));
    }
    let table = rewording_table(table)?;
    let bytes = std::fs::read(pob).map_err(|e| format!("{}: {e}", pob.display()))?;
    let lines: Vec<custom_lines::PobLine> =
        serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", pob.display()))?;
    let bundle = load_bundle(bundle_dir)?;
    let data = loom_parser::import::BundleData::new(&bundle);
    let resolver = loom_parser::import::Resolver::new(&data).map_err(|e| e.to_string())?;
    let keystone = |t: &str| {
        bundle.passives.as_ref().is_some_and(|p| {
            p.passives
                .iter()
                .any(|n| n.keystone && n.name.eq_ignore_ascii_case(t))
        })
    };
    let report = item_rewordings::check(&resolver, &table, &lines, keystone);
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())? + "\n";
    std::fs::write(out.join("report.json"), json).map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::to_string(&report.counts).map_err(|e| e.to_string())?
    );
    Ok(report.passed())
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
    let builds = corpus_builds(builds_dir)?;
    let (summary, report, check) = resolve_corpus::run(
        &loaded,
        &lookup,
        lookup_report,
        &bases,
        mods,
        &game,
        &builds,
        tool(),
        data.as_ref(),
    )?;
    write_out(out, &summary, &report)?;
    let json =
        serde_json::to_string_pretty(&(&summary.rate, &summary.gate)).map_err(|e| e.to_string())?;
    println!("{json}");
    let check_equal = match &check {
        Some(check) => write_build_check(out, check)?,
        None => true,
    };
    Ok(summary.gate.met && summary.unreadable_builds.is_empty() && check_equal)
}

/// This tool's commit, and whether its source tree has changes.
fn tool() -> resolve_corpus::Tool {
    resolve_corpus::Tool {
        commit: git(&["rev-parse", "HEAD"]),
        dirty: git(&["status", "--porcelain"]).map(|s| !s.is_empty()),
    }
}

/// Every `*.xml` build directly in `dir`.
fn corpus_builds(dir: &Path) -> Result<Vec<resolve_corpus::Build>, String> {
    let mut builds = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let is_xml = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("xml"));
        if is_xml && path.is_file() {
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            builds.push(resolve_corpus::Build::new(bytes));
        }
    }
    Ok(builds)
}

/// `--bundle`: the core's `resolve_build` against the harness, per line.
/// Writes `resolve-build-check.json`; whether every line is equal.
fn write_build_check(out: &Path, check: &resolve_corpus::BuildCheck) -> Result<bool, String> {
    let path = out.join("resolve-build-check.json");
    let mut text = serde_json::to_string_pretty(check).map_err(|e| e.to_string())?;
    text.push('\n');
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    println!(
        "resolve_build: {} of {} modifier lines equal the harness",
        check.equal, check.lines
    );
    Ok(check.differences.is_empty())
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
    let Some((positional, values, _)) = options(args, &["--rules", "--out"], &[]) else {
        return usage();
    };
    let ([bundle_dir, export], Some(rules), Some(out)) = (
        positional.as_slice(),
        values.get("--rules"),
        values.get("--out"),
    ) else {
        return usage();
    };
    let out = Path::new(out);
    let outputs = data_parity_outputs(
        Path::new(bundle_dir),
        Path::new(export),
        Path::new(rules),
        out,
    )
    .and_then(|outputs| data_parity_write(out, &outputs).map(|()| outputs));
    let outputs = match outputs {
        Ok(outputs) => outputs,
        Err(message) => {
            eprintln!("data-parity: {message}");
            return ExitCode::from(3);
        }
    };
    data_parity_print(&outputs);
    if outputs.summary.pass {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Reads `data-parity`'s inputs and compares; `out` must not exist.
fn data_parity_outputs(
    bundle_dir: &Path,
    export: &Path,
    rules: &Path,
    out: &Path,
) -> Result<data_parity::Outputs, String> {
    if out.exists() {
        return Err(format!("{}: already exists", out.display()));
    }
    let bundle = load_bundle(bundle_dir)?;
    let read = |path: &Path| std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()));
    let manifest = read(&bundle_dir.join("bundle.json"))?;
    let export = read(export)?;
    let rules = read(rules)?;
    // Each timeless jewel results file's Adler-32 (P6-10a).
    let mut timeless_adlers = std::collections::BTreeMap::new();
    for f in bundle.manifest.files.iter().filter(|f| {
        f.path
            .starts_with(loom_data::bundle::timeless::TIMELESS_DIR)
    }) {
        let bytes = read(&bundle_dir.join(&f.path))?;
        timeless_adlers.insert(f.path.clone(), data_parity::timeless::adler32(&bytes));
    }
    data_parity::run(&data_parity::Request {
        timeless_adlers: &timeless_adlers,
        bundle: &bundle,
        bundle_manifest: &manifest,
        export: &export,
        rules: &rules,
        passive_scope: data_parity::compare::PASSIVE_SCOPE,
        bosses: &loom_calc::poe1::enemy::BOSSES,
        tool: tool(),
    })
}

/// Writes `summary.json`, `report.json`, and `overlay.json` to `out`.
fn data_parity_write(out: &Path, outputs: &data_parity::Outputs) -> Result<(), String> {
    write_out(out, &outputs.summary, &outputs.report)?;
    let mut text = serde_json::to_string_pretty(&outputs.overlay).map_err(|e| e.to_string())?;
    text.push('\n');
    let path = out.join("overlay.json");
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Prints `data-parity`'s counts, and each failed rule on stderr.
fn data_parity_print(outputs: &data_parity::Outputs) {
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
    let mut inputs = Vec::new();
    for arg in args {
        let path = Path::new(arg);
        let folder = path.is_dir();
        let files = if folder {
            folder_builds(arg, path)?
        } else {
            vec![path.to_path_buf()]
        };
        for file in &files {
            inputs.push(read_input(arg, folder, file)?);
        }
    }
    Ok(inputs)
}

/// A build file's format, by its extension.
fn format_of(path: &Path) -> Option<build_round_trip::Format> {
    use build_round_trip::Format;
    match path.extension().and_then(|e| e.to_str()) {
        Some(e) if e.eq_ignore_ascii_case("xml") => Some(Format::PobXml),
        Some(e) if e.eq_ignore_ascii_case("json") => Some(Format::BuildDocument),
        _ => None,
    }
}

/// Each `*.xml` directly in the folder `path` (given as `arg`), in name
/// order.
fn folder_builds(arg: &str, path: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(path).map_err(|e| format!("{arg}: {e}"))? {
        let p = entry.map_err(|e| format!("{arg}: {e}"))?.path();
        if p.is_file() && format_of(&p) == Some(build_round_trip::Format::PobXml) {
            files.push(p);
        }
    }
    files.sort();
    Ok(files)
}

/// Reads the input file `p`: the argument `arg` itself, or a file of the
/// folder `arg` names (`folder`).
fn read_input(arg: &str, folder: bool, p: &Path) -> Result<build_round_trip::Input, String> {
    let format = format_of(p).ok_or_else(|| format!("{}: not *.xml or *.json", p.display()))?;
    let file = p
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let shown = if folder {
        format!("{}/{file}", arg.trim_end_matches(['/', '\\']))
    } else {
        arg.to_owned()
    };
    Ok(build_round_trip::Input {
        path: shown.replace('\\', "/"),
        file,
        format,
        bytes: std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?,
    })
}

/// `build-round-trip` (module `build_round_trip`). Exit 0 when every build
/// passes, 1 when one fails, 2 on a usage error, 3 when the run cannot
/// start (an input or the bundle does not read or validate, or `--out`
/// exists).
fn build_round_trip_cmd(args: &[String]) -> ExitCode {
    let Some(args) = round_trip_args(args) else {
        return usage();
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
        tool: tool(),
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
        if arg == "--anonymous" {
            out.anonymous = true;
        } else if let Some(slot) = out.slot(arg) {
            let [value, tail @ ..] = rest else {
                return None;
            };
            if slot.replace(value.clone()).is_some() {
                return None;
            }
            rest = tail;
        } else if arg.starts_with("--") {
            return None;
        } else {
            out.inputs.push(arg.clone());
        }
    }
    out.complete().then_some(out)
}

impl SurveyArgs {
    /// The value a `--name <value>` option sets.
    fn slot(&mut self, name: &str) -> Option<&mut Option<String>> {
        match name {
            "--bundle" => Some(&mut self.bundle),
            "--scope" => Some(&mut self.scope),
            "--data-rules" => Some(&mut self.data_rules),
            "--out" => Some(&mut self.out),
            _ => None,
        }
    }

    /// Whether an input and every required option are given.
    fn complete(&self) -> bool {
        !self.inputs.is_empty()
            && self.bundle.is_some()
            && self.scope.is_some()
            && self.data_rules.is_some()
            && self.out.is_some()
    }
}

/// `calc-survey` (module `calc_survey`).
fn calc_survey_cmd(args: &[String]) -> ExitCode {
    let Some(args) = survey_args(args) else {
        return usage();
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
            tool: tool(),
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
    calc_survey_print(&summary);
    Ok(summary.pass)
}

/// Prints `calc-survey`'s counts, the first candidates per kind, and the
/// chosen skills.
fn calc_survey_print(summary: &calc_survey::Summary) {
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
}

/// Positional arguments, `--name value` options, and flags.
type CliArgs = (Vec<String>, BTreeMap<String, String>, BTreeSet<String>);

/// Options of the form `--name value`, plus positional arguments and the
/// flags in `flags`; `None` on an unknown option or a repeated one.
fn options(args: &[String], names: &[&str], flags: &[&str]) -> Option<CliArgs> {
    let mut positional = Vec::new();
    let mut values = BTreeMap::new();
    let mut set = BTreeSet::new();
    let mut rest = args;
    while let [arg, tail @ ..] = rest {
        rest = tail;
        if flags.contains(&arg.as_str()) {
            if !set.insert(arg.clone()) {
                return None;
            }
        } else if names.contains(&arg.as_str()) {
            let [value, tail @ ..] = rest else {
                return None;
            };
            rest = tail;
            if values.insert(arg.clone(), value.clone()).is_some() {
                return None;
            }
        } else if arg.starts_with("--") {
            return None;
        } else {
            positional.push(arg.clone());
        }
    }
    Some((positional, values, set))
}

/// `calc-run` (module `calc_run`).
fn calc_run_cmd(args: &[String]) -> ExitCode {
    let Some((inputs, values, flags)) = options(
        args,
        &["--bundle", "--out", "--data-overlay", "--scope"],
        &["--anonymous"],
    ) else {
        return usage();
    };
    let (Some(bundle), Some(out)) = (values.get("--bundle"), values.get("--out")) else {
        return usage();
    };
    if inputs.is_empty() {
        return usage();
    }
    let run = calc_run_run(
        &inputs,
        Path::new(bundle),
        Path::new(out),
        values.get("--data-overlay").map(Path::new),
        values.get("--scope").map(Path::new),
        flags.contains("--anonymous"),
    );
    match run {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(message) => {
            eprintln!("calc-run: {message}");
            ExitCode::from(3)
        }
    }
}

fn calc_run_run(
    inputs: &[String],
    bundle_dir: &Path,
    out: &Path,
    overlay: Option<&Path>,
    scope: Option<&Path>,
    anonymous: bool,
) -> Result<bool, String> {
    if out.exists() {
        return Err(format!("{}: already exists", out.display()));
    }
    let inputs = round_trip_inputs(inputs)?;
    let bundle = load_bundle(bundle_dir)?;
    let manifest_path = bundle_dir.join("bundle.json");
    let manifest_bytes =
        std::fs::read(&manifest_path).map_err(|e| format!("{}: {e}", manifest_path.display()))?;
    let manifest_sha256 = oracle::sha256_hex(&manifest_bytes);
    let data = loom_parser::import::BundleData::new(&bundle);
    let resolver = loom_parser::import::Resolver::new(&data)
        .map_err(|e| format!("the bundle's pipeline does not build: {} ({e})", e.code()))?;
    let overlay = overlay
        .map(|path| calc_run_overlay(path, &bundle, &manifest_sha256))
        .transpose()?;
    let scope = scope
        .map(|path| calc_run_scope(path, &bundle))
        .transpose()?;
    let tree = loom_tree::Tree::from_bundle(&bundle);
    let trees = tree
        .as_ref()
        .map(|t| loom_pipeline::pack_trees(&bundle, t))
        .unwrap_or_default();
    let timeless_results = timeless_results(bundle_dir, &bundle)?;
    let request = calc_run::Request {
        timeless_results: &timeless_results,
        scope,
        bundle: &bundle,
        resolver: &resolver,
        tree: tree.as_ref(),
        trees: &trees,
        data: &data,
        bundle_record: calc_run::BundleRecord {
            game: bundle.game.name().to_owned(),
            steam_build: bundle.manifest.source.steam_build,
            data_schema: bundle.manifest.data_schema,
            manifest_sha256,
        },
        overlay: overlay.as_ref().map(|(data, sha256)| {
            (
                data,
                calc_run::OverlayRecord {
                    sha256: sha256.clone(),
                    rows: data.counts(),
                },
            )
        }),
        anonymous,
        tool: calc_run::ToolRecord {
            commit: git(&["rev-parse", "HEAD"]),
            dirty: git(&["status", "--porcelain"]).map(|s| !s.is_empty()),
        },
    };
    let outputs = calc_run::run(&inputs, &request)?;
    calc_run::write(out, &outputs)?;
    calc_run_print(&outputs);
    let s = &outputs.summary;
    Ok(s.failed == 0 && s.builds > 0)
}

/// `--data-overlay`: data-parity's overlay for this bundle, and the
/// file's SHA-256.
fn calc_run_overlay<'b>(
    path: &Path,
    bundle: &'b loom_data::bundle::Bundle,
    manifest_sha256: &str,
) -> Result<(calc_run::OverlayData<'b>, String), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let data = calc_run::OverlayData::read(&bytes, bundle, manifest_sha256)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok((data, oracle::sha256_hex(&bytes)))
}

/// `--scope`: calc-survey's scope file, which must be for this bundle.
fn calc_run_scope(
    path: &Path,
    bundle: &loom_data::bundle::Bundle,
) -> Result<calc_run::ScopeRecord, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let file =
        calc_survey::rules::Scope::read(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    if file.game != bundle.game.name()
        || Some(file.steam_build) != bundle.manifest.source.steam_build
    {
        return Err(format!(
            "{}: for {} Steam build {}, not this bundle",
            path.display(),
            file.game,
            file.steam_build
        ));
    }
    Ok(calc_run::ScopeRecord {
        sha256: oracle::sha256_hex(&bytes),
        tree_version: file.tree_version,
    })
}

/// The Timeless jewels' results (P6-10a): every type, read once for
/// every build (the app reads one type when a build needs it).
fn timeless_results(
    bundle_dir: &Path,
    bundle: &loom_data::bundle::Bundle,
) -> Result<BTreeMap<u32, Vec<u8>>, String> {
    let mut results = BTreeMap::new();
    if let Some(luts) = bundle.poe1.as_ref().and_then(|p| p.timeless_luts.as_ref()) {
        for j in &luts.jewels {
            let path = bundle_dir.join(&j.file);
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            results.insert(j.jewel, bytes);
        }
    }
    Ok(results)
}

/// Prints `calc-run`'s counts, and each failed build on stderr.
fn calc_run_print(outputs: &calc_run::Outputs) {
    let s = &outputs.summary;
    println!(
        "calc-run: {} builds, {} ok, {} failed; fields {} complete, {} partial, {} unsupported, {} indeterminate; calculation {} us total",
        s.builds,
        s.ok,
        s.failed,
        s.field_states.complete,
        s.field_states.partial,
        s.field_states.unsupported,
        s.field_states.indeterminate,
        s.timing.total_micros
    );
    for b in outputs.report.builds.iter().filter(|b| b.status != "ok") {
        eprintln!(
            "{}: {} at {}",
            b.id,
            b.code.as_deref().unwrap_or("?"),
            b.step
                .map_or_else(String::new, |step| calc_run::reason_kind(&step))
        );
    }
}

/// `calc-parity` (module `calc_parity`).
fn calc_parity_cmd(args: &[String]) -> ExitCode {
    let parsed = options(
        args,
        &[
            "--slice",
            "--out",
            "--register",
            "--overlay-run",
            "--manifest",
        ],
        &[],
    );
    let request = parsed.and_then(|(positional, values, _)| {
        let [oracle, successor] = positional.as_slice() else {
            return None;
        };
        Some(calc_parity::Request {
            oracle: oracle.into(),
            successor: successor.into(),
            slice: values.get("--slice")?.into(),
            out: values.get("--out")?.into(),
            register: values.get("--register").map(Into::into),
            overlay_run: values.get("--overlay-run").map(Into::into),
            manifest: values.get("--manifest").map(Into::into),
        })
    });
    let Some(request) = request else {
        return usage();
    };
    match calc_parity::run(&request) {
        Ok(report) => {
            print!("{}", report.summary_markdown());
            ExitCode::from(report.exit_code())
        }
        Err(message) => {
            eprintln!("calc-parity: {message}");
            ExitCode::from(3)
        }
    }
}

/// Prints a tree-parity record; whether it matched.
fn print_record<T: Serialize>((record, ok): (T, bool)) -> Result<bool, String> {
    let json = serde_json::to_string_pretty(&record).map_err(|e| e.to_string())?;
    println!("{json}");
    Ok(ok)
}

/// Prints a command's summary as JSON; the command passed.
fn print_summary(summary: &impl Serialize) -> Result<bool, String> {
    let json = serde_json::to_string_pretty(summary).map_err(|e| e.to_string())?;
    println!("{json}");
    Ok(true)
}

/// `tree-parity alloc-requests`: prints the seeded requests.
fn alloc_requests(
    bundle: &str,
    count: &str,
    seed: &str,
    version: Option<&str>,
) -> Result<bool, String> {
    let count = count.parse().map_err(|e| format!("count: {e}"))?;
    let seed = seed.parse().map_err(|e| format!("seed: {e}"))?;
    let text = tree_parity::alloc_requests(Path::new(bundle), count, seed, version)?;
    print!("{text}");
    Ok(true)
}

/// `tree-parity <mode> ...` (module `tree_parity`).
fn tree_parity_cmd(args: &[String]) -> ExitCode {
    let [mode, rest @ ..] = args else {
        return usage();
    };
    let result = match (mode.as_str(), rest) {
        ("layout", [pob, bundle]) => {
            tree_parity::layout(Path::new(pob), Path::new(bundle)).and_then(print_record)
        }
        ("alloc-requests", [bundle, count, seed, version @ ..]) if version.len() <= 1 => {
            alloc_requests(bundle, count, seed, version.first().map(String::as_str))
        }
        ("alloc", [requests, pob, bundle]) => {
            tree_parity::alloc(Path::new(requests), Path::new(pob), Path::new(bundle))
                .and_then(print_record)
        }
        ("convert", [requests, pob, bundle, from]) => {
            tree_parity::convert(Path::new(requests), Path::new(pob), Path::new(bundle), from)
                .and_then(print_record)
        }
        ("readback", [build, pob]) => {
            tree_parity::readback(Path::new(build), Path::new(pob)).and_then(print_record)
        }
        _ => return usage(),
    };
    finish(result)
}

/// `tree-pack <manifest> <version> <export-dir> <bundle-dir>
/// [--accept-unread] --out <dir>` (module `tree_pack`).
fn tree_pack_cmd(args: &[String]) -> ExitCode {
    let [pins, version, export, bundle, rest @ .., flag, out] = args else {
        return usage();
    };
    if flag != "--out" || !(rest.is_empty() || rest == ["--accept-unread"]) {
        return usage();
    }
    let summary = tree_pack::run(
        Path::new(pins),
        version,
        Path::new(export),
        Path::new(bundle),
        Path::new(out),
        !rest.is_empty(),
    );
    finish(summary.and_then(|summary| print_summary(&summary)))
}

/// The usage text on stderr; exit 2.
fn usage() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(2)
}

/// The exit code of a command that passed, failed, or stopped with a
/// message.
fn finish(result: Result<bool, String>) -> ExitCode {
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(message) => {
            eprintln!("loom-parity-cli: {message}");
            ExitCode::FAILURE
        }
    }
}

/// A subcommand: the arguments after its name to the exit code. Arguments
/// of another shape are a usage error.
type Command = fn(&[String]) -> ExitCode;

/// Every subcommand by name, in [`USAGE`]'s order.
const COMMANDS: &[(&str, Command)] = &[
    ("tree-parity", tree_parity_cmd),
    ("tree-pack", tree_pack_cmd),
    ("timeless-pack", |args| match args {
        [bundle, pob, version, flag, out] if flag == "--out" => finish(
            timeless::pack(Path::new(bundle), Path::new(pob), version, Path::new(out))
                .and_then(|summary| print_summary(&summary)),
        ),
        _ => usage(),
    }),
    ("tree-layout", |args| match args {
        [pin, export, bundle, flag, out] if flag == "--out" => finish(
            tree_layout::run(
                Path::new(pin),
                Path::new(export),
                Path::new(bundle),
                Path::new(out),
            )
            .and_then(|summary| print_summary(&summary)),
        ),
        _ => usage(),
    }),
    ("bundle", |args| match args {
        [extract, flag, out] if flag == "--out" => {
            finish(bundle_cmd(Path::new(extract), Path::new(out)))
        }
        _ => usage(),
    }),
    ("bundle-check", |args| match args {
        [dir] => finish(bundle_check(Path::new(dir))),
        _ => usage(),
    }),
    ("load-statdesc", |args| match args {
        [extract] => finish(load_statdesc(Path::new(extract))),
        _ => usage(),
    }),
    ("round-trip", |args| match args {
        [extract, flag, out] if flag == "--out" => {
            finish(round_trip(Path::new(extract), Path::new(out)))
        }
        _ => usage(),
    }),
    ("render-check", |args| match args {
        [extract, pob, flag, out] if flag == "--out" => finish(render_check(
            Path::new(extract),
            Path::new(pob),
            Path::new(out),
        )),
        _ => usage(),
    }),
    ("bundle-equal", |args| match args {
        [extract, bundle] => finish(bundle_equal(Path::new(extract), Path::new(bundle))),
        _ => usage(),
    }),
    ("resolve-corpus", |args| match args {
        [builds, extract, flag, out, pob @ ..] if flag == "--out" => finish(resolve_corpus(
            Path::new(builds),
            Path::new(extract),
            Path::new(out),
            pob,
        )),
        _ => usage(),
    }),
    ("oracle-verify", |args| match args {
        [manifest, oracle_dir] => oracle_verify(Path::new(manifest), Path::new(oracle_dir)),
        _ => usage(),
    }),
    ("oracle-run", |args| match args {
        [manifest, oracle_dir, rest @ ..] => {
            oracle_run(Path::new(manifest), Path::new(oracle_dir), rest)
        }
        _ => usage(),
    }),
    ("oracle-compare", |args| match args {
        [left, right, rest @ ..] => oracle_compare(Path::new(left), Path::new(right), rest),
        _ => usage(),
    }),
    ("dto-hashes", |args| match args {
        [] => usage(),
        dirs => finish(dto_hashes(dirs)),
    }),
    ("pob-skills", |args| match args {
        [export, flag, out] if flag == "--out" => {
            finish(pob_skills::run_skills(Path::new(export), Path::new(out)))
        }
        _ => usage(),
    }),
    ("pob-minions", |args| match args {
        [export, flag, out] if flag == "--out" => {
            finish(pob_skills::run_minions(Path::new(export), Path::new(out)))
        }
        _ => usage(),
    }),
    ("custom-lines", |args| match args {
        [bundle, pob, flag, out] if flag == "--out" => finish(custom_lines_cmd(
            Path::new(bundle),
            Path::new(pob),
            Path::new(out),
        )),
        _ => usage(),
    }),
    ("item-rewordings", |args| match args {
        [step, table, flag, out, extra @ ..] if step == "lines" && flag == "--out" => finish(
            item_rewordings_lines(Path::new(table), Path::new(out), extra),
        ),
        [step, bundle, table, pob, flag, out] if step == "check" && flag == "--out" => {
            finish(item_rewordings_check(
                Path::new(bundle),
                Path::new(table),
                Path::new(pob),
                Path::new(out),
            ))
        }
        _ => usage(),
    }),
    ("data-parity", data_parity_cmd),
    ("build-round-trip", build_round_trip_cmd),
    ("calc-survey", calc_survey_cmd),
    ("calc-run", calc_run_cmd),
    ("calc-parity", calc_parity_cmd),
    ("divergence-check", |args| match args {
        [register] => divergence_check(Path::new(register)),
        _ => usage(),
    }),
];

/// The subcommand named `name`.
fn command(name: &str) -> Option<Command> {
    COMMANDS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, run)| *run)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.split_first() {
        Some((name, rest)) => command(name).map_or_else(usage, |run| run(rest)),
        None => usage(),
    }
}

#[cfg(test)]
mod tests;
