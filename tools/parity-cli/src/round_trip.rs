//! `round-trip <extract> --out <dir>` (Spike B, PR B2b): render every
//! English variant of every stat-description file with values inside its
//! conditions, resolve the rendered lines in the same file's scope, and
//! require the same description and values back.
//!
//! - **Scope:** each `ok` file is a root; its own descriptions (depth 0)
//!   are rendered, and its whole include chain is indexed, so a line that
//!   a description of an included file also writes shows as ambiguity.
//! - **Variants:** every variant of each hand, except `gem_quality` and
//!   `table_only` ones (the item index leaves them out; counted).
//! - **Samples:** per variant, three exact samples (low, middle, high end
//!   of each stat's condition; an open end is 100 past the other end, or
//!   `1`, `50`, `100` with no bound) and one ranged sample (low to high).
//!   A stat with a lookup placeholder takes the first, middle, and last
//!   key the lookup has that meets the condition, never a range.
//!   Identical samples are tried once. Values stay in the `i32` range.
//! - **Render:** `render_description` with the extract's [`TableLookup`];
//!   the renderer picks the variant (the first whose conditions hold), so a
//!   sample can be written by an earlier variant of the same description.
//! - **Check:** the lines must resolve as one [`Resolution`] whose
//!   candidate is the rendered description and hand, with every stat's
//!   value read back (an exact value or range containing the sample; a
//!   hidden stat's condition holding for it; a looked-up key among the
//!   keys) by the first verifying variant or one of its alternatives.
//!
//! Every sample ends in exactly one outcome (field docs of [`Summary`]).

use std::collections::{BTreeMap, BTreeSet};

use loom_data::stat_desc::{
    Condition, DescRef, Description, Hand, Handler, Lookup, LookupHandler, RAW_MAX, RAW_MIN, Scope,
    StatValue, VariantFilter, render_description,
};
use loom_parser::resolve::{Candidate, Index, IndexStats, Outcome, RawValue, ResolvedStat};
use serde::Serialize;

use crate::extract::{Extract, Inputs};
use crate::lookup::{LookupReport, TableLookup};

/// Examples kept per outcome in the summary (all are in the report).
const EXAMPLES: usize = 5;

/// Counts of one run. Field order is the JSON order.
#[derive(Debug, Default, Serialize)]
pub struct Summary {
    /// Recorded inputs.
    pub inputs: Inputs,
    /// What each lookup handler's table gave.
    pub lookup: LookupReport,
    /// Scopes built (one per `ok` description file).
    pub scopes: usize,
    /// [`IndexStats`] of the item scope (`stat_descriptions`), by field.
    pub item_index: BTreeMap<&'static str, usize>,
    /// Descriptions (per hand) of the item scope whose stats the scope
    /// finds in another description: the renderer never uses them, so
    /// lines they would match depend on the include-precedence rule.
    pub item_scope_shadowed_descriptions: usize,
    /// Variants per hand, by what happened: `tried`, or skipped as
    /// `gem_quality` or `table_only`.
    pub variants: BTreeMap<&'static str, usize>,
    /// Samples tried.
    pub samples: usize,
    /// Samples by outcome, all files:
    /// - `ok`: read back;
    /// - `not-rendered/<reason>`: the renderer wrote no line;
    /// - `shadowed-description`: the description is shadowed in its own
    ///   file's scope, so the scope never renders it (not resolved);
    /// - `rendered-double-sign`: `no-template`, and a line has `--` or `+-`
    ///   before a digit (a literal sign in the template before a negative
    ///   value);
    /// - `split`: the lines resolved as more than one resolution;
    /// - `no-template`, `only-shadowed`, `no-variant-fits/<first reason>`:
    ///   unresolved;
    /// - `ambiguous-handed`: the two hands of the description;
    /// - `ambiguous-template`: the description and others;
    /// - `wrong-description`: resolved or ambiguous without it;
    /// - `value-mismatch`: the description, other values.
    pub outcomes: BTreeMap<String, usize>,
    /// The same for the item scope's root file only.
    pub item_scope_outcomes: BTreeMap<String, usize>,
    /// Samples whose resolution lists shadowed matches (their reading
    /// depends on include precedence).
    pub samples_with_shadowed_matches: usize,
    /// The first examples per non-`ok` outcome.
    pub examples: BTreeMap<String, Vec<Failure>>,
}

/// One sample that did not read back.
#[derive(Debug, Clone, Serialize)]
pub struct Failure {
    /// Outcome name.
    pub outcome: String,
    /// Manifest path of the root file.
    pub file: String,
    /// 1-based line of the description in the game file.
    pub description_line: usize,
    /// 1-based line of the variant.
    pub variant_line: usize,
    /// `main` or `off`.
    pub hand: &'static str,
    /// Sample values in stat order (`[min, max]`).
    pub values: Vec<[i64; 2]>,
    /// Rendered lines.
    pub lines: Vec<String>,
    /// What came back, briefly.
    pub detail: String,
}

/// Outcomes that mean the resolver read a rendered line wrongly; the
/// subcommand fails if any occurs.
pub const WRONG_READINGS: [&str; 3] = ["value-mismatch", "wrong-description", "split"];

/// Every failure.
#[derive(Debug, Default, Serialize)]
pub struct Report {
    /// Failures in file, description, variant, sample order.
    pub failures: Vec<Failure>,
}

/// Whether a manifest path is the item scope's root file.
fn is_item_scope(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.eq_ignore_ascii_case("stat_descriptions.txt")
        || name.eq_ignore_ascii_case("stat_descriptions.csd")
}

/// Runs the round trip over a loaded extract.
///
/// # Errors
/// A message when a scope does not build.
pub fn run(
    extract: &Extract,
    lookup: &TableLookup,
    lookup_report: LookupReport,
) -> Result<(Summary, Report), String> {
    let mut s = Summary {
        inputs: extract.inputs.clone(),
        lookup: lookup_report,
        ..Summary::default()
    };
    let mut report = Report::default();
    for path in &extract.files {
        let scope = extract
            .set
            .scope(path)
            .map_err(|e| format!("{path}: {} ({e})", e.code()))?;
        s.scopes += 1;
        let index = Index::build_with(&scope, lookup);
        let item = is_item_scope(path);
        if item {
            s.item_index = index_fields(index.stats());
            s.item_scope_shadowed_descriptions = shadowed_descriptions(&scope);
        }
        let ctx = Ctx {
            path,
            scope: &scope,
            index: &index,
            lookup,
        };
        for (main, description) in scope.descriptions().filter(|(at, _)| at.depth == 0) {
            for hand in [Hand::Main, Hand::Off] {
                if description.stats_of(hand).is_none() {
                    continue;
                }
                let at = DescRef { hand, ..main };
                for variant in 0..description.variants.len() {
                    for (outcome, failure, shadowed) in
                        ctx.variant(at, description, variant, &mut s)
                    {
                        if shadowed {
                            s.samples_with_shadowed_matches += 1;
                        }
                        *s.outcomes.entry(outcome.clone()).or_default() += 1;
                        if item {
                            *s.item_scope_outcomes.entry(outcome.clone()).or_default() += 1;
                        }
                        if let Some(f) = failure {
                            let examples = s.examples.entry(outcome).or_default();
                            if examples.len() < EXAMPLES {
                                examples.push(f.clone());
                            }
                            report.failures.push(f);
                        }
                    }
                }
            }
        }
    }
    Ok((s, report))
}

/// [`IndexStats`] by field name.
fn index_fields(i: IndexStats) -> BTreeMap<&'static str, usize> {
    BTreeMap::from([
        ("indexed", i.indexed),
        ("multi_line", i.multi_line),
        ("lookup", i.lookup),
        ("entries", i.entries),
        ("keys", i.keys),
        ("max_lines", i.max_lines),
        ("gem_quality", i.gem_quality),
        ("table_only", i.table_only),
        ("bad_template", i.bad_template),
        ("empty_text", i.empty_text),
        ("lookup_unavailable", i.lookup_unavailable),
        ("lookup_multi", i.lookup_multi),
        ("lookup_blank_names", i.lookup_blank_names),
    ])
}

/// Descriptions (per hand) of a scope with a stat the scope finds
/// elsewhere.
fn shadowed_descriptions(scope: &Scope<'_>) -> usize {
    let mut n = 0;
    for (main, d) in scope.descriptions() {
        for hand in [Hand::Main, Hand::Off] {
            let Some(ids) = d.stats_of(hand) else {
                continue;
            };
            if is_shadowed(scope, DescRef { hand, ..main }, ids) {
                n += 1;
            }
        }
    }
    n
}

/// Same rule as the index: depth and index differ for some stat.
fn is_shadowed(scope: &Scope<'_>, at: DescRef, ids: &[String]) -> bool {
    ids.iter().any(|id| {
        scope
            .find(id)
            .is_none_or(|f| (f.depth, f.index) != (at.depth, at.index))
    })
}

struct Ctx<'c, 'a> {
    path: &'c str,
    scope: &'c Scope<'a>,
    index: &'c Index<'a>,
    lookup: &'c TableLookup,
}

impl Ctx<'_, '_> {
    /// Every sample of one variant: (outcome, failure, has shadowed
    /// matches).
    fn variant(
        &self,
        at: DescRef,
        description: &Description,
        variant: usize,
        s: &mut Summary,
    ) -> Vec<(String, Option<Failure>, bool)> {
        let Some(v) = description.variants.get(variant) else {
            return Vec::new();
        };
        if v.gem_quality || v.table_only {
            let flag = if v.gem_quality {
                "gem_quality"
            } else {
                "table_only"
            };
            *s.variants.entry(flag).or_default() += 1;
            return Vec::new();
        }
        *s.variants.entry("tried").or_default() += 1;
        let ids = description.stats_of(at.hand).unwrap_or_default();
        let shadowed_desc = is_shadowed(self.scope, at, ids);
        let lookups: BTreeMap<usize, LookupHandler> = v
            .handlers
            .iter()
            .filter_map(|h| match h {
                Handler::Lookup { handler, stat } => Some((*stat, *handler)),
                _ => None,
            })
            .collect();
        let mut out = Vec::new();
        for values in samples(&v.conditions, ids.len(), &lookups, self.lookup) {
            s.samples += 1;
            let rendered = render_description(
                description,
                at.hand,
                &values,
                self.lookup,
                VariantFilter::default(),
            );
            let fail = |outcome: String, lines: Vec<String>, detail: String| Failure {
                outcome,
                file: self.path.to_owned(),
                description_line: description.line,
                variant_line: v.line,
                hand: match at.hand {
                    Hand::Main => "main",
                    Hand::Off => "off",
                },
                values: values.iter().map(|x| [x.min, x.max]).collect(),
                lines,
                detail,
            };
            let lines = match rendered {
                Ok((_, lines)) => lines,
                Err(reason) => {
                    let outcome = format!("not-rendered/{}", reason.name());
                    let f = fail(outcome.clone(), Vec::new(), format!("{reason:?}"));
                    out.push((outcome, Some(f), false));
                    continue;
                }
            };
            if shadowed_desc {
                let outcome = "shadowed-description".to_owned();
                let f = fail(outcome.clone(), lines, String::new());
                out.push((outcome, Some(f), false));
                continue;
            }
            let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
            let resolutions = self.index.resolve_lines(&refs);
            let has_shadowed = resolutions.iter().any(|r| !r.shadowed.is_empty());
            let (mut outcome, detail) = match resolutions.as_slice() {
                [one] => classify(&one.outcome, at, &values),
                more => ("split".to_owned(), format!("{} resolutions", more.len())),
            };
            if outcome == "no-template" && lines.iter().any(|l| double_sign(l)) {
                "rendered-double-sign".clone_into(&mut outcome);
            }
            let failure = (outcome != "ok").then(|| fail(outcome.clone(), lines, detail));
            out.push((outcome, failure, has_shadowed));
        }
        out
    }
}

/// Whether a line has two signs before a digit (`--5`, `+-5`). On a line
/// without a template key it marks a template with a literal sign before a
/// plain placeholder, rendered with a negative value; no game text is
/// known to show it. (A negative range, `(-10--5)`, has it too, and
/// resolves.)
fn double_sign(line: &str) -> bool {
    let bytes = line.as_bytes();
    bytes
        .windows(3)
        .any(|w| matches!(w, [a, b'-', d] if (*a == b'-' || *a == b'+') && d.is_ascii_digit()))
}

/// The outcome name of one resolution, and a short detail.
fn classify(outcome: &Outcome, at: DescRef, values: &[StatValue]) -> (String, String) {
    match outcome {
        Outcome::Resolved(c) if c.at == at => {
            if reads_back(c, values) {
                ("ok".to_owned(), String::new())
            } else {
                ("value-mismatch".to_owned(), format!("{:?}", c.stats))
            }
        }
        Outcome::Resolved(c) => ("wrong-description".to_owned(), format!("{:?}", c.at)),
        Outcome::Ambiguous(cs) => {
            let ats: Vec<DescRef> = cs.iter().map(|c| c.at).collect();
            let detail = format!("{ats:?}");
            if !ats.contains(&at) {
                ("wrong-description".to_owned(), detail)
            } else if ats
                .iter()
                .all(|x| (x.depth, x.index) == (at.depth, at.index))
            {
                ("ambiguous-handed".to_owned(), detail)
            } else {
                ("ambiguous-template".to_owned(), detail)
            }
        }
        Outcome::Unresolved(u) => {
            let name = match u {
                loom_parser::resolve::Unresolved::NoVariantFits(rejected) => format!(
                    "no-variant-fits/{}",
                    rejected.first().map_or("none", |r| r.reason.name())
                ),
                other => other.name().to_owned(),
            };
            (name, format!("{u:?}"))
        }
    }
}

/// Whether the candidate's reading, or one of its alternatives, holds
/// every sample value.
fn reads_back(c: &Candidate, values: &[StatValue]) -> bool {
    let holds = |stats: &[ResolvedStat]| {
        stats.len() == values.len()
            && stats
                .iter()
                .zip(values)
                .all(|(s, v)| contains(&s.value, *v))
    };
    holds(&c.stats) || c.alternatives.iter().any(|a| holds(&a.stats))
}

fn contains(read: &RawValue, v: StatValue) -> bool {
    match read {
        RawValue::Exact(x) | RawValue::Ranged(x) => x.min <= v.min && v.max <= x.max,
        RawValue::Hidden { condition, .. } => condition.holds(v.min),
        RawValue::Lookup(keys) => v.min == v.max && keys.contains(&v.min),
    }
}

/// Low, middle, and high values a condition allows (fewer if they
/// coincide), inside the `i32` range.
fn ends(condition: Condition) -> [i64; 3] {
    let (lo, hi) = match condition {
        Condition::Range {
            min: Some(a),
            max: Some(b),
        } => (a, b),
        Condition::Range {
            min: Some(a),
            max: None,
        } => (a, a.saturating_add(100)),
        Condition::Range {
            min: None,
            max: Some(b),
        } => (b.saturating_sub(100), b),
        Condition::Range {
            min: None,
            max: None,
        } => (1, 100),
        Condition::Not(v) => (v.saturating_add(1), v.saturating_add(100)),
    };
    let clamp = |x: i64| x.clamp(RAW_MIN, RAW_MAX);
    let (lo, hi) = (clamp(lo), clamp(hi));
    [lo, lo + (hi - lo) / 2, hi]
}

/// The samples of a variant with `count` stats.
fn samples(
    conditions: &[Condition],
    count: usize,
    lookups: &BTreeMap<usize, LookupHandler>,
    lookup: &dyn Lookup,
) -> BTreeSet<Vec<StatValue>> {
    let any = Condition::Range {
        min: None,
        max: None,
    };
    let per_stat: Vec<([i64; 3], bool)> = (0..count)
        .map(|i| {
            let condition = conditions.get(i).copied().unwrap_or(any);
            let keys: Option<Vec<i64>> = lookups.get(&i).map(|h| {
                lookup
                    .entries(*h)
                    .into_iter()
                    .map(|(k, _)| k)
                    .filter(|k| condition.holds(*k))
                    .collect()
            });
            match keys.as_deref() {
                Some([first, .., last]) => {
                    let all = keys.as_deref().unwrap_or_default();
                    let mid = all.get(all.len() / 2).copied().unwrap_or(*first);
                    ([*first, mid, *last], true)
                }
                Some([one]) => ([*one; 3], true),
                // No key: a number, so the renderer reports the lookup.
                Some([]) | None => (ends(condition), keys.is_some()),
            }
        })
        .collect();
    let mut out = BTreeSet::new();
    for k in 0..3 {
        out.insert(
            per_stat
                .iter()
                .map(|(e, _)| StatValue::exact(e.get(k).copied().unwrap_or(0)))
                .collect(),
        );
    }
    out.insert(
        per_stat
            .iter()
            .map(|([lo, _, hi], looked_up)| {
                if *looked_up {
                    StatValue::exact(*lo)
                } else {
                    StatValue { min: *lo, max: *hi }
                }
            })
            .collect(),
    );
    out
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]
mod tests {
    use super::*;
    use crate::extract::load;

    const MANIFEST: &str = r#"{"format": 3, "game": "poe1", "steam_build_id": "1",
        "stat_descriptions": {"files": {
          "metadata/statdescriptions/stat_descriptions.txt": {"kind": "descriptions",
            "status": "ok", "output": "a.json", "descriptions": 3, "english_variants": 4}},
        "handlers": {}}}"#;

    /// Increased/reduced pair, a two-line variant, and a description whose
    /// text a second description also writes.
    const FILE: &str = r#"{"descriptions": [
        {"handed":false,"line":1,"name":null,"off_hand_stats":null,"stats":["s"],"variants":[
          {"conditions":[{"kind":"range","max":null,"min":1}],"flags":[],"handlers":[],"line":2,
           "placeholders":[{"format":"","index":0}],"text":"{0}% increased X"},
          {"conditions":[{"kind":"range","max":-1,"min":null}],"flags":[],
           "handlers":[{"name":"negate","stat":1}],"line":3,
           "placeholders":[{"format":"","index":0}],"text":"{0}% reduced X"}]},
        {"handed":false,"line":4,"name":null,"off_hand_stats":null,"stats":["m"],"variants":[
          {"conditions":[{"kind":"range","max":null,"min":null}],"flags":[],"handlers":[],"line":5,
           "placeholders":[{"format":"+d","index":0}],"text":"{0:+d} first\\nSecond"}]},
        {"handed":false,"line":6,"name":null,"off_hand_stats":null,"stats":["t"],"variants":[
          {"conditions":[{"kind":"range","max":null,"min":null}],"flags":[],"handlers":[],"line":7,
           "placeholders":[{"format":"","index":0}],"text":"{0}% increased X"}]}],
        "file_flags": [], "includes": [], "languages": {}, "no_description": [],
        "source": "metadata/statdescriptions/stat_descriptions.txt"}"#;

    fn read(path: &str) -> Result<Vec<u8>, String> {
        match path {
            "manifest.json" => Ok(MANIFEST.as_bytes().to_vec()),
            "a.json" => Ok(FILE.as_bytes().to_vec()),
            _ => Err(format!("{path}: not found")),
        }
    }

    #[test]
    fn run_should_count_every_sample_under_one_outcome() {
        let extract = load(&read).unwrap();
        let (lookup, report) = TableLookup::load(&read, &extract).unwrap();
        let (s, r) = run(&extract, &lookup, report).unwrap();
        assert_eq!(s.scopes, 1);
        assert_eq!(s.variants.get("tried"), Some(&4));
        assert_eq!(s.outcomes.values().sum::<usize>(), s.samples);
        assert_eq!(s.item_scope_outcomes, s.outcomes);
        // `s` and `t` write the same increased text: ambiguous; the reduced
        // text and the two-line text read back.
        assert_eq!(s.outcomes.get("ambiguous-template"), Some(&8));
        assert_eq!(s.outcomes.get("ok"), Some(&8));
        assert_eq!(r.failures.len(), 8);
        assert_eq!(s.item_index.get("multi_line"), Some(&1));
        assert_eq!(s.item_index.get("indexed"), Some(&4));
    }

    #[test]
    fn double_sign_should_find_two_signs_before_a_digit_only() {
        assert!(double_sign("--5 to X") && double_sign("a +-1"));
        assert!(!double_sign("-5 to X") && !double_sign("Off--hand") && !double_sign("x --"));
    }

    #[test]
    fn ends_should_stay_inside_each_condition_and_the_i32_range() {
        let conditions = [
            Condition::Range {
                min: Some(3),
                max: Some(9),
            },
            Condition::Range {
                min: Some(i64::from(i32::MAX)),
                max: None,
            },
            Condition::Range {
                min: None,
                max: Some(-1),
            },
            Condition::Range {
                min: None,
                max: None,
            },
            Condition::Not(0),
        ];
        for c in conditions {
            for v in ends(c) {
                assert!(c.holds(v) && (RAW_MIN..=RAW_MAX).contains(&v), "{c:?} {v}");
            }
        }
        assert_eq!(ends(conditions[0]), [3, 6, 9]);
    }

    #[test]
    fn classify_should_tell_handed_ambiguity_from_template_ambiguity() {
        let at = DescRef {
            depth: 0,
            index: 2,
            hand: Hand::Main,
        };
        let cand = |at: DescRef| Candidate {
            at,
            variant: 0,
            stats: Vec::new(),
            alternatives: Vec::new(),
        };
        let off = DescRef {
            hand: Hand::Off,
            ..at
        };
        let other = DescRef { index: 3, ..at };
        let name = |o: Outcome| classify(&o, at, &[]).0;
        assert_eq!(
            name(Outcome::Ambiguous(vec![cand(at), cand(off)])),
            "ambiguous-handed"
        );
        assert_eq!(
            name(Outcome::Ambiguous(vec![cand(at), cand(other)])),
            "ambiguous-template"
        );
        assert_eq!(
            name(Outcome::Ambiguous(vec![cand(off), cand(other)])),
            "wrong-description"
        );
        assert_eq!(name(Outcome::Resolved(cand(at))), "ok");
        assert_eq!(name(Outcome::Resolved(cand(other))), "wrong-description");
    }

    #[test]
    fn samples_should_take_lookup_keys_that_meet_the_condition() {
        struct Keys;
        impl Lookup for Keys {
            fn name(&self, _: LookupHandler, v: i64) -> Option<String> {
                Some(v.to_string())
            }
            fn entries(&self, _: LookupHandler) -> Vec<(i64, String)> {
                (1..=5).map(|k| (k, k.to_string())).collect()
            }
        }
        let lookups = BTreeMap::from([(0, LookupHandler::PassiveHash)]);
        let at_least_two = Condition::Range {
            min: Some(2),
            max: None,
        };
        let s = samples(&[at_least_two], 1, &lookups, &Keys);
        let firsts: Vec<StatValue> = s.iter().map(|v| v[0]).collect();
        assert_eq!(
            firsts,
            vec![
                StatValue::exact(2),
                StatValue::exact(4),
                StatValue::exact(5)
            ]
        );
    }
}
