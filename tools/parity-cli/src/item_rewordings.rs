//! `item-rewordings`: the reviewed rewording table for item lines against
//! PoB's own parse (ADR-015 Amendment 3; `docs/phase-6/item-rewordings.md`).
//!
//! Two steps, with PoB's parse between them (maintainer-run with the
//! oracle, as `custom-lines` is):
//!
//! 1. `lines` writes, one per line, each pair's old and current wording at
//!    the same sample numbers ([`SAMPLES`]), then any further lines given
//!    (an item line's `{...}` prefixes removed, each line once), for
//!    `tools/oracle/data/pob_parse_lines.lua`.
//! 2. `check` reads that parse and puts each pair and line in one
//!    category:
//!    - `accepted`: an accepted pair PoB applies alike in both wordings,
//!      and whose current wording resolves through a stat description;
//!    - `accepted-differs` (**a failure**): an accepted pair PoB parses
//!      differently, or leaves text over in either wording;
//!    - `accepted-ambiguous`: an accepted pair whose current wording
//!      several descriptions print (listed: an item's mods choose at
//!      import, and a line they cannot choose for stays a reason);
//!    - `accepted-unread` (**a failure**): an accepted pair whose current
//!      wording no description of the bundle reads;
//!    - `rejected-pob-agrees`, `rejected-pob-differs`: a rejected pair
//!      (listed; the review's reason stands either way);
//!    - `candidate-pob-agrees`, `candidate-pob-differs`: a pair not
//!      reviewed yet (listed, for the review);
//!    - for a further line: `reworded` (the table reads it), `note` (the
//!      item note rule takes it and PoB applies nothing),
//!      `note-pob-applies` (**a failure**: the note would drop an effect
//!      PoB applies), or `unread` (a reason; listed with PoB's mods).
//!
//! Exit 0 without a failure, 1 with one.

use std::collections::BTreeMap;

use loom_domain::{LineResolution, Resolved, UnresolvedReason};
use loom_parser::import::Resolver;
use loom_parser::import::poe1::{custom, reword};
use serde::Serialize;

use crate::custom_lines::{PobLine, PobMod};

/// The numbers a pair's slots are filled with, in turn.
pub const SAMPLES: &[&str] = &["7", "13", "19", "29"];

/// The failure categories.
pub const FAILURES: &[&str] = &["accepted-differs", "accepted-unread", "note-pob-applies"];

/// A template filled with [`SAMPLES`].
#[must_use]
pub fn sample(template: &str) -> String {
    let slots = template.matches('#').count();
    let numbers: Vec<&str> = (0..slots).map(|i| SAMPLES[i % SAMPLES.len()]).collect();
    reword::fill(template, &numbers)
}

/// An item line as the resolver reads it: its `{...}` prefixes removed,
/// trimmed.
#[must_use]
pub fn bare(line: &str) -> &str {
    let mut s = line.trim();
    while let Some(rest) = s.strip_prefix('{') {
        match rest.find('}') {
            Some(end) => s = rest[end + 1..].trim_start(),
            None => break,
        }
    }
    s.trim()
}

/// The lines for PoB's parse: every pair's two wordings at the sample
/// numbers, then `extra` (bare, each once, skipping empty lines and those
/// already listed).
#[must_use]
pub fn lines(table: &reword::Table, extra: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |l: String| {
        if !l.is_empty() && !out.contains(&l) {
            out.push(l);
        }
    };
    for p in table
        .accepted
        .iter()
        .chain(&table.rejected)
        .chain(&table.candidates)
    {
        push(sample(&p.old));
        push(sample(&p.current));
    }
    for l in extra {
        push(bare(l).to_owned());
    }
    out
}

/// One pair's entry.
#[derive(Debug, Serialize)]
pub struct PairEntry {
    /// The old template.
    pub old: String,
    /// The current template.
    pub current: String,
    /// Its category (module docs).
    pub category: &'static str,
    /// PoB's mods for the old wording at the sample numbers, if it
    /// applies it.
    pub pob_old: Option<Vec<PobMod>>,
    /// The same for the current wording.
    pub pob_current: Option<Vec<PobMod>>,
}

/// One further line's entry.
#[derive(Debug, Serialize)]
pub struct LineEntry {
    /// The line.
    pub line: String,
    /// Its category (module docs).
    pub category: &'static str,
    /// The current wording, for a reworded line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reworded: Option<String>,
    /// PoB's mods when it applies the line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pob: Option<Vec<PobMod>>,
}

/// The report.
#[derive(Debug, Serialize)]
pub struct Report {
    /// Pairs and lines by category.
    pub counts: BTreeMap<&'static str, usize>,
    /// Every pair, in table order.
    pub pairs: Vec<PairEntry>,
    /// Every further line, in input order.
    pub lines: Vec<LineEntry>,
}

impl Report {
    /// Whether nothing failed.
    #[must_use]
    pub fn passed(&self) -> bool {
        FAILURES
            .iter()
            .all(|c| self.counts.get(c).copied().unwrap_or(0) == 0)
    }
}

/// Whether the current wording resolves through a stat description.
fn reads(resolver: &Resolver<'_, '_>, text: &str) -> Reading {
    match resolver.resolve_custom_text(text) {
        LineResolution::Resolved {
            result: Resolved::Stats(_),
            ..
        } => Reading::One,
        LineResolution::Unresolved(UnresolvedReason::Ambiguous(_)) => Reading::Several,
        _ => Reading::None,
    }
}

/// How the bundle's descriptions read a current wording alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reading {
    /// One description.
    One,
    /// Several: an item's mods choose at import, as for any line.
    Several,
    /// None.
    None,
}

/// Compares the table and the further lines with PoB's parse.
#[must_use]
pub fn check(
    resolver: &Resolver<'_, '_>,
    table: &reword::Table,
    pob: &[PobLine],
    is_keystone: impl Fn(&str) -> bool,
) -> Report {
    let parsed: BTreeMap<&str, &PobLine> = pob.iter().map(|l| (l.line.as_str(), l)).collect();
    let applied = |line: &str| parsed.get(line).and_then(|l| l.applied());
    let mut counts = BTreeMap::new();
    let mut pairs = Vec::new();
    let groups: [(&[reword::Pair], &str); 3] = [
        (&table.accepted, "accepted"),
        (&table.rejected, "rejected"),
        (&table.candidates, "candidate"),
    ];
    let mut listed = Vec::new();
    for (list, group) in groups {
        for p in list {
            let (old, current) = (sample(&p.old), sample(&p.current));
            let (pob_old, pob_current) = (applied(&old), applied(&current));
            let agrees = pob_old.is_some() && pob_old == pob_current;
            let category = match (group, agrees) {
                ("accepted", true) => match reads(resolver, &current) {
                    Reading::One => "accepted",
                    Reading::Several => "accepted-ambiguous",
                    Reading::None => "accepted-unread",
                },
                ("accepted", false) => "accepted-differs",
                ("rejected", true) => "rejected-pob-agrees",
                ("rejected", false) => "rejected-pob-differs",
                (_, true) => "candidate-pob-agrees",
                (_, false) => "candidate-pob-differs",
            };
            *counts.entry(category).or_insert(0) += 1;
            listed.push(old);
            listed.push(current);
            pairs.push(PairEntry {
                old: p.old.clone(),
                current: p.current.clone(),
                category,
                pob_old,
                pob_current,
            });
        }
    }
    let mut lines = Vec::new();
    for l in pob.iter().filter(|l| !listed.contains(&l.line)) {
        let text = l.line.as_str();
        let pob = l.applied().filter(|m| !m.is_empty());
        let current = reword::reword(text);
        let category = if current.is_some() {
            "reworded"
        } else if custom::is_label(text, &is_keystone) {
            if pob.is_some() {
                "note-pob-applies"
            } else {
                "note"
            }
        } else {
            "unread"
        };
        *counts.entry(category).or_insert(0) += 1;
        lines.push(LineEntry {
            line: text.to_owned(),
            category,
            reworded: current,
            pob,
        });
    }
    Report {
        counts,
        pairs,
        lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_should_fill_each_slot_in_turn() {
        assert_eq!(sample("+# to # and #"), "+7 to 13 and 19");
        assert_eq!(sample("no number"), "no number");
    }

    #[test]
    fn bare_should_remove_the_prefixes() {
        assert_eq!(
            bare("{tags:elemental,resistance}+1 to Maximum Endurance Charge"),
            "+1 to Maximum Endurance Charge"
        );
        assert_eq!(bare("{variant:3}Corrupted "), "Corrupted");
        assert_eq!(bare("{unclosed text"), "{unclosed text");
    }

    #[test]
    fn lines_should_list_each_line_once() {
        let table = reword::Table::parse(
            r##"{"format":1,"accepted":[{"old":"+# a","current":"+# b","why":"w"}]}"##,
        )
        .expect("table");
        let extra = vec![
            "{variant:1}+7 a".to_owned(),
            "Note".to_owned(),
            "Note".to_owned(),
        ];
        assert_eq!(lines(&table, &extra), vec!["+7 a", "+7 b", "Note"]);
    }
}
