//! `resolve-corpus --bundle` cross-check (P2-6b): every item is also
//! resolved with `loom_parser::import::Resolver` (the core's
//! `resolve_build`), and each modifier line's domain resolution is
//! compared with the harness's outcome before the categories (which are
//! tool-only evidence). Written to `resolve-build-check.json`.
//!
//! A line is compared by a key: the outcome, plus the stats and values of
//! a stat resolution (as the report writes them), the `kind` of a typed
//! one, the kept candidates of an ambiguous one, and the tolerance. A
//! continuation takes the key of the line it continues, as the harness
//! gives every line of a multi-line reading the same record.
//!
//! Scrubbing (ADR-007): a difference names the build hash, the item and
//! line numbers, and the two keys (outcome names, stat IDs, values); no
//! line text.

use std::collections::BTreeMap;

use loom_domain::{
    ItemBody, LineKind, LineResolution, Resolved, StatLine, StatValue, UnresolvedReason,
};
use serde::Serialize;

use super::LineRecord;

/// The cross-check's result.
#[derive(Debug, Default, Serialize)]
pub struct BuildCheck {
    /// Modifier lines compared.
    pub lines: usize,
    /// Lines whose keys are equal.
    pub equal: usize,
    /// Domain keys by count (the outcome only).
    pub outcomes: BTreeMap<String, usize>,
    /// Every line whose keys differ.
    pub differences: Vec<Difference>,
}

/// One line whose keys differ.
#[derive(Debug, Serialize)]
pub struct Difference {
    /// Build hash.
    pub build: String,
    /// Item number in the build.
    pub item: usize,
    /// Line number in the item.
    pub line: usize,
    /// The harness's key.
    pub harness: String,
    /// The domain's key.
    pub domain: String,
}

impl BuildCheck {
    /// Compares the harness records of one item (before categorizing)
    /// with its domain resolution.
    pub fn compare(&mut self, build: &str, item: usize, body: &ItemBody, records: &[LineRecord]) {
        for r in records {
            let Some(domain) = domain_key(body, r.line) else {
                continue; // Not a modifier line (flags, influences, ...).
            };
            self.lines += 1;
            let outcome = domain.split('|').next().unwrap_or_default().to_owned();
            *self.outcomes.entry(outcome).or_default() += 1;
            let harness = harness_key(r);
            if harness == domain {
                self.equal += 1;
            } else {
                self.differences.push(Difference {
                    build: build.to_owned(),
                    item,
                    line: r.line,
                    harness,
                    domain,
                });
            }
        }
    }
}

/// The harness record's key.
fn harness_key(r: &LineRecord) -> String {
    let adapted = r.adapted.unwrap_or("");
    match r.outcome.as_str() {
        "resolved" => match r.kind {
            Some(kind) => format!("resolved|{kind}|{adapted}"),
            None => format!("resolved|{}|{adapted}", r.detail.join(" ")),
        },
        "ambiguous" => {
            let kept: Vec<&str> = r
                .detail
                .iter()
                .map(|c| c.strip_suffix(" (off hand)").unwrap_or(c))
                .collect();
            format!("ambiguous|{}", kept.join(" "))
        }
        o if o.starts_with("unknown-base/") => "no-base".to_owned(),
        o if o.starts_with("excluded/") => "excluded".to_owned(),
        "internal/not-covered" => "pending".to_owned(),
        o => o.to_owned(),
    }
}

/// Line `n`'s domain key; `None` when it is not a modifier line.
fn domain_key(body: &ItemBody, n: usize) -> Option<String> {
    let resolution = match &body.lines.get(n)?.kind {
        LineKind::Modifier(m) => &m.resolution,
        _ => return None,
    };
    let resolution = match resolution {
        LineResolution::Continues { line } => match &body.lines.get(*line)?.kind {
            LineKind::Modifier(m) => &m.resolution,
            _ => return Some("broken-continuation".to_owned()),
        },
        r => r,
    };
    Some(match resolution {
        LineResolution::Resolved { result, adapted } => {
            let adapted = adapted.map_or("", |a| a.name());
            match result {
                Resolved::Stats(stats) => {
                    let stats: Vec<String> = stats.iter().map(stat_text).collect();
                    format!("resolved|{}|{adapted}", stats.join(" "))
                }
                Resolved::Typed(t) => format!("resolved|{}|{adapted}", t.kind()),
            }
        }
        LineResolution::Unresolved(UnresolvedReason::Ambiguous(cs)) => {
            let kept: Vec<String> = cs
                .iter()
                .map(|c| {
                    c.iter()
                        .map(|s| s.as_str().to_owned())
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .collect();
            format!("ambiguous|{}", kept.join(" "))
        }
        LineResolution::Unresolved(UnresolvedReason::NoBase(_)) => "no-base".to_owned(),
        LineResolution::Unresolved(r) => format!("unresolved/{}", r.name()),
        LineResolution::Excluded(_) => "excluded".to_owned(),
        LineResolution::Pending => "pending".to_owned(),
        LineResolution::Continues { .. } => "continues-a-continuation".to_owned(),
    })
}

/// `stat=value` as the report writes a resolved stat (`value_text`).
fn stat_text(s: &StatLine) -> String {
    let span = |min: i64, max: i64| {
        if min == max {
            min.to_string()
        } else {
            format!("{min}..{max}")
        }
    };
    let value = match &s.value {
        StatValue::Exact { min, max } => span(*min, *max),
        StatValue::Ranged { min, max } => format!("({})", span(*min, *max)),
        StatValue::Hidden { assumed } => format!(
            "hidden:{}",
            assumed.map_or("?".to_owned(), |a| a.to_string())
        ),
        StatValue::Lookup(keys) => {
            let keys: Vec<String> = keys.iter().map(i64::to_string).collect();
            format!("keys:{}", keys.join("|"))
        }
    };
    format!("{}={value}", s.stat.as_str())
}
