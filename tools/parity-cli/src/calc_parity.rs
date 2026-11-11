//! `calc-parity` (Phase 3 plan, step 5, decision 9): the successor's
//! calc result documents (`calc-run`) against the oracle's outputs
//! (`oracle-run`), per build and per slice field
//! (`docs/phase-3/slice-fields.json`), classified with the divergence
//! register.
//!
//! **Fields.** Only `complete` fields are compared, with the field's rule
//! (`exact`, or `abs`/`rel` with a written reason). A `partial` or
//! `indeterminate` field is listed with its reasons and not compared; an
//! `unsupported` one (or one the result does not have) is listed as not
//! computed, never as a pass. A `complete` field the oracle does not
//! write is a difference.
//!
//! **Breakdowns.** For a `complete` field with one of the oracle's ten
//! breakdowns, PoB's display lines are read into terms
//! ([`breakdown`]) and each term is compared with the node of our graph
//! the slice file names (`graph`), printed the way PoB prints the line,
//! so a term is compared at the decimals its line shows. A line the
//! reader does not know fails the build with `ORACLE_NORMALIZATION`.
//! PoB's breakdowns come from its CALCS view and `main_output` from its
//! main view, which select the main skill separately; when a chain
//! breakdown's total is not the main output's value, the views computed
//! different things and the build's breakdowns are not compared
//! (`views_differ`).
//!
//! **Register.** Each difference is a JSON pointer into the oracle's
//! output document (`/main_output/<key>`, `/breakdowns/<key>/<line>`,
//! `/breakdowns/<key>/slots/<row>/<part>`; a term only our graph has is
//! named `/breakdowns/<key>/<term>`) and is expected when an accepted
//! entry covers it (`oracle_compare::classify`). An entry of kind
//! `data-version` covers a difference only when the `--overlay-run`
//! (`calc-run --data-overlay`, PoB's data values) matches the oracle on
//! that pointer (decision 5); otherwise the difference stays unexpected.
//!
//! **Mechanic fixtures.** With `--manifest`, each `mechanic` fixture's
//! `targets` must each match or be registered (gate item 2).
//!
//! Classes and exit codes as `oracle-compare`: a build without output on
//! a side carries that side's class (the successor's: `ORACLE_IMPORT` for
//! a build that does not import, `ORACLE_CALCULATION` for resolution and
//! the engine, `ORACLE_PROTOCOL` for its document), unless both sides
//! failed to import a fixture whose manifest expects it; the report's
//! class is the smallest failure class, else the larger difference class.

pub mod breakdown;
pub mod slice;

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed scripted documents; a bad index should panic"
)]
mod tests;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use loom_serialization::dto::calc::v1 as calc_dto;
use loom_serialization::dto::v1::Number;
use serde::Serialize;
use serde_json::Value;

use self::breakdown::{Format, Printed};
use self::slice::{BreakdownSpec, Shape, SliceField, SliceFile};
use crate::calc_run::{self, BuildRecord, Step};
use crate::divergence::{self, Kind, Register, Status};
use crate::oracle::{self, FailureClass};
use crate::oracle_compare::{RegisterRecord, SideRecord};
use crate::oracle_fixtures::{Manifest, Tier};

/// Report format.
pub const FORMAT: u32 = 1;

/// What a field comparison found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    /// Equal.
    Match,
    /// Within the field's `abs` or `rel` rule.
    WithinTolerance,
    /// Different (see the class).
    Difference,
    /// Our field is `partial`: not compared.
    Partial,
    /// Our field is `indeterminate`: not compared.
    Indeterminate,
    /// Our field is not computed (or not in the result).
    NotComputed,
}

/// One slice field of one build.
#[derive(Debug, Clone, Serialize)]
pub struct FieldComparison {
    /// PoB's id.
    pub field: String,
    /// Our name.
    pub ours: &'static str,
    /// The pointer a register entry names.
    pub pointer: String,
    /// What was found.
    pub outcome: Outcome,
    /// Our state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// The oracle's value (absent: not written).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oracle: Option<Value>,
    /// Our value (absent: not computed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub successor: Option<Value>,
    /// Our reasons' kinds, for a field not compared.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
    /// A difference's class.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<FailureClass>,
    /// The entry that covers it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    /// Why an entry for it does not cover it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// What a breakdown comparison found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BreakdownOutcome {
    /// Every term equal.
    Match,
    /// A term differs.
    Difference,
    /// Not compared (see the reason).
    NotCompared,
    /// The oracle wrote none, and ours has nothing PoB would print.
    Absent,
}

/// One term of a breakdown.
#[derive(Debug, Clone, Serialize)]
pub struct TermComparison {
    /// The pointer a register entry names.
    pub pointer: String,
    /// The term (or slot row part).
    pub term: String,
    /// As PoB printed it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oracle: Option<String>,
    /// Ours, printed the same way.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub successor: Option<String>,
    /// Whether they are equal.
    pub equal: bool,
    /// A difference's class.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<FailureClass>,
    /// The entry that covers it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    /// Why an entry for it does not cover it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// One breakdown of one build.
#[derive(Debug, Clone, Serialize)]
pub struct BreakdownComparison {
    /// The slice field's PoB id.
    pub field: String,
    /// The breakdown's key.
    pub pob_key: String,
    /// What was found.
    pub outcome: BreakdownOutcome,
    /// Why it was not compared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Every term compared.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub terms: Vec<TermComparison>,
}

/// A mechanic fixture's target field.
#[derive(Debug, Clone, Serialize)]
pub struct Target {
    /// PoB's id.
    pub field: String,
    /// Matched, within its rule, or registered.
    pub met: bool,
}

/// One build.
#[derive(Debug, Serialize)]
pub struct BuildComparison {
    /// The id both runs name it by.
    pub id: String,
    /// SHA-256 of the build on the oracle side.
    pub oracle_sha256: Option<String>,
    /// SHA-256 on the successor side.
    pub successor_sha256: Option<String>,
    /// The two sides read different bytes.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub input_differs: bool,
    /// `no_difference`, `expected`, `unexpected`, `failed` (a breakdown
    /// the reader does not know), `expected_failure`, or `not_compared`.
    pub status: &'static str,
    /// The build's class.
    pub class: Option<FailureClass>,
    /// Why it was not compared, or failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Our result's state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// Where PoB's CALCS and main views disagree (breakdowns then not
    /// compared).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub views_differ: Vec<String>,
    /// Every slice field.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<FieldComparison>,
    /// Every breakdown.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub breakdowns: Vec<BreakdownComparison>,
    /// A mechanic fixture's targets.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<Target>,
}

/// Counts.
#[derive(Debug, Default, Serialize)]
pub struct Counts {
    /// Builds paired.
    pub builds: usize,
    /// No difference.
    pub no_difference: usize,
    /// Only expected differences.
    pub expected: usize,
    /// An unexpected difference.
    pub unexpected: usize,
    /// A breakdown that did not read.
    pub failed: usize,
    /// Both sides failed to import, as expected.
    pub expected_failures: usize,
    /// A side has no output.
    pub not_compared: usize,
    /// Field outcomes over all builds.
    pub fields: BTreeMap<String, usize>,
    /// Breakdown outcomes over all builds.
    pub breakdowns: BTreeMap<String, usize>,
    /// Builds whose views differ.
    pub views_differ: usize,
    /// Expected differences (fields and terms).
    pub expected_differences: usize,
    /// Unexpected differences.
    pub unexpected_differences: usize,
}

/// The slice file used.
#[derive(Debug, Serialize)]
pub struct SliceRecord {
    /// SHA-256 of the file.
    pub sha256: String,
    /// Its revision.
    pub revision: u32,
    /// `proposed` or `confirmed`.
    pub status: String,
}

/// `report.json`.
#[derive(Debug, Serialize)]
pub struct Report {
    /// [`FORMAT`].
    pub format: u32,
    /// `no_difference`, `expected`, `unexpected`, or `failed`.
    pub status: &'static str,
    /// The smallest failure class, else the larger difference class.
    pub class: Option<FailureClass>,
    /// The oracle run.
    pub oracle: SideRecord,
    /// The successor run.
    pub successor: SideRecord,
    /// The overlay run, if given.
    pub overlay: Option<SideRecord>,
    /// The bundle both successor runs read.
    pub bundle: calc_run::BundleRecord,
    /// The slice file.
    pub slice: SliceRecord,
    /// The register, if given.
    pub register: Option<RegisterRecord>,
    /// Counts.
    pub counts: Counts,
    /// Every paired build.
    pub builds: Vec<BuildComparison>,
    /// Builds on one side only, as `(id, side)`.
    pub unpaired: Vec<(String, &'static str)>,
    /// Accepted entries for a compared build that covered nothing.
    pub unused_entries: Vec<String>,
}

/// What `calc-parity` is asked to do.
#[derive(Debug, Clone)]
pub struct Request {
    /// The `oracle-run` directory.
    pub oracle: PathBuf,
    /// The `calc-run` directory.
    pub successor: PathBuf,
    /// The slice file.
    pub slice: PathBuf,
    /// The register.
    pub register: Option<PathBuf>,
    /// A `calc-run --data-overlay` directory.
    pub overlay_run: Option<PathBuf>,
    /// The fixture manifest (mechanic targets).
    pub manifest: Option<PathBuf>,
    /// Output directory; must not exist.
    pub out: PathBuf,
}

/// A number of the document as a double.
fn number(n: &Number) -> f64 {
    match n {
        Number::Finite(v) => *v,
        Number::NonFinite(b) => {
            u64::from_str_radix(&b.f64_bits, 16).map_or(f64::NAN, f64::from_bits)
        }
    }
}

/// A double as JSON (non-finite as C prints it).
fn json_number(v: f64) -> Value {
    serde_json::Number::from_f64(v)
        .map_or_else(|| Value::String(breakdown::g(v, 17)), Value::Number)
}

/// Our side: a calc result document.
struct Ours<'d> {
    doc: &'d calc_dto::Document,
}

impl<'d> Ours<'d> {
    fn field(&self, name: &str) -> Option<&'d calc_dto::FieldResult> {
        self.doc.result.fields.iter().find(|f| f.field == name)
    }

    /// A field's value when it is `complete`.
    fn complete(&self, name: &str) -> Option<(f64, u32)> {
        let f = self.field(name)?;
        if !matches!(f.state, calc_dto::StateKind::Complete) {
            return None;
        }
        Some((number(f.value.as_ref()?), f.node?))
    }

    fn node_value(&self, i: u32) -> Option<f64> {
        self.doc
            .result
            .graph
            .get(usize::try_from(i).ok()?)
            .map(|n| number(&n.value))
    }

    /// The nodes below `root` (and `root`), nearest first, children in
    /// order, each once.
    fn below(&self, root: u32) -> Vec<u32> {
        let graph = &self.doc.result.graph;
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        let mut queue = VecDeque::from([root]);
        while let Some(i) = queue.pop_front() {
            if !seen.insert(i) {
                continue;
            }
            out.push(i);
            if let Some(n) = usize::try_from(i).ok().and_then(|u| graph.get(u)) {
                queue.extend(n.children.iter().copied());
            }
        }
        out
    }

    /// The first node below `root` whose term is `term`.
    fn find(&self, root: u32, term: &str) -> Option<u32> {
        let graph = &self.doc.result.graph;
        self.below(root).into_iter().find(|&i| {
            usize::try_from(i)
                .ok()
                .and_then(|u| graph.get(u))
                .is_some_and(|n| n.term.as_deref() == Some(term))
        })
    }

    /// The nodes below `root` whose term starts with `prefix`, by term.
    fn find_prefix(&self, root: u32, prefix: &str) -> BTreeMap<String, u32> {
        let graph = &self.doc.result.graph;
        let mut out = BTreeMap::new();
        for i in self.below(root) {
            if let Some(term) = usize::try_from(i)
                .ok()
                .and_then(|u| graph.get(u))
                .and_then(|n| n.term.as_deref())
                .filter(|t| t.starts_with(prefix))
            {
                out.entry(term.to_owned()).or_insert(i);
            }
        }
        out
    }

    /// A breakdown term's graph reference below a field's node.
    fn resolve(&self, field_node: u32, graph: &str) -> Option<f64> {
        if graph == "result" || graph == "row" {
            return self.node_value(field_node);
        }
        if let Some(id) = graph.strip_prefix("field:") {
            return self.complete(SliceFile::our_name(id)?).map(|(v, _)| v);
        }
        self.node_value(self.find(field_node, graph)?)
    }
}

/// A comparison's raw outcome, before the register.
struct Raw {
    fields: Vec<FieldComparison>,
    breakdowns: Vec<BreakdownComparison>,
    views_differ: Vec<String>,
    /// A breakdown the reader does not know.
    normalization: Option<String>,
}

impl Raw {
    /// The pointers compared and found equal (or within the rule).
    fn equal_pointers(&self) -> BTreeSet<String> {
        let fields = self
            .fields
            .iter()
            .filter(|f| matches!(f.outcome, Outcome::Match | Outcome::WithinTolerance))
            .map(|f| f.pointer.clone());
        let terms = self
            .breakdowns
            .iter()
            .flat_map(|b| &b.terms)
            .filter(|t| t.equal)
            .map(|t| t.pointer.clone());
        fields.chain(terms).collect()
    }
}

/// Parses one oracle breakdown by its shape; `None` when the oracle wrote
/// none (absent or `null`).
enum Parsed {
    Terms(Vec<Printed>),
    Slots(Vec<breakdown::SlotRow>),
}

fn parse(spec: &BreakdownSpec, value: Option<&Value>) -> Result<Option<Parsed>, String> {
    let Some(v) = value.filter(|v| !v.is_null()) else {
        return Ok(None);
    };
    let parsed = match spec.shape {
        Shape::Chain => Parsed::Terms(breakdown::parse_chain(v)?),
        Shape::MinMaxTotal => Parsed::Terms(breakdown::parse_min_max_total(v)?),
        Shape::Slots => Parsed::Slots(breakdown::parse_slots(v)?),
    };
    if let Parsed::Terms(terms) = &parsed
        && let Some(t) = terms
            .iter()
            .find(|t| !spec.terms.iter().any(|s| s.term == t.term))
    {
        return Err(format!(
            "line {}: term {} is not in the slice file",
            t.index, t.term
        ));
    }
    Ok(Some(parsed))
}

/// The text a printed term compares with: as printed, or the sum of two
/// parts printed in the term's format.
fn printed_text(p: &Printed, format: Format) -> String {
    match p.text.as_slice() {
        [one] => one.clone(),
        _ => format.print(p.value()),
    }
}

/// Compares one build's documents, without the register.
fn compare_raw(oracle: &Value, ours: &calc_dto::Document, slice: &SliceFile) -> Raw {
    let ours = Ours { doc: ours };
    let main = oracle.get("main_output");
    let breakdowns = oracle.get("breakdowns");
    let mut raw = Raw {
        fields: Vec::new(),
        breakdowns: Vec::new(),
        views_differ: Vec::new(),
        normalization: None,
    };
    // Every breakdown read first: an unknown line fails the build.
    let mut parsed: BTreeMap<&str, Option<Parsed>> = BTreeMap::new();
    for f in &slice.fields {
        let Some(spec) = &f.breakdown else { continue };
        match parse(spec, breakdowns.and_then(|b| b.get(&spec.pob_key))) {
            Ok(p) => {
                parsed.insert(f.id.as_str(), p);
            }
            Err(e) => {
                raw.normalization
                    .get_or_insert_with(|| format!("breakdown {}: {e}", spec.pob_key));
            }
        }
    }
    // PoB's CALCS view (breakdowns) against its main view (main_output).
    for f in &slice.fields {
        let (Some(spec), Some(Some(Parsed::Terms(terms)))) =
            (&f.breakdown, parsed.get(f.id.as_str()))
        else {
            continue;
        };
        if spec.shape != Shape::Chain {
            continue;
        }
        let (Some(total), Some(t)) = (
            spec.terms.iter().find(|t| t.term == "total"),
            terms.iter().find(|t| t.term == "total"),
        ) else {
            continue;
        };
        let (Some(format), Some(value)) = (
            Format::of_line(&total.line),
            main.and_then(|m| m.get(&f.pob_key)).and_then(Value::as_f64),
        ) else {
            continue;
        };
        let printed = printed_text(t, format);
        if format.print(value) != printed {
            raw.views_differ.push(format!(
                "{}: breakdown total {printed}, main output {}",
                spec.pob_key,
                format.print(value)
            ));
        }
    }
    for f in &slice.fields {
        raw.fields.push(compare_field(f, main, &ours));
        let Some(spec) = &f.breakdown else { continue };
        let not_compared = |reason: String| BreakdownComparison {
            field: f.id.clone(),
            pob_key: spec.pob_key.clone(),
            outcome: BreakdownOutcome::NotCompared,
            reason: Some(reason),
            terms: Vec::new(),
        };
        let Some(name) = SliceFile::our_name(&f.id) else {
            continue;
        };
        let comparison = match (ours.complete(name), parsed.get(f.id.as_str())) {
            (None, _) => not_compared("our field is not complete".to_owned()),
            (_, None) => not_compared("the breakdown did not read".to_owned()),
            (Some(_), Some(_)) if !raw.views_differ.is_empty() => {
                not_compared("PoB's CALCS and main views differ".to_owned())
            }
            (Some((_, node)), Some(p)) => compare_breakdown(f, spec, p.as_ref(), &ours, node),
        };
        raw.breakdowns.push(comparison);
    }
    raw
}

fn compare_field(f: &SliceField, main: Option<&Value>, ours: &Ours<'_>) -> FieldComparison {
    let name = SliceFile::our_name(&f.id).unwrap_or("");
    let oracle = main.and_then(|m| m.get(&f.pob_key)).cloned();
    let mut out = FieldComparison {
        field: f.id.clone(),
        ours: name,
        pointer: format!("/main_output/{}", divergence::pointer_escape(&f.pob_key)),
        outcome: Outcome::NotComputed,
        state: None,
        oracle,
        successor: None,
        reasons: Vec::new(),
        class: None,
        entry: None,
        note: None,
    };
    let Some(field) = ours.field(name) else {
        out.reasons.push("not-in-result".to_owned());
        return out;
    };
    out.state = Some(calc_run::state_name(&field.state));
    out.successor = field.value.as_ref().map(|v| json_number(number(v)));
    let mut reasons: Vec<String> = field
        .reason
        .iter()
        .chain(&field.unsupported)
        .map(calc_run::reason_kind)
        .chain(field.indeterminate.iter().map(calc_run::reason_kind))
        .collect();
    reasons.sort();
    reasons.dedup();
    out.outcome = match field.state {
        calc_dto::StateKind::Unsupported => Outcome::NotComputed,
        calc_dto::StateKind::Partial => Outcome::Partial,
        calc_dto::StateKind::Indeterminate => Outcome::Indeterminate,
        calc_dto::StateKind::Complete => {
            let ours = field.value.as_ref().map(number);
            match (out.oracle.as_ref().and_then(Value::as_f64), ours) {
                (Some(o), Some(s)) if o == s => Outcome::Match,
                (Some(o), Some(s)) if f.comparison.comparison().within(o, s) => {
                    Outcome::WithinTolerance
                }
                _ => Outcome::Difference,
            }
        }
    };
    if !matches!(field.state, calc_dto::StateKind::Complete) {
        out.reasons = reasons;
    }
    out
}

fn compare_breakdown(
    f: &SliceField,
    spec: &BreakdownSpec,
    parsed: Option<&Parsed>,
    ours: &Ours<'_>,
    node: u32,
) -> BreakdownComparison {
    let key = divergence::pointer_escape(&spec.pob_key);
    let mut terms = Vec::new();
    // No breakdown from the oracle: a pass only where the slice file says
    // when PoB writes none (`absent_when`); then the optional terms must
    // be 1 on our side, and the others are not printed.
    if parsed.is_none() && spec.absent_when.is_none() {
        return BreakdownComparison {
            field: f.id.clone(),
            pob_key: spec.pob_key.clone(),
            outcome: BreakdownOutcome::NotCompared,
            reason: Some("the oracle wrote no breakdown".to_owned()),
            terms,
        };
    }
    match (spec.shape, parsed) {
        (Shape::Chain | Shape::MinMaxTotal, printed) => {
            let printed: &[Printed] = match printed {
                Some(Parsed::Terms(t)) => t,
                _ => &[],
            };
            for t in &spec.terms {
                if parsed.is_none() && t.optional.is_none() {
                    continue;
                }
                let format = Format::of_line(&t.line).unwrap_or(Format::Exact);
                let p = printed.iter().find(|p| p.term == t.term);
                let oracle = p.map(|p| printed_text(p, format));
                let successor = ours.resolve(node, &t.graph).map(|v| format.print(v));
                let equal = match (&oracle, &successor) {
                    (Some(a), Some(b)) => a == b,
                    (None, None) => true,
                    (None, Some(b)) => t.optional.is_some() && *b == format.print(1.0),
                    (Some(_), None) => false,
                };
                terms.push(TermComparison {
                    pointer: p.map_or_else(
                        || format!("/breakdowns/{key}/{}", t.term),
                        |p| format!("/breakdowns/{key}/{}", p.index),
                    ),
                    term: t.term.clone(),
                    oracle,
                    successor,
                    equal,
                    class: None,
                    entry: None,
                    note: None,
                });
            }
        }
        (Shape::Slots, rows) => {
            let rows: &[breakdown::SlotRow] = match rows {
                Some(Parsed::Slots(r)) => r,
                _ => &[],
            };
            let ours_rows = ours.find_prefix(node, "step.slot.");
            if let Some(row_spec) = &spec.row {
                let format = |line: &str| Format::of_line(line).unwrap_or(Format::Exact);
                for row in rows {
                    let at = |part: &str| format!("/breakdowns/{key}/slots/{}/{part}", row.index);
                    let our_node = ours_rows.get(&row.node).copied();
                    let part = |graph: &str| our_node.and_then(|n| ours.resolve(n, graph));
                    let mut push = |name: &str,
                                    oracle: Option<String>,
                                    successor: Option<String>,
                                    neutral: Option<String>| {
                        let equal = match (&oracle, &successor) {
                            (Some(a), Some(b)) => a == b,
                            (None, None) => true,
                            (None, Some(b)) => neutral.as_ref() == Some(b),
                            (Some(_), None) => false,
                        };
                        terms.push(TermComparison {
                            pointer: at(name),
                            term: format!("{}.{name}", row.node),
                            oracle,
                            successor,
                            equal,
                            class: None,
                            entry: None,
                            note: None,
                        });
                    };
                    let base = format(&row_spec.base.line);
                    push(
                        "base",
                        Some(base.print(row.base)),
                        part(&row_spec.base.graph).map(|v| base.print(v)),
                        None,
                    );
                    let inc = format(&row_spec.inc.line);
                    push(
                        "inc",
                        row.inc.clone(),
                        part(&row_spec.inc.graph).map(|v| inc.print(v)),
                        Some(inc.print(1.0)),
                    );
                    let more = format(&row_spec.more.line);
                    push(
                        "more",
                        row.more.clone(),
                        part(&row_spec.more.graph).map(|v| more.print(v)),
                        Some(more.print(1.0)),
                    );
                    let total = format(&row_spec.total.line);
                    push(
                        "total",
                        Some(row.total.clone()),
                        our_node
                            .and_then(|n| ours.node_value(n))
                            .map(|v| total.print(v)),
                        None,
                    );
                }
                let total = format(&row_spec.total.line);
                for (name, n) in &ours_rows {
                    if !rows.iter().any(|r| &r.node == name) {
                        terms.push(TermComparison {
                            pointer: format!("/breakdowns/{key}/{name}"),
                            term: name.clone(),
                            oracle: None,
                            successor: ours.node_value(*n).map(|v| total.print(v)),
                            equal: false,
                            class: None,
                            entry: None,
                            note: None,
                        });
                    }
                }
            }
        }
    }
    let differs = terms.iter().any(|t| !t.equal);
    let outcome = match (parsed.is_some(), differs) {
        (_, true) => BreakdownOutcome::Difference,
        (true, false) => BreakdownOutcome::Match,
        (false, false) => BreakdownOutcome::Absent,
    };
    BreakdownComparison {
        field: f.id.clone(),
        pob_key: spec.pob_key.clone(),
        outcome,
        reason: None,
        terms,
    }
}

/// The successor's failure class for where a build stopped.
const fn successor_class(step: Option<Step>) -> FailureClass {
    match step {
        Some(Step::Read) => FailureClass::Import,
        Some(Step::Resolve | Step::Calculate) | None => FailureClass::Calculation,
        Some(Step::Document) => FailureClass::Protocol,
    }
}

/// The register and its record.
struct Registry {
    register: Register,
    record: RegisterRecord,
}

/// Classifies a build's differences, polices `data-version` entries with
/// the overlay run's equal pointers, and sets the classes back on the
/// comparisons. Returns the entries that covered a difference.
fn classify(
    raw: &mut Raw,
    id: &str,
    hashes: &[&str],
    registry: Option<&Registry>,
    overlay_equal: Option<&BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut differences: Vec<(String, Option<Value>, Option<Value>)> = raw
        .fields
        .iter()
        .filter(|f| f.outcome == Outcome::Difference)
        .map(|f| (f.pointer.clone(), f.oracle.clone(), f.successor.clone()))
        .collect();
    differences.extend(
        raw.breakdowns
            .iter()
            .flat_map(|b| &b.terms)
            .filter(|t| !t.equal)
            .map(|t| {
                (
                    t.pointer.clone(),
                    t.oracle.clone().map(Value::String),
                    t.successor.clone().map(Value::String),
                )
            }),
    );
    let register = registry.map(|r| &r.register);
    let (classified, _) = crate::oracle_compare::classify(differences, id, hashes, register);
    let mut used = BTreeSet::new();
    let mut by_pointer: BTreeMap<String, (FailureClass, Option<String>, Option<String>)> =
        BTreeMap::new();
    for d in classified {
        let (mut class, mut entry, mut note) = (d.class, d.entry, d.note);
        let data_version = entry.as_ref().is_some_and(|e| {
            register.is_some_and(|r| {
                r.entries
                    .iter()
                    .any(|x| &x.id == e && x.kind == Kind::DataVersion)
            })
        });
        if data_version && !overlay_equal.is_some_and(|eq| eq.contains(&d.field)) {
            note = Some(format!(
                "{} is data-version: it covers the difference only when the data-overlay run matches the oracle here, and {}",
                entry.as_deref().unwrap_or(""),
                if overlay_equal.is_some() {
                    "that run differs or did not compare it"
                } else {
                    "no --overlay-run was given"
                }
            ));
            class = FailureClass::UnexpectedDifference;
            entry = None;
        }
        if let Some(e) = &entry {
            used.insert(e.clone());
        }
        by_pointer.insert(d.field, (class, entry, note));
    }
    for f in &mut raw.fields {
        if let Some((class, entry, note)) = by_pointer.get(&f.pointer) {
            f.class = Some(*class);
            f.entry.clone_from(entry);
            f.note.clone_from(note);
        }
    }
    for t in raw.breakdowns.iter_mut().flat_map(|b| &mut b.terms) {
        if let Some((class, entry, note)) = by_pointer.get(&t.pointer).filter(|_| !t.equal) {
            t.class = Some(*class);
            t.entry.clone_from(entry);
            t.note.clone_from(note);
        }
    }
    used
}

/// One side of a successor run.
struct Run {
    dir: PathBuf,
    report: calc_run::Report,
    record: SideRecord,
}

fn read_run(dir: &Path) -> Result<Run, String> {
    let (report, sha256) = calc_run::read_report(dir)?;
    Ok(Run {
        dir: dir.to_path_buf(),
        record: SideRecord {
            sha256,
            platform: None,
            tool_commit: report.tool.commit.clone(),
        },
        report,
    })
}

/// The class of a whole report from its builds' classes (as
/// `oracle-compare`): the smallest failure class, else the larger
/// difference class.
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

/// What the comparison reads.
struct Inputs {
    slice: SliceFile,
    slice_record: SliceRecord,
    registry: Option<Registry>,
    oracle: crate::oracle_compare::RunDir,
    successor: Run,
    overlay: Option<Run>,
    mechanic: BTreeMap<String, Vec<String>>,
}

fn read_inputs(request: &Request) -> Result<Inputs, String> {
    let read = |p: &Path| std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()));
    let slice_bytes = read(&request.slice)?;
    let slice =
        SliceFile::read(&slice_bytes).map_err(|e| format!("{}: {e}", request.slice.display()))?;
    let slice_record = SliceRecord {
        sha256: oracle::sha256_hex(&slice_bytes),
        revision: slice.revision,
        status: slice.status.clone(),
    };
    let registry = match &request.register {
        Some(path) => {
            let bytes = read(path)?;
            let register = Register::parse(&bytes)
                .map_err(|p| format!("{}: invalid register: {}", path.display(), p.join("; ")))?;
            let accepted = register
                .entries
                .iter()
                .filter(|e| e.status == Status::Accepted)
                .count();
            Some(Registry {
                record: RegisterRecord {
                    sha256: oracle::sha256_hex(&bytes),
                    entries: register.entries.len(),
                    accepted,
                },
                register,
            })
        }
        None => None,
    };
    let oracle = crate::oracle_compare::read_run(&request.oracle)?;
    let successor = read_run(&request.successor)?;
    if successor.report.overlay.is_some() {
        return Err(format!(
            "{}: a data-overlay run; compare a plain run and pass this one as --overlay-run",
            request.successor.display()
        ));
    }
    let overlay = match &request.overlay_run {
        Some(dir) => {
            let run = read_run(dir)?;
            if run.report.overlay.is_none() {
                return Err(format!(
                    "{}: not a data-overlay run (calc-run --data-overlay)",
                    dir.display()
                ));
            }
            if run.report.bundle != successor.report.bundle {
                return Err(format!(
                    "{}: read another bundle than {}",
                    dir.display(),
                    request.successor.display()
                ));
            }
            Some(run)
        }
        None => None,
    };
    let mut mechanic = BTreeMap::new();
    if let Some(path) = &request.manifest {
        let manifest =
            Manifest::parse(&read(path)?).map_err(|e| format!("{}: {e}", path.display()))?;
        for e in manifest.fixtures {
            if e.tier == Tier::Mechanic {
                if let Some(bad) = e
                    .targets
                    .iter()
                    .find(|t| !slice.fields.iter().any(|f| &&f.id == t))
                {
                    return Err(format!(
                        "{}: fixture {}: target {bad} is not a slice field",
                        path.display(),
                        e.id
                    ));
                }
                mechanic.insert(e.id, e.targets);
            }
        }
    }
    Ok(Inputs {
        slice,
        slice_record,
        registry,
        oracle,
        successor,
        overlay,
        mechanic,
    })
}

/// The overlay run's equal pointers for a build, when it has a document
/// (`None`: the overlay run has none for it).
fn overlay_equal(
    overlay: &Run,
    id: &str,
    oracle: &Value,
    slice: &SliceFile,
) -> Result<Option<BTreeSet<String>>, String> {
    let Some(b) = overlay
        .report
        .builds
        .iter()
        .find(|b| b.id == id && b.status == "ok")
    else {
        return Ok(None);
    };
    let doc = calc_run::read_document(&overlay.dir, b)?;
    Ok(Some(compare_raw(oracle, &doc, slice).equal_pointers()))
}

/// Compares and builds the report (nothing is written).
pub fn compare(request: &Request) -> Result<Report, String> {
    let inputs = read_inputs(request)?;
    let mut builds = Vec::new();
    let mut unpaired = Vec::new();
    let mut used = BTreeSet::new();
    let successor_builds = &inputs.successor.report.builds;
    for o in inputs.oracle.fixtures() {
        let Some(s) = successor_builds.iter().find(|s| s.id == o.id) else {
            unpaired.push((o.id.clone(), "oracle"));
            continue;
        };
        let mut b = BuildComparison {
            id: o.id.clone(),
            oracle_sha256: o.sha256.clone(),
            successor_sha256: Some(s.sha256.clone()),
            input_differs: o.sha256.as_deref() != Some(s.sha256.as_str()),
            status: "no_difference",
            class: None,
            reason: None,
            state: s.state.clone(),
            views_differ: Vec::new(),
            fields: Vec::new(),
            breakdowns: Vec::new(),
            targets: Vec::new(),
        };
        let oracle_has = matches!(o.status.as_str(), "ok" | "partial");
        let successor_has = s.status == "ok";
        if !(oracle_has && successor_has) {
            not_compared(&mut b, o, s, oracle_has, successor_has);
            builds.push(b);
            continue;
        }
        let oracle_doc = inputs.oracle.output(&o.id)?;
        let doc = calc_run::read_document(&inputs.successor.dir, s)?;
        let mut raw = compare_raw(&oracle_doc.value, &doc, &inputs.slice);
        let overlay_eq = match &inputs.overlay {
            Some(run) => overlay_equal(run, &o.id, &oracle_doc.value, &inputs.slice)?,
            None => None,
        };
        let hashes: Vec<&str> = [o.sha256.as_deref(), Some(s.sha256.as_str())]
            .into_iter()
            .flatten()
            .collect();
        used.extend(classify(
            &mut raw,
            &o.id,
            &hashes,
            inputs.registry.as_ref(),
            overlay_eq.as_ref(),
        ));
        let mut classes: Vec<FailureClass> = raw
            .fields
            .iter()
            .filter_map(|f| f.class)
            .chain(
                raw.breakdowns
                    .iter()
                    .flat_map(|b| &b.terms)
                    .filter_map(|t| t.class),
            )
            .collect();
        if let Some(targets) = inputs.mechanic.get(&o.id) {
            for t in targets {
                let met = raw.fields.iter().any(|f| {
                    &f.field == t
                        && (matches!(f.outcome, Outcome::Match | Outcome::WithinTolerance)
                            || f.class == Some(FailureClass::ExpectedDifference))
                });
                if !met {
                    classes.push(FailureClass::UnexpectedDifference);
                }
                b.targets.push(Target {
                    field: t.clone(),
                    met,
                });
            }
        }
        let class = classes.iter().copied().max();
        b.status = match class {
            None => "no_difference",
            Some(FailureClass::ExpectedDifference) => "expected",
            Some(_) => "unexpected",
        };
        b.class = class;
        if let Some(why) = raw.normalization.take() {
            b.status = "failed";
            b.class = Some(FailureClass::Normalization);
            b.reason = Some(why);
        }
        b.views_differ = raw.views_differ;
        b.fields = raw.fields;
        b.breakdowns = raw.breakdowns;
        builds.push(b);
    }
    for s in successor_builds {
        if !inputs.oracle.fixtures().iter().any(|o| o.id == s.id) {
            unpaired.push((s.id.clone(), "successor"));
        }
    }
    let mut report = Report {
        format: FORMAT,
        status: "no_difference",
        class: None,
        oracle: inputs.oracle.record().clone(),
        successor: inputs.successor.record.clone(),
        overlay: inputs.overlay.as_ref().map(|r| r.record.clone()),
        bundle: inputs.successor.report.bundle.clone(),
        slice: inputs.slice_record,
        register: inputs.registry.as_ref().map(|r| r.record.clone()),
        counts: Counts::default(),
        builds,
        unpaired,
        unused_entries: Vec::new(),
    };
    finish(
        &mut report,
        inputs.registry.as_ref().map(|r| &r.register),
        &used,
    );
    Ok(report)
}

fn not_compared(
    b: &mut BuildComparison,
    o: &crate::oracle_compare::RunFixture,
    s: &BuildRecord,
    oracle_has: bool,
    successor_has: bool,
) {
    let successor_failed = (!successor_has).then(|| successor_class(s.step));
    let oracle_class = (!oracle_has).then(|| o.class.unwrap_or(FailureClass::Protocol));
    // The fixture must not import, and neither side imported it.
    let expected_failure = o.status == "failed"
        && o.met == Some(true)
        && o.expect.as_deref() == Some(FailureClass::Import.name())
        && o.class == Some(FailureClass::Import)
        && s.step == Some(Step::Read);
    let describe_s = || {
        format!(
            "{} at {}",
            s.code.as_deref().unwrap_or("?"),
            s.step
                .map_or_else(String::new, |step| calc_run::reason_kind(&step))
        )
    };
    let describe_o = || o.class.map_or(o.status.clone(), |c| c.name().to_owned());
    if expected_failure {
        b.status = "expected_failure";
        b.reason = Some(format!(
            "both sides failed to import, as the fixture manifest expects (oracle {}, successor {})",
            describe_o(),
            describe_s()
        ));
        return;
    }
    b.status = "not_compared";
    b.class = [oracle_class, successor_failed].into_iter().flatten().min();
    b.reason = Some(match (oracle_has, successor_has) {
        (true, _) => format!("no document on the successor side ({})", describe_s()),
        (_, true) => format!("no output on the oracle side ({})", describe_o()),
        _ => format!(
            "no output on either side (oracle {}, successor {})",
            describe_o(),
            describe_s()
        ),
    });
}

fn finish(report: &mut Report, register: Option<&Register>, used: &BTreeSet<String>) {
    let mut counts = Counts {
        builds: report.builds.len(),
        ..Counts::default()
    };

    for b in &report.builds {
        match b.status {
            "no_difference" => counts.no_difference += 1,
            "expected" => counts.expected += 1,
            "unexpected" => counts.unexpected += 1,
            "failed" => counts.failed += 1,
            "expected_failure" => counts.expected_failures += 1,
            _ => counts.not_compared += 1,
        }
        if !b.views_differ.is_empty() {
            counts.views_differ += 1;
        }
        for f in &b.fields {
            *counts
                .fields
                .entry(calc_run::reason_kind(&f.outcome))
                .or_default() += 1;
        }
        for d in &b.breakdowns {
            *counts
                .breakdowns
                .entry(calc_run::reason_kind(&d.outcome))
                .or_default() += 1;
        }
        let classes = b.fields.iter().filter_map(|f| f.class).chain(
            b.breakdowns
                .iter()
                .flat_map(|d| &d.terms)
                .filter_map(|t| t.class),
        );
        for c in classes {
            if c == FailureClass::ExpectedDifference {
                counts.expected_differences += 1;
            } else {
                counts.unexpected_differences += 1;
            }
        }
    }
    report.counts = counts;
    let classes: Vec<FailureClass> = report
        .builds
        .iter()
        .filter_map(|b| b.class)
        .chain((!report.unpaired.is_empty()).then_some(FailureClass::UnexpectedDifference))
        .collect();
    report.class = report_class(&classes);
    report.status = match report.class {
        None => "no_difference",
        Some(FailureClass::ExpectedDifference) => "expected",
        Some(FailureClass::UnexpectedDifference) => "unexpected",
        Some(_) => "failed",
    };
    if let Some(register) = register {
        report.unused_entries = register
            .entries
            .iter()
            .filter(|e| e.status == Status::Accepted && !used.contains(&e.id))
            .filter(|e| {
                report.builds.iter().any(|b| {
                    matches!(b.status, "no_difference" | "expected" | "unexpected")
                        && e.names_fixture(
                            &b.id,
                            &[&b.oracle_sha256, &b.successor_sha256]
                                .into_iter()
                                .filter_map(|h| h.as_deref())
                                .collect::<Vec<_>>(),
                        )
                })
            })
            .map(|e| e.id.clone())
            .collect();
    }
}

impl Report {
    /// A short Markdown summary.
    pub fn summary_markdown(&self) -> String {
        let c = &self.counts;
        let mut text = format!(
            "# calc-parity\n\nStatus: **{}**{}. {} builds: {} without difference, {} expected, {} \
             unexpected, {} failed, {} expected failures, {} not compared; {} with PoB's views \
             differing. Differences: {} expected, {} unexpected.\n\nFields: {}.\n\nBreakdowns: {}.\n",
            self.status,
            self.class
                .map(|c| format!(" ({})", c.name()))
                .unwrap_or_default(),
            c.builds,
            c.no_difference,
            c.expected,
            c.unexpected,
            c.failed,
            c.expected_failures,
            c.not_compared,
            c.views_differ,
            c.expected_differences,
            c.unexpected_differences,
            join_counts(&c.fields),
            join_counts(&c.breakdowns),
        );
        for b in &self.builds {
            let differences: Vec<String> = b
                .fields
                .iter()
                .filter(|f| f.class.is_some())
                .map(|f| {
                    format!(
                        "| `{}` | {} | {} | {} | {} |",
                        f.pointer,
                        cell(f.oracle.as_ref().map(ToString::to_string)),
                        cell(f.successor.as_ref().map(ToString::to_string)),
                        f.class.map_or("", FailureClass::name),
                        cell(f.entry.clone().or_else(|| f.note.clone())),
                    )
                })
                .chain(
                    b.breakdowns
                        .iter()
                        .flat_map(|d| &d.terms)
                        .filter(|t| t.class.is_some())
                        .map(|t| {
                            format!(
                                "| `{}` | {} | {} | {} | {} |",
                                t.pointer,
                                cell(t.oracle.clone()),
                                cell(t.successor.clone()),
                                t.class.map_or("", FailureClass::name),
                                cell(t.entry.clone().or_else(|| t.note.clone())),
                            )
                        }),
                )
                .collect();
            if differences.is_empty() && b.reason.is_none() && b.views_differ.is_empty() {
                continue;
            }
            text.push_str(&format!(
                "\n## {} ({})\n\n",
                b.id.replace(['\r', '\n'], " "),
                b.status
            ));
            if let Some(reason) = &b.reason {
                text.push_str(&format!("{reason}\n\n"));
            }
            for v in &b.views_differ {
                text.push_str(&format!("PoB's views differ: {v}\n\n"));
            }
            if !differences.is_empty() {
                text.push_str(
                    "| Pointer | Oracle | Successor | Class | Entry |\n|---|---|---|---|---|\n",
                );
                for d in differences {
                    text.push_str(&d);
                    text.push('\n');
                }
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

fn join_counts(counts: &BTreeMap<String, usize>) -> String {
    if counts.is_empty() {
        return "none".to_owned();
    }
    counts
        .iter()
        .map(|(k, v)| format!("{v} {k}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn cell(text: Option<String>) -> String {
    let t = text.unwrap_or_else(|| "(absent)".to_owned());
    let t = if t.chars().count() > 60 {
        format!("{}...", t.chars().take(57).collect::<String>())
    } else {
        t
    };
    t.replace('|', "\\|")
}

/// Compares and writes `report.json` and `summary.md` to `out`, which
/// must not exist.
pub fn run(request: &Request) -> Result<Report, String> {
    if request.out.exists() {
        return Err(format!("{}: already exists", request.out.display()));
    }
    let report = compare(request)?;
    std::fs::create_dir_all(&request.out).map_err(|e| format!("{}: {e}", request.out.display()))?;
    for (name, text) in [
        ("report.json", calc_run::pretty(&report)?),
        ("summary.md", report.summary_markdown()),
    ] {
        let path = request.out.join(name);
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(report)
}
