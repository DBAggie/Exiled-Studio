//! `build-round-trip` (Phase 2 plan, step 8; gates 1 and 2): every build
//! through import, resolution, the v1 JSON document, and PoB XML export,
//! with four checks and the unknown-content mutation set.
//!
//! **Sequence**, per build:
//! 1. read the input (Build A): a PoB build (`*.xml`, `pob_xml::import`)
//!    or a v1 build document (`*.json`, `dto::v1::from_json`);
//! 2. resolve A when a bundle is given (`Resolver::resolve_build`, with
//!    PoB's base rule `pob_item::item_base` over the bundle's base names);
//! 3. write A as JSON and read it back (Build B);
//! 4. export B as PoB XML and import it again (Build C), then resolve C
//!    with the same bundle. PoB's format is PoE1's, so a PoE2 build (a
//!    document) has no C.
//!
//! **Checks:**
//! - **(a) `xml-equal`:** `xml_equal(export(B), original)`, for PoB input;
//! - **(b) `builds-equal`:** B and C equal A, resolutions included (C was
//!   resolved again, so resolution is compared too);
//! - **(c) `json-stable`:** steps 1 to 3 run twice give the same bytes;
//! - **(d) `source-lines`:** every typed item's source lines equal the
//!   lines of its text in the input (PoB input: the `<Item>`'s last
//!   non-blank text child split into non-blank trimmed lines, as
//!   `pob_item::read` does; a document: A's lines before resolution), in
//!   A after resolution, in B, and in C.
//!
//! Each check is `pass`, `fail`, `not-applicable`, or `not-run` (an
//! earlier step failed). PoB input also runs the mutation set ([`mutate`]):
//! each mutated copy must import, write and read back as JSON to an equal
//! build, and export to a document `xml_equal` to the copy.
//!
//! **Expected import errors:** a file named by `--expect-import-error`
//! must not import (an `IMPORT_*` or `DTO_*` code); it then passes.
//!
//! **Scrubbing (ADR-007):** no item, gem, or author text is written. The
//! summary has counts and, per build, its name: the path as given, or
//! with `--anonymous` the SHA-256 of its bytes (as `resolve-corpus` names
//! builds). The report adds failures, which name checks, item IDs, line
//! numbers, codes, and element paths, never text. Identical files count
//! once.

pub mod mutate;

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]
mod tests;

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use loom_domain::{Build, Game, LineKind, LineResolution};
use loom_parser::import::{BundleData, ItemBaseName, ItemData, Resolver};
use loom_serialization::dto::v1::{from_json, to_json};
use loom_serialization::legacy::pob_item;
use loom_serialization::legacy::pob_xml::{export, import, kept_whole, xml_equal};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::resolve_corpus::{BundleInputs, Tool};

/// The input's format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Format {
    /// A PoB build (`*.xml`).
    PobXml,
    /// A v1 build document (`*.json`).
    BuildDocument,
}

/// One input file.
#[derive(Debug)]
pub struct Input {
    /// The path as given (`/` separators).
    pub path: String,
    /// The file name (what `--expect-import-error` names).
    pub file: String,
    /// The format.
    pub format: Format,
    /// The bytes.
    pub bytes: Vec<u8>,
}

/// The bundle a run resolves with.
pub struct Data<'r, 'd, 'b> {
    /// The pipeline, built once.
    pub resolver: &'r Resolver<'d, 'b>,
    /// Its data (for the base names).
    pub data: &'d BundleData<'b>,
}

/// A check's result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// Held.
    Pass,
    /// Did not hold.
    Fail,
    /// Does not apply to this input (module docs).
    NotApplicable,
    /// An earlier step failed.
    NotRun,
}

impl Status {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::NotApplicable => "not-applicable",
            Self::NotRun => "not-run",
        }
    }

    const fn of(ok: bool) -> Self {
        if ok { Self::Pass } else { Self::Fail }
    }
}

/// The four checks (module docs).
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Checks {
    /// (a)
    pub a_xml_equal: Status,
    /// (b)
    pub b_builds_equal: Status,
    /// (c)
    pub c_json_stable: Status,
    /// (d)
    pub d_source_lines: Status,
}

impl Checks {
    const NOT_RUN: Self = Self {
        a_xml_equal: Status::NotRun,
        b_builds_equal: Status::NotRun,
        c_json_stable: Status::NotRun,
        d_source_lines: Status::NotRun,
    };

    fn all(self) -> [(&'static str, Status); 4] {
        [
            ("a_xml_equal", self.a_xml_equal),
            ("b_builds_equal", self.b_builds_equal),
            ("c_json_stable", self.c_json_stable),
            ("d_source_lines", self.d_source_lines),
        ]
    }
}

/// Mutations tried and survived.
#[derive(Debug, Default, Clone, Copy, Serialize)]
pub struct MutationCount {
    /// Mutated copies run.
    pub tried: usize,
    /// Copies whose content survived.
    pub survived: usize,
}

/// One build's result (summary row).
#[derive(Debug, Serialize)]
pub struct BuildRow {
    /// The path as given, or `sha256:<hex>` with `--anonymous`.
    pub name: String,
    /// The input's format.
    pub format: Format,
    /// `poe1` or `poe2`, once read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game: Option<&'static str>,
    /// Whether every check passed (or the expected import error came).
    pub ok: bool,
    /// The read's error code, if it did not read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_error: Option<&'static str>,
    /// Whether the file was named by `--expect-import-error`.
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    pub import_error_expected: bool,
    /// The checks.
    pub checks: Checks,
    /// Typed items.
    pub items: usize,
    /// Their source lines.
    pub lines: usize,
    /// Modifier lines of A by resolution (`pending`, `excluded`,
    /// `resolved`, `resolved-adapted`, `continues`, `unresolved`).
    pub modifier_lines: BTreeMap<&'static str, usize>,
    /// A's completeness after resolution, when a bundle was given.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completeness: Option<&'static str>,
    /// Typed elements the import kept whole (`pob_xml::kept_whole`).
    pub kept_whole: usize,
    /// Mutations by kind (PoB input).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub mutations: BTreeMap<&'static str, MutationCount>,
    /// SHA-256 of A's JSON document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_sha256: Option<String>,
}

/// One build's failures (report).
#[derive(Debug, Serialize)]
pub struct BuildFailures {
    /// As in [`BuildRow::name`].
    pub name: String,
    /// What failed (no build text).
    pub failures: Vec<String>,
}

/// Recorded inputs.
#[derive(Debug, Serialize)]
pub struct RunInputs {
    /// The tool.
    pub tool: Tool,
    /// The bundle, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle: Option<BundleInputs>,
    /// Whether builds are named by content hash.
    pub anonymous: bool,
}

/// The summary (scrubbed; module docs).
#[derive(Debug, Serialize)]
pub struct Summary {
    /// Recorded inputs.
    pub inputs: RunInputs,
    /// Distinct input files.
    pub builds: usize,
    /// Files skipped as identical to an earlier one.
    pub duplicates: usize,
    /// Builds with every check passed (or the expected import error).
    pub passed: usize,
    /// The others.
    pub failed: usize,
    /// Check to status to builds.
    pub checks: BTreeMap<&'static str, BTreeMap<&'static str, usize>>,
    /// Typed items.
    pub items: usize,
    /// Their source lines.
    pub lines: usize,
    /// Modifier lines by resolution.
    pub modifier_lines: BTreeMap<&'static str, usize>,
    /// Builds by completeness (with a bundle).
    pub completeness: BTreeMap<&'static str, usize>,
    /// Typed elements kept whole.
    pub kept_whole: usize,
    /// Mutations by kind.
    pub mutations: BTreeMap<&'static str, MutationCount>,
    /// Every build.
    pub per_build: Vec<BuildRow>,
}

/// The report: failures per build.
#[derive(Debug, Serialize)]
pub struct Report {
    /// Builds with a failure.
    pub builds: Vec<BuildFailures>,
}

/// The run's options.
pub struct Options<'a, 'r, 'd, 'b> {
    /// The bundle, if any.
    pub data: Option<Data<'r, 'd, 'b>>,
    /// Name builds by content hash.
    pub anonymous: bool,
    /// File names that must not import.
    pub expect_import_error: &'a BTreeSet<String>,
    /// The tool's commit.
    pub tool: Tool,
}

/// Runs every input (module docs).
#[must_use]
pub fn run(inputs: &[Input], options: &Options<'_, '_, '_, '_>) -> (Summary, Report) {
    let mut summary = Summary {
        inputs: RunInputs {
            tool: options.tool.clone(),
            bundle: options.data.as_ref().map(|d| {
                let m = &d.data.bundle().manifest;
                BundleInputs {
                    game: m.game.clone(),
                    steam_build: m.source.steam_build,
                    index_sha256: m.source.index_sha256.clone(),
                }
            }),
            anonymous: options.anonymous,
        },
        builds: 0,
        duplicates: 0,
        passed: 0,
        failed: 0,
        checks: BTreeMap::new(),
        items: 0,
        lines: 0,
        modifier_lines: BTreeMap::new(),
        completeness: BTreeMap::new(),
        kept_whole: 0,
        mutations: BTreeMap::new(),
        per_build: Vec::new(),
    };
    let mut report = Report { builds: Vec::new() };
    let mut seen = BTreeSet::new();
    for input in inputs {
        let hash = hex(&Sha256::digest(&input.bytes));
        if !seen.insert(hash.clone()) {
            summary.duplicates += 1;
            continue;
        }
        let name = if options.anonymous {
            format!("sha256:{hash}")
        } else {
            input.path.clone()
        };
        let expected = options.expect_import_error.contains(&input.file);
        let (row, failures) = one(name, input, options.data.as_ref(), expected);
        add(&mut summary, &row);
        if !failures.is_empty() {
            report.builds.push(BuildFailures {
                name: row.name.clone(),
                failures,
            });
        }
        summary.per_build.push(row);
    }
    (summary, report)
}

fn add(s: &mut Summary, row: &BuildRow) {
    s.builds += 1;
    if row.ok {
        s.passed += 1;
    } else {
        s.failed += 1;
    }
    for (check, status) in row.checks.all() {
        *s.checks
            .entry(check)
            .or_default()
            .entry(status.name())
            .or_default() += 1;
    }
    s.items += row.items;
    s.lines += row.lines;
    for (k, n) in &row.modifier_lines {
        *s.modifier_lines.entry(k).or_default() += n;
    }
    if let Some(c) = row.completeness {
        *s.completeness.entry(c).or_default() += 1;
    }
    s.kept_whole += row.kept_whole;
    for (k, m) in &row.mutations {
        let t = s.mutations.entry(k).or_default();
        t.tried += m.tried;
        t.survived += m.survived;
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Steps 1 and 2: the input read and resolved.
struct Read {
    /// A as read.
    read: Build,
    /// A after resolution (equal to `read` without a bundle).
    resolved: Build,
}

/// Reads an input; the error is a stable code.
fn read(format: Format, text: &str) -> Result<Build, &'static str> {
    match format {
        Format::PobXml => import(text).map_err(|e| e.code()),
        Format::BuildDocument => from_json(text).map_err(|e| e.code()),
    }
}

/// Resolves a build with the bundle, if any; the error is a stable code.
fn resolve(build: &mut Build, data: Option<&Data<'_, '_, '_>>) -> Result<(), &'static str> {
    let Some(d) = data else {
        return Ok(());
    };
    d.resolver
        .resolve_build(build, |body| {
            pob_item::item_base(body, d.data.base_names())
                .map(|(name, fallback)| ItemBaseName { name, fallback })
                .map_err(|e| e.name().to_owned())
        })
        .map(|_| ())
        .map_err(|e| e.code())
}

/// Why steps 1 and 2 stopped: a stable code.
enum StepError {
    Read(&'static str),
    Resolve(&'static str),
}

fn read_and_resolve(
    format: Format,
    text: &str,
    data: Option<&Data<'_, '_, '_>>,
) -> Result<Read, StepError> {
    let a = read(format, text).map_err(StepError::Read)?;
    let mut resolved = a.clone();
    resolve(&mut resolved, data).map_err(StepError::Resolve)?;
    Ok(Read { read: a, resolved })
}

fn one(
    name: String,
    input: &Input,
    data: Option<&Data<'_, '_, '_>>,
    expected_error: bool,
) -> (BuildRow, Vec<String>) {
    let mut row = BuildRow {
        name,
        format: input.format,
        game: None,
        ok: false,
        read_error: None,
        import_error_expected: expected_error,
        checks: Checks::NOT_RUN,
        items: 0,
        lines: 0,
        modifier_lines: BTreeMap::new(),
        completeness: None,
        kept_whole: 0,
        mutations: BTreeMap::new(),
        json_sha256: None,
    };
    let mut failures = Vec::new();
    let Ok(text) = core::str::from_utf8(&input.bytes) else {
        row.read_error = Some("NOT_UTF8");
        if !expected_error {
            failures.push("read: not UTF-8".to_owned());
        }
        row.ok = failures.is_empty();
        return (row, failures);
    };
    if expected_error {
        match read(input.format, text) {
            Err(code) => {
                row.read_error = Some(code);
                row.checks = Checks {
                    a_xml_equal: Status::NotApplicable,
                    b_builds_equal: Status::NotApplicable,
                    c_json_stable: Status::NotApplicable,
                    d_source_lines: Status::NotApplicable,
                };
                row.ok = true;
            }
            Ok(_) => failures.push("expected an import error; the file imported".to_owned()),
        }
        return (row, failures);
    }
    checks(&mut row, &mut failures, input.format, text, data);
    // A file that did not read has no content to mutate.
    if input.format == Format::PobXml && row.read_error.is_none() {
        mutations(&mut row, &mut failures, text);
    }
    row.ok = failures.is_empty();
    (row, failures)
}

/// Steps 1 to 4 and checks (a) to (d).
fn checks(
    row: &mut BuildRow,
    failures: &mut Vec<String>,
    format: Format,
    text: &str,
    data: Option<&Data<'_, '_, '_>>,
) {
    let a = match read_and_resolve(format, text, data) {
        Ok(a) => a,
        Err(StepError::Read(code)) => {
            row.read_error = Some(code);
            failures.push(format!("read: {code}"));
            return;
        }
        Err(StepError::Resolve(code)) => {
            failures.push(format!("resolve A: {code}"));
            return;
        }
    };
    row.game = Some(a.resolved.game().name());
    if format == Format::PobXml {
        row.kept_whole = kept_whole(&a.read).len();
    }
    if data.is_some() {
        row.completeness = Some(a.resolved.completeness().name());
    }
    count_lines(row, &a.resolved);

    // Step 3 and check (c).
    let json = match to_json(&a.resolved) {
        Ok(j) => j,
        Err(e) => {
            failures.push(format!("write A: {}", e.code()));
            return;
        }
    };
    row.json_sha256 = Some(hex(&Sha256::digest(json.as_bytes())));
    let again = read_and_resolve(format, text, data)
        .ok()
        .and_then(|a2| to_json(&a2.resolved).ok());
    row.checks.c_json_stable = Status::of(again.as_deref() == Some(json.as_str()));
    if row.checks.c_json_stable == Status::Fail {
        failures.push("(c) a second run wrote other JSON bytes".to_owned());
    }
    let b = match from_json(&json) {
        Ok(b) => b,
        Err(e) => {
            failures.push(format!("read B: {}", e.code()));
            return;
        }
    };

    // Step 4 and checks (a) and (b).
    let mut equal = b == a.resolved;
    if !equal {
        failures.push("(b) B differs from A".to_owned());
    }
    let mut c = None;
    if a.resolved.game() == Game::Poe1 {
        let xml = export(&b);
        if format == Format::PobXml {
            let same = xml_equal(&xml, text).unwrap_or(false);
            row.checks.a_xml_equal = Status::of(same);
            if !same {
                failures.push("(a) export(B) is not xml_equal to the input".to_owned());
            }
        } else {
            row.checks.a_xml_equal = Status::NotApplicable;
        }
        match import(&xml) {
            Ok(mut built) => match resolve(&mut built, data) {
                Ok(()) => {
                    if built != a.resolved {
                        equal = false;
                        failures.push("(b) C differs from A".to_owned());
                    }
                    c = Some(built);
                }
                Err(code) => {
                    equal = false;
                    failures.push(format!("resolve C: {code}"));
                }
            },
            Err(e) => {
                equal = false;
                failures.push(format!("import C: {}", e.code()));
            }
        }
    } else {
        row.checks.a_xml_equal = Status::NotApplicable;
    }
    row.checks.b_builds_equal = Status::of(equal);

    // Check (d).
    let input_lines = match format {
        Format::PobXml => xml_item_lines(text),
        Format::BuildDocument => item_lines(&a.read)
            .into_iter()
            .map(|(id, lines)| (id.to_string(), lines))
            .collect(),
    };
    let mut lines_ok = true;
    let mut outs: Vec<(&str, &Build)> = vec![("A", &a.resolved), ("B", &b)];
    if let Some(c) = &c {
        outs.push(("C", c));
    }
    for (label, build) in outs {
        lines_ok &= same_lines(&input_lines, build, label, failures);
    }
    row.checks.d_source_lines = Status::of(lines_ok);
}

fn count_lines(row: &mut BuildRow, b: &Build) {
    for item in b.items.items() {
        row.items += 1;
        row.lines += item.body.lines.len();
        for line in &item.body.lines {
            if let LineKind::Modifier(m) = &line.kind {
                *row.modifier_lines
                    .entry(resolution_class(&m.resolution))
                    .or_default() += 1;
            }
        }
    }
}

/// A resolution's class for the counts.
const fn resolution_class(r: &LineResolution) -> &'static str {
    match r {
        LineResolution::Pending => "pending",
        LineResolution::Excluded(_) => "excluded",
        LineResolution::Resolved { adapted: None, .. } => "resolved",
        LineResolution::Resolved {
            adapted: Some(_), ..
        } => "resolved-adapted",
        LineResolution::Continues { .. } => "continues",
        LineResolution::Unresolved(_) => "unresolved",
    }
}

/// Every typed item's ID and source lines, in order.
fn item_lines(b: &Build) -> Vec<(u32, Vec<String>)> {
    b.items
        .items()
        .map(|i| {
            (
                i.id.0,
                i.body.lines.iter().map(|l| l.source.clone()).collect(),
            )
        })
        .collect()
}

/// Every `PathOfBuilding/Items/Item`'s `id` attribute and the lines of its
/// last non-blank text child, as `pob_item::read` splits them.
fn xml_item_lines(text: &str) -> Vec<(String, Vec<String>)> {
    let Ok(doc) = roxmltree::Document::parse(text) else {
        return Vec::new();
    };
    let root = doc.root_element();
    root.children()
        .filter(|n| n.has_tag_name("Items"))
        .flat_map(|items| items.children().filter(|n| n.has_tag_name("Item")))
        .map(|item| {
            let body = item
                .children()
                .filter(roxmltree::Node::is_text)
                .filter_map(|n| n.text())
                .rfind(|t| !t.trim().is_empty())
                .unwrap_or("");
            let lines = body
                .split('\n')
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_owned)
                .collect();
            (item.attribute("id").unwrap_or("").to_owned(), lines)
        })
        .collect()
}

/// Whether every typed item of `out` has the input lines of the item with
/// its ID (in input order for repeated IDs). Pushes a failure per item.
fn same_lines(
    input: &[(String, Vec<String>)],
    out: &Build,
    label: &str,
    failures: &mut Vec<String>,
) -> bool {
    let mut by_id: BTreeMap<u32, VecDeque<&Vec<String>>> = BTreeMap::new();
    for (id, lines) in input {
        if let Ok(id) = id.parse::<u32>() {
            by_id.entry(id).or_default().push_back(lines);
        }
    }
    let mut ok = true;
    for (id, lines) in item_lines(out) {
        let problem = match by_id.get_mut(&id).and_then(VecDeque::pop_front) {
            None => Some("not in the input".to_owned()),
            Some(want) if *want == lines => None,
            Some(want) if want.len() != lines.len() => Some(format!(
                "{} source lines in, {} out",
                want.len(),
                lines.len()
            )),
            Some(want) => want
                .iter()
                .zip(&lines)
                .position(|(w, l)| w != l)
                .map(|n| format!("line {n} differs")),
        };
        if let Some(p) = problem {
            ok = false;
            failures.push(format!("(d) {label} item {id}: {p}"));
        }
    }
    ok
}

/// The mutation set (module [`mutate`]).
fn mutations(row: &mut BuildRow, failures: &mut Vec<String>, text: &str) {
    let set = match mutate::mutations(text) {
        Ok(s) => s,
        Err(e) => {
            failures.push(format!("mutations: {e}"));
            return;
        }
    };
    for m in set {
        let count = row.mutations.entry(m.kind.name()).or_default();
        count.tried += 1;
        match survives(&m.xml) {
            Ok(()) => count.survived += 1,
            Err(problem) => failures.push(format!(
                "mutation {} at {}: {problem}",
                m.kind.name(),
                m.path
            )),
        }
    }
}

/// Whether a mutated document survives XML to JSON to XML.
fn survives(xml: &str) -> Result<(), String> {
    let a = import(xml).map_err(|e| format!("import: {}", e.code()))?;
    let json = to_json(&a).map_err(|e| format!("write: {}", e.code()))?;
    let b = from_json(&json).map_err(|e| format!("read: {}", e.code()))?;
    if b != a {
        return Err("the build read back differs".to_owned());
    }
    let out = export(&b);
    match xml_equal(&out, xml) {
        Ok(true) => Ok(()),
        Ok(false) => Err("lost or changed in the export".to_owned()),
        Err(e) => Err(format!("export is not XML: {e}")),
    }
}
