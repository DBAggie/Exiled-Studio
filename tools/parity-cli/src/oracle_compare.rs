//! `oracle-compare`: compares two canonical output documents, or every
//! fixture of two `oracle-run` output directories, field by field, and
//! classifies each difference with the divergence register (Spike C plan,
//! step 3; ADR-006).
//!
//! The left side is the oracle, the right side the candidate (the
//! successor in Phase 3; another oracle run, platform, or fixture today).
//! Only the computed data is compared ([`COMPARED`]); the rest of the
//! document records the run ([`NOT_COMPARED`]). Comparison is exact
//! (numbers as doubles) unless an accepted register entry names a
//! tolerance for the field.
//!
//! Classes: a difference covered by an accepted entry is
//! `ORACLE_EXPECTED_DIFFERENCE`, any other `ORACLE_UNEXPECTED_DIFFERENCE`;
//! a fixture without output on a side (it failed there) is not compared
//! and carries that side's class, so an environment failure is never read
//! as a difference, unless both sides failed with the class their fixture
//! manifest expects (`malformed.xml`), which is a match. A fixture present
//! on one side only is unexpected. The report's class is the smallest
//! failure class (10 to 18) if there is one, else the larger difference
//! class: one unregistered difference makes the whole report unexpected.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use crate::divergence::{self, Register, Status};
use crate::oracle::{self, FailureClass};

/// The roots of the output document that are compared: the outcome and
/// the computed data.
pub const COMPARED: [&str; 4] = ["stage", "partial", "main_output", "breakdowns"];

/// The other roots, and why they are not compared.
pub const NOT_COMPARED: [(&str, &str); 7] = [
    (
        "protocol",
        "must be equal on both sides (checked before comparing)",
    ),
    (
        "error",
        "null in every output document (only ok and partial runs have one)",
    ),
    ("modules", "the run's environment, checked by oracle-run"),
    ("settings", "the run's environment, checked by oracle-run"),
    (
        "shims",
        "the run's environment (the zlib version differs per platform)",
    ),
    (
        "probe",
        "the shims' record; a missing jewel load shows as partial",
    ),
    ("breakdown_source", "a constant of the protocol"),
];

/// One field that differs.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Difference {
    /// JSON pointer of the field.
    pub field: String,
    /// `changed`, `only_left`, or `only_right`.
    pub change: &'static str,
    /// The left value (absent: not in the left document).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<Value>,
    /// The right value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<Value>,
    /// Expected or unexpected difference.
    pub class: FailureClass,
    /// The register entry that covers it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    /// Why an entry for this field does not cover it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The fields that differ between two documents, in document order of
/// [`COMPARED`] and then sorted keys, unclassified.
pub fn diff(left: &Value, right: &Value) -> Vec<(String, Option<Value>, Option<Value>)> {
    let mut out = Vec::new();
    for root in COMPARED {
        walk(
            &format!("/{root}"),
            left.get(root),
            right.get(root),
            &mut out,
        );
    }
    out
}

fn walk(
    path: &str,
    left: Option<&Value>,
    right: Option<&Value>,
    out: &mut Vec<(String, Option<Value>, Option<Value>)>,
) {
    match (left, right) {
        (None, None) => {}
        (Some(Value::Object(l)), Some(Value::Object(r))) => {
            let keys: BTreeSet<&String> = l.keys().chain(r.keys()).collect();
            for key in keys {
                let child = format!("{path}/{}", divergence::pointer_escape(key));
                walk(&child, l.get(key), r.get(key), out);
            }
        }
        (Some(Value::Array(l)), Some(Value::Array(r))) => {
            for i in 0..l.len().max(r.len()) {
                walk(&format!("{path}/{i}"), l.get(i), r.get(i), out);
            }
        }
        (l, r) => {
            let equal = matches!((l, r), (Some(a), Some(b)) if divergence::same_value(a, b));
            if !equal {
                out.push((path.to_owned(), l.cloned(), r.cloned()));
            }
        }
    }
}

/// Classifies the differences of one fixture with the register's accepted
/// entries. Returns the differences and the ids of the entries that
/// covered one.
pub fn classify(
    raw: Vec<(String, Option<Value>, Option<Value>)>,
    id: &str,
    sha256: &[&str],
    register: Option<&Register>,
) -> (Vec<Difference>, BTreeSet<String>) {
    let mut used = BTreeSet::new();
    let differences = raw
        .into_iter()
        .map(|(field, left, right)| {
            let change = match (&left, &right) {
                (Some(_), Some(_)) => "changed",
                (Some(_), None) => "only_left",
                _ => "only_right",
            };
            let entry = register.and_then(|r| {
                r.entries.iter().find(|e| {
                    e.status == Status::Accepted && e.field == field && e.names_fixture(id, sha256)
                })
            });
            let (class, entry, note) =
                match entry.map(|e| (e, e.covers(left.as_ref(), right.as_ref()))) {
                    Some((e, Ok(()))) => {
                        used.insert(e.id.clone());
                        (FailureClass::ExpectedDifference, Some(e.id.clone()), None)
                    }
                    Some((_, Err(why))) => (FailureClass::UnexpectedDifference, None, Some(why)),
                    None => (FailureClass::UnexpectedDifference, None, None),
                };
            Difference {
                field,
                change,
                left,
                right,
                class,
                entry,
                note,
            }
        })
        .collect();
    (differences, used)
}

/// One fixture's comparison.
#[derive(Debug, Serialize)]
pub struct FixtureComparison {
    /// Fixture id.
    pub id: String,
    /// SHA-256 of the fixture bytes, per side (runs mode).
    pub left_sha256: Option<String>,
    /// See `left_sha256`.
    pub right_sha256: Option<String>,
    /// SHA-256 of each side's output document, when it has one.
    pub left_output_sha256: Option<String>,
    /// See `left_output_sha256`.
    pub right_output_sha256: Option<String>,
    /// `identical`, `expected`, `unexpected`, `expected_failure` (both
    /// sides failed with the class their fixture manifest expects), or
    /// `not_compared`.
    pub status: &'static str,
    /// The two sides ran different bytes (`sha256` differs). Expected for
    /// PoB's fixtures across platforms (CRLF on Windows, LF on Linux);
    /// for anything else the inputs differ.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub input_differs: bool,
    /// The fixture's class: the larger difference class, or the class of
    /// the side that has no output.
    pub class: Option<FailureClass>,
    /// Why it was not compared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Every differing field.
    pub differences: Vec<Difference>,
}

impl FixtureComparison {
    fn of(
        id: &str,
        sha256: (Option<String>, Option<String>),
        outputs: (&Output, &Output),
        register: Option<&Register>,
    ) -> (Self, BTreeSet<String>) {
        let raw = diff(&outputs.0.value, &outputs.1.value);
        let hashes: Vec<&str> = [&sha256.0, &sha256.1]
            .into_iter()
            .filter_map(|h| h.as_deref())
            .collect();
        let (differences, used) = classify(raw, id, &hashes, register);
        let class = differences.iter().map(|d| d.class).max();
        let status = match class {
            None => "identical",
            Some(FailureClass::ExpectedDifference) => "expected",
            Some(_) => "unexpected",
        };
        (
            Self {
                id: id.to_owned(),
                input_differs: matches!(&sha256, (Some(l), Some(r)) if l != r),
                left_sha256: sha256.0,
                right_sha256: sha256.1,
                left_output_sha256: Some(outputs.0.sha256.clone()),
                right_output_sha256: Some(outputs.1.sha256.clone()),
                status,
                class,
                reason: None,
                differences,
            },
            used,
        )
    }
}

/// A side of the comparison as recorded in the report (no paths).
#[derive(Debug, Clone, Serialize)]
pub struct SideRecord {
    /// SHA-256 of the output document (files) or of `report.json` (runs).
    pub sha256: String,
    /// The run's platform (runs mode).
    pub platform: Option<String>,
    /// The run's tool commit (runs mode).
    pub tool_commit: Option<String>,
}

/// The register used.
#[derive(Debug, Clone, Serialize)]
pub struct RegisterRecord {
    /// SHA-256 of the register file.
    pub sha256: String,
    /// Entries.
    pub entries: usize,
    /// Accepted entries.
    pub accepted: usize,
}

/// A root that is not compared, and why.
#[derive(Debug, Serialize)]
pub struct NotCompared {
    /// The root.
    pub field: &'static str,
    /// Why.
    pub reason: &'static str,
}

/// Fixture counts.
#[derive(Debug, Default, Serialize)]
pub struct Counts {
    /// Fixtures paired.
    pub fixtures: usize,
    /// Identical.
    pub identical: usize,
    /// Only expected differences.
    pub expected: usize,
    /// At least one unexpected difference.
    pub unexpected: usize,
    /// Both sides failed as their fixture manifest expects.
    pub expected_failures: usize,
    /// Without output on a side.
    pub not_compared: usize,
    /// Expected differences over all fixtures.
    pub expected_differences: usize,
    /// Unexpected differences over all fixtures.
    pub unexpected_differences: usize,
}

/// `report.json` of `oracle-compare`.
#[derive(Debug, Serialize)]
pub struct Report {
    /// Report format; 1.
    pub format: u32,
    /// `identical`, `expected` (only registered differences), `unexpected`,
    /// or `failed` (a fixture has no output on a side).
    pub status: &'static str,
    /// The smallest class over the fixtures and the unpaired ones.
    pub class: Option<FailureClass>,
    /// `files` (two output documents) or `runs` (two oracle-run
    /// directories).
    pub mode: &'static str,
    /// The left (oracle) side.
    pub left: SideRecord,
    /// The right side.
    pub right: SideRecord,
    /// The register, when one was given.
    pub register: Option<RegisterRecord>,
    /// Compared roots.
    pub compared: [&'static str; 4],
    /// Roots not compared.
    pub not_compared: Vec<NotCompared>,
    /// Counts.
    pub counts: Counts,
    /// Every paired fixture.
    pub fixtures: Vec<FixtureComparison>,
    /// Fixtures on one side only, as `(id, side)`.
    pub unpaired: Vec<(String, &'static str)>,
    /// Accepted entries for a compared fixture that covered no difference
    /// (the difference is gone; the entry may be retired).
    pub unused_entries: Vec<String>,
}

impl Report {
    fn new(mode: &'static str, left: SideRecord, right: SideRecord) -> Self {
        Self {
            format: 1,
            status: "identical",
            class: None,
            mode,
            left,
            right,
            register: None,
            compared: COMPARED,
            not_compared: NOT_COMPARED
                .iter()
                .map(|&(field, reason)| NotCompared { field, reason })
                .collect(),
            counts: Counts::default(),
            fixtures: Vec::new(),
            unpaired: Vec::new(),
            unused_entries: Vec::new(),
        }
    }

    /// Counts, class, status, and unused entries from the fixtures.
    fn finish(mut self, register: Option<&Register>, used: &BTreeSet<String>) -> Self {
        let mut counts = Counts {
            fixtures: self.fixtures.len(),
            ..Counts::default()
        };
        for f in &self.fixtures {
            match f.status {
                "identical" => counts.identical += 1,
                "expected" => counts.expected += 1,
                "unexpected" => counts.unexpected += 1,
                "expected_failure" => counts.expected_failures += 1,
                _ => counts.not_compared += 1,
            }
            for d in &f.differences {
                if d.class == FailureClass::ExpectedDifference {
                    counts.expected_differences += 1;
                } else {
                    counts.unexpected_differences += 1;
                }
            }
        }
        self.counts = counts;
        let classes: Vec<FailureClass> = self
            .fixtures
            .iter()
            .filter_map(|f| f.class)
            .chain((!self.unpaired.is_empty()).then_some(FailureClass::UnexpectedDifference))
            .collect();
        self.class = report_class(&classes);
        self.status = match self.class {
            None => "identical",
            Some(FailureClass::ExpectedDifference) => "expected",
            Some(FailureClass::UnexpectedDifference) => "unexpected",
            Some(_) => "failed",
        };
        if let Some(register) = register {
            self.unused_entries = register
                .entries
                .iter()
                .filter(|e| e.status == Status::Accepted && !used.contains(&e.id))
                .filter(|e| {
                    self.fixtures.iter().any(|f| {
                        matches!(f.status, "identical" | "expected" | "unexpected")
                            && e.names_fixture(
                                &f.id,
                                &[&f.left_sha256, &f.right_sha256]
                                    .into_iter()
                                    .filter_map(|h| h.as_deref())
                                    .collect::<Vec<_>>(),
                            )
                    })
                })
                .map(|e| e.id.clone())
                .collect();
        }
        self
    }

    /// A short Markdown summary.
    pub fn summary_markdown(&self) -> String {
        let c = &self.counts;
        let mut text = format!(
            "# oracle-compare\n\nStatus: **{}**{}. Mode `{}`; {} fixtures: {} identical, {} expected, \
             {} unexpected, {} expected failures, {} not compared; differences: {} expected, {} \
             unexpected.\n",
            self.status,
            self.class
                .map(|c| format!(" ({})", c.name()))
                .unwrap_or_default(),
            self.mode,
            c.fixtures,
            c.identical,
            c.expected,
            c.unexpected,
            c.expected_failures,
            c.not_compared,
            c.expected_differences,
            c.unexpected_differences,
        );
        for f in self
            .fixtures
            .iter()
            .filter(|f| !matches!(f.status, "identical" | "expected_failure"))
        {
            text.push_str(&format!(
                "\n## {} ({})\n\n",
                f.id.replace(['\r', '\n'], " "),
                f.status
            ));
            if f.input_differs {
                text.push_str("The two sides ran different fixture bytes.\n\n");
            }
            if let Some(reason) = &f.reason {
                text.push_str(&format!("{reason}\n"));
                continue;
            }
            text.push_str("| Field | Left | Right | Class | Entry |\n|---|---|---|---|---|\n");
            let cell = |v: &Option<Value>| {
                v.as_ref()
                    .map_or("(absent)".to_owned(), short)
                    .replace('|', "\\|")
            };
            for d in &f.differences {
                text.push_str(&format!(
                    "| `{}` | {} | {} | {} | {} |\n",
                    d.field,
                    cell(&d.left),
                    cell(&d.right),
                    d.class.name(),
                    d.entry
                        .as_deref()
                        .or(d.note.as_deref())
                        .unwrap_or("-")
                        .replace('|', "\\|"),
                ));
            }
        }
        if !self.unpaired.is_empty() {
            text.push_str("\nOn one side only:\n");
            for (id, side) in &self.unpaired {
                text.push_str(&format!("- {id} ({side})\n"));
            }
        }
        if !self.unused_entries.is_empty() {
            text.push_str(&format!(
                "\nAccepted entries that covered nothing: {}\n",
                self.unused_entries.join(", ")
            ));
        }
        text
    }
}

/// The report's class from the fixtures' (and unpaired fixtures') classes:
/// the smallest failure class of a run (10 to 18) when there is one, else
/// the larger difference class. A minimum over all classes would let an
/// expected difference (19) hide an unexpected one (20).
fn report_class(classes: &[FailureClass]) -> Option<FailureClass> {
    let difference = |c: &&FailureClass| {
        matches!(
            c,
            FailureClass::ExpectedDifference | FailureClass::UnexpectedDifference
        )
    };
    classes
        .iter()
        .filter(|c| !difference(c))
        .min()
        .or_else(|| classes.iter().filter(difference).max())
        .copied()
}

/// A value for a table cell: scalars as JSON, containers shortened.
fn short(value: &Value) -> String {
    let text = value.to_string();
    if text.chars().count() <= 60 {
        text
    } else {
        format!("{}...", text.chars().take(57).collect::<String>())
    }
}

/// What `oracle-compare` is asked to do.
pub struct Request {
    /// Left output document or oracle-run directory.
    pub left: PathBuf,
    /// Right output document or oracle-run directory.
    pub right: PathBuf,
    /// The divergence register, if any.
    pub register: Option<PathBuf>,
    /// Fixture id for files mode (default: the right file name without
    /// `.json`).
    pub fixture: Option<String>,
    /// Output directory; must not exist.
    pub out: PathBuf,
}

/// An output document and the SHA-256 of its file.
pub struct Output {
    /// The document.
    pub value: Value,
    /// SHA-256 of the file's bytes.
    pub sha256: String,
}

/// Reads an output document of a complete run: a JSON object with this
/// reader's protocol, exactly the protocol's fields, stage `done`, and
/// objects for `main_output` and `breakdowns` (so a truncated document is
/// refused rather than compared as empty).
fn read_output(path: &Path) -> Result<Output, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("{}: not JSON: {e}", path.display()))?;
    let bad = |what: String| Err(format!("{}: {what}", path.display()));
    if value.get("protocol").and_then(Value::as_u64) != Some(u64::from(oracle::PROTOCOL)) {
        return bad(format!(
            "not an output document of adapter protocol {}",
            oracle::PROTOCOL
        ));
    }
    let Some(top) = value.as_object() else {
        return bad("not an object".to_owned());
    };
    let mut keys: Vec<&str> = top.keys().map(String::as_str).collect();
    keys.sort_unstable();
    let mut expected = crate::oracle_output::FIELDS.to_vec();
    expected.sort_unstable();
    if keys != expected {
        return bad(format!("expected exactly the fields {expected:?}"));
    }
    if value["stage"] != "done"
        || !value["main_output"].is_object()
        || !value["breakdowns"].is_object()
    {
        return bad(
            "not the output of a complete run (stage done, main_output, breakdowns)".to_owned(),
        );
    }
    Ok(Output {
        value,
        sha256: oracle::sha256_hex(&bytes),
    })
}

/// A fixture as an oracle-run report lists it.
pub struct RunFixture {
    /// The fixture id.
    pub id: String,
    /// SHA-256 of its bytes.
    pub sha256: Option<String>,
    /// `ok`, `partial`, or `failed`.
    pub status: String,
    /// A failed fixture's class.
    pub class: Option<FailureClass>,
    /// The outcome the fixture manifest named.
    pub expect: Option<String>,
    /// Whether it was met.
    pub met: Option<bool>,
}

/// An oracle-run directory: its report's fixtures and record.
pub struct RunDir {
    dir: PathBuf,
    fixtures: Vec<RunFixture>,
    record: SideRecord,
}

/// Reads an `oracle-run` directory's report (`calc-parity` reads it too).
pub fn read_run(dir: &Path) -> Result<RunDir, String> {
    let path = dir.join("report.json");
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let report: Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("{}: not JSON: {e}", path.display()))?;
    let text = |v: &Value| v.as_str().map(str::to_owned);
    let fixtures = report
        .get("fixtures")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{}: no fixtures list", path.display()))?
        .iter()
        .map(|f| {
            let id =
                text(&f["id"]).ok_or_else(|| format!("{}: a fixture has no id", path.display()))?;
            // Joined into a path below: a file name only.
            if id.is_empty() || id == "." || id == ".." || id.contains(['/', '\\']) {
                return Err(format!(
                    "{}: fixture id {id:?} is not a file name",
                    path.display()
                ));
            }
            Ok(RunFixture {
                id,
                sha256: text(&f["sha256"]),
                status: text(&f["status"]).unwrap_or_default(),
                class: serde_json::from_value(f["class"].clone()).unwrap_or(None),
                expect: text(&f["expect"]),
                met: f["met"].as_bool(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(RunDir {
        dir: dir.to_path_buf(),
        fixtures,
        record: SideRecord {
            sha256: oracle::sha256_hex(&bytes),
            platform: text(&report["platform"]),
            tool_commit: text(&report["tool"]["commit"]),
        },
    })
}

impl RunDir {
    /// A fixture's output document.
    pub fn output(&self, id: &str) -> Result<Output, String> {
        read_output(&self.dir.join("outputs").join(format!("{id}.json")))
    }

    /// The fixtures, in the report's order.
    pub fn fixtures(&self) -> &[RunFixture] {
        &self.fixtures
    }

    /// The run's record.
    pub const fn record(&self) -> &SideRecord {
        &self.record
    }
}

/// Compares and builds the report (nothing is written).
pub fn compare(request: &Request) -> Result<Report, String> {
    let register = match &request.register {
        Some(path) => {
            let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
            let register = Register::parse(&bytes).map_err(|problems| {
                format!(
                    "{}: invalid register: {}",
                    path.display(),
                    problems.join("; ")
                )
            })?;
            Some((register, oracle::sha256_hex(&bytes)))
        }
        None => None,
    };
    let reg = register.as_ref().map(|(r, _)| r);
    let mut used = BTreeSet::new();
    let mut report = match (request.left.is_dir(), request.right.is_dir()) {
        (false, false) => {
            let left = read_output(&request.left)?;
            let right = read_output(&request.right)?;
            let id = match &request.fixture {
                Some(id) => id.clone(),
                None => request
                    .right
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .map(|n| n.strip_suffix(".json").unwrap_or(&n).to_owned())
                    .ok_or_else(|| "--fixture <id> is required".to_owned())?,
            };
            let mut report = Report::new(
                "files",
                SideRecord {
                    sha256: left.sha256.clone(),
                    platform: None,
                    tool_commit: None,
                },
                SideRecord {
                    sha256: right.sha256.clone(),
                    platform: None,
                    tool_commit: None,
                },
            );
            let (fixture, covered) = FixtureComparison::of(&id, (None, None), (&left, &right), reg);
            used.extend(covered);
            report.fixtures.push(fixture);
            report
        }
        (true, true) => {
            if request.fixture.is_some() {
                return Err("--fixture applies to two output documents only".to_owned());
            }
            let left = read_run(&request.left)?;
            let right = read_run(&request.right)?;
            let mut report = Report::new("runs", left.record.clone(), right.record.clone());
            for l in &left.fixtures {
                let Some(r) = right.fixtures.iter().find(|r| r.id == l.id) else {
                    report.unpaired.push((l.id.clone(), "left"));
                    continue;
                };
                let has_output = |f: &RunFixture| matches!(f.status.as_str(), "ok" | "partial");
                if !(has_output(l) && has_output(r)) {
                    let describe =
                        |f: &RunFixture| f.class.map_or(f.status.clone(), |c| c.name().to_owned());
                    // Both sides failed as the fixture manifest expects
                    // (the same expectation, met on both): a match.
                    let expected_failure = l.status == "failed"
                        && r.status == "failed"
                        && l.met == Some(true)
                        && r.met == Some(true)
                        && l.expect.is_some()
                        && l.expect == r.expect
                        && l.class == r.class;
                    let (status, class, reason) = if expected_failure {
                        (
                            "expected_failure",
                            None,
                            format!("both sides failed as expected ({})", describe(l)),
                        )
                    } else {
                        let failed: Vec<&RunFixture> =
                            [l, r].into_iter().filter(|f| !has_output(f)).collect();
                        let sides = match (has_output(l), has_output(r)) {
                            (true, _) => format!("the right side ({})", describe(r)),
                            (_, true) => format!("the left side ({})", describe(l)),
                            _ => {
                                format!("either side (left {}, right {})", describe(l), describe(r))
                            }
                        };
                        (
                            "not_compared",
                            // A failed run without a class cannot be written
                            // by oracle-run; treat it as a protocol problem.
                            failed
                                .iter()
                                .map(|f| f.class.unwrap_or(FailureClass::Protocol))
                                .min(),
                            format!("no output on {sides}"),
                        )
                    };
                    report.fixtures.push(FixtureComparison {
                        id: l.id.clone(),
                        input_differs: l.sha256 != r.sha256,
                        left_sha256: l.sha256.clone(),
                        right_sha256: r.sha256.clone(),
                        left_output_sha256: None,
                        right_output_sha256: None,
                        status,
                        class,
                        reason: Some(reason),
                        differences: Vec::new(),
                    });
                    continue;
                }
                let (fixture, covered) = FixtureComparison::of(
                    &l.id,
                    (l.sha256.clone(), r.sha256.clone()),
                    (&left.output(&l.id)?, &right.output(&r.id)?),
                    reg,
                );
                used.extend(covered);
                report.fixtures.push(fixture);
            }
            for r in &right.fixtures {
                if !left.fixtures.iter().any(|l| l.id == r.id) {
                    report.unpaired.push((r.id.clone(), "right"));
                }
            }
            report
        }
        _ => {
            return Err(
                "compare two output documents or two oracle-run directories, not one of each"
                    .to_owned(),
            );
        }
    };
    report.register = register.as_ref().map(|(r, sha256)| RegisterRecord {
        sha256: sha256.clone(),
        entries: r.entries.len(),
        accepted: r
            .entries
            .iter()
            .filter(|e| e.status == Status::Accepted)
            .count(),
    });
    Ok(report.finish(reg, &used))
}

/// Compares and writes `report.json` and `summary.md` to `out`, which
/// must not exist.
pub fn run(request: &Request) -> Result<Report, String> {
    if request.out.exists() {
        return Err(format!("{}: already exists", request.out.display()));
    }
    let report = compare(request)?;
    std::fs::create_dir_all(&request.out).map_err(|e| format!("{}: {e}", request.out.display()))?;
    let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())? + "\n";
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
