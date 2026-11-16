//! `custom-lines`: our reading of custom modifier lines against PoB's
//! parse of the same lines (ADR-015 Amendment 2; Phase 3 close-out;
//! `docs/phase-3/custom-lines.md`).
//!
//! The input is `tools/oracle/data/pob_parse_lines.lua`'s output: each line
//! as PoB v2.67.2's config tab reads it (`StripEscapes`, trimmed), with
//! PoB's `modLib.parseMod` result. PoB applies a line only when the whole
//! of it parses (`Classes/ConfigTab.lua` 1115 to 1116: `mods and not
//! extra`); `extra` is any text left over, which Lua treats as true even
//! when empty. Each line is read by [`Resolver::resolve_custom_text`] and
//! put in one category:
//!
//! - `note`: a custom note, and PoB ignores the line (agrees);
//! - `note-pob-applies`: a custom note PoB applies (**a failure**: the
//!   note would drop an effect);
//! - `pattern`: a curated pattern whose effect equals PoB's mods (agrees);
//! - `pattern-differs`: a curated pattern whose effect does not
//!   (**a failure**), or a pattern PoB ignores (**a failure**);
//! - `stats`: resolved through a stat description (checked by the
//!   calculation fixtures, not here), PoB applies it;
//! - `stats-pob-ignores`: resolved through a stat description, PoB ignores
//!   it (a difference the calc runs carry; listed);
//! - `unresolved`: a reason in every calculation (listed with PoB's mods).
//!
//! Exit 0 without a failure, 1 with one.

use std::collections::BTreeMap;

use loom_domain::{LineResolution, Resolved, TypedResolution, UnresolvedReason};
use loom_parser::import::Resolver;
use serde::{Deserialize, Serialize};

/// One line of PoB's parse output.
#[derive(Debug, Deserialize)]
pub struct PobLine {
    /// The line as PoB parses it.
    pub line: String,
    /// What was left over, or `false`.
    pub extra: serde_json::Value,
    /// The mods, or `false` when PoB's parser returned none.
    pub mods: serde_json::Value,
}

/// One PoB mod, as the probe writes it.
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct PobMod {
    /// The mod's name.
    pub name: String,
    /// BASE, INC, MORE, FLAG, ...
    #[serde(rename = "type")]
    pub kind: String,
    /// The value.
    pub value: serde_json::Value,
    /// `ModFlag` bits.
    pub flags: i64,
    /// `KeywordFlag` bits.
    #[serde(rename = "keywordFlags")]
    pub keyword_flags: i64,
    /// Tags (an empty list may be written as `{}`).
    pub tags: serde_json::Value,
}

impl PobLine {
    /// PoB's mods when it applies the line, else `None`.
    fn applied(&self) -> Option<Vec<PobMod>> {
        let extra = !matches!(self.extra, serde_json::Value::Bool(false));
        if extra {
            return None;
        }
        serde_json::from_value(self.mods.clone()).ok()
    }
}

/// The PoB mod each curated pattern stands for: `(pattern, mod name, type)`;
/// the value is the pattern's value.
pub const PATTERN_MODS: &[(&str, &str, &str)] = &[
    ("base-critical-strike-chance", "CritChance", "BASE"),
    ("more-block-chance", "BlockChance", "MORE"),
    ("more-damage", "Damage", "MORE"),
    ("more-spell-block-chance", "SpellBlockChance", "MORE"),
];

/// One line's entry in the report.
#[derive(Debug, Serialize)]
pub struct Entry {
    /// The line.
    pub line: String,
    /// Its category (module docs).
    pub category: &'static str,
    /// Our reading: `note`, `pattern <name> <value>`, `stats <ids>`, or the
    /// unresolved reason with its candidates.
    pub ours: String,
    /// PoB's mods when it applies the line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pob: Option<Vec<PobMod>>,
}

/// The report.
#[derive(Debug, Serialize)]
pub struct Report {
    /// Lines by category.
    pub counts: BTreeMap<&'static str, usize>,
    /// Every line, in input order.
    pub lines: Vec<Entry>,
}

impl Report {
    /// Whether no line failed.
    #[must_use]
    pub fn passed(&self) -> bool {
        FAILURES
            .iter()
            .all(|c| self.counts.get(c).copied().unwrap_or(0) == 0)
    }
}

/// The failure categories.
pub const FAILURES: &[&str] = &["note-pob-applies", "pattern-differs"];

/// Our reading of a line, as text.
fn describe(r: &LineResolution) -> String {
    match r {
        LineResolution::Resolved {
            result: Resolved::Typed(TypedResolution::CustomNote),
            ..
        } => "note".to_owned(),
        LineResolution::Resolved {
            result: Resolved::Typed(TypedResolution::CustomPattern { pattern, value }),
            ..
        } => format!("pattern {pattern} {value}"),
        LineResolution::Resolved {
            result: Resolved::Stats(s),
            ..
        } => format!(
            "stats {}",
            s.iter()
                .map(|l| l.stat.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        ),
        LineResolution::Resolved {
            result: Resolved::Typed(t),
            ..
        } => format!("typed {}", t.kind()),
        LineResolution::Unresolved(UnresolvedReason::Ambiguous(cs)) => format!(
            "ambiguous {}",
            cs.iter()
                .map(|c| c
                    .iter()
                    .map(loom_domain::StatId::as_str)
                    .collect::<Vec<_>>()
                    .join("+"))
                .collect::<Vec<_>>()
                .join(" | ")
        ),
        LineResolution::Unresolved(u) => format!("unresolved {}", u.name()),
        LineResolution::Pending => "pending".to_owned(),
        LineResolution::Excluded(e) => format!("excluded {}", e.name()),
        LineResolution::Continues { .. } => "continues".to_owned(),
    }
}

/// Whether a pattern's reading equals PoB's mods: one mod, of the
/// pattern's name and type, with the pattern's value.
fn pattern_agrees(pattern: &str, value: &str, pob: &[PobMod]) -> bool {
    let Some(&(_, name, kind)) = PATTERN_MODS.iter().find(|(p, ..)| *p == pattern) else {
        return false;
    };
    let Ok(ours) = value.parse::<f64>() else {
        return false;
    };
    match pob {
        [m] => {
            m.name == name
                && m.kind == kind
                && m.flags == 0
                && m.keyword_flags == 0
                && m.value.as_f64() == Some(ours)
                && match &m.tags {
                    serde_json::Value::Array(a) => a.is_empty(),
                    serde_json::Value::Object(o) => o.is_empty(),
                    _ => false,
                }
        }
        _ => false,
    }
}

/// Reads every line with `resolver` and compares it with PoB's.
#[must_use]
pub fn compare(resolver: &Resolver<'_, '_>, pob: &[PobLine]) -> Report {
    let mut counts = BTreeMap::new();
    let mut lines = Vec::new();
    for p in pob {
        let ours = resolver.resolve_custom_text(&p.line);
        let applied = p.applied();
        let category = match (&ours, &applied) {
            (
                LineResolution::Resolved {
                    result: Resolved::Typed(TypedResolution::CustomNote),
                    ..
                },
                None,
            ) => "note",
            (
                LineResolution::Resolved {
                    result: Resolved::Typed(TypedResolution::CustomNote),
                    ..
                },
                Some(_),
            ) => "note-pob-applies",
            (
                LineResolution::Resolved {
                    result: Resolved::Typed(TypedResolution::CustomPattern { pattern, value }),
                    ..
                },
                Some(mods),
            ) if pattern_agrees(pattern, value, mods) => "pattern",
            (
                LineResolution::Resolved {
                    result: Resolved::Typed(TypedResolution::CustomPattern { .. }),
                    ..
                },
                _,
            ) => "pattern-differs",
            (LineResolution::Resolved { .. }, Some(_)) => "stats",
            (LineResolution::Resolved { .. }, None) => "stats-pob-ignores",
            _ => "unresolved",
        };
        *counts.entry(category).or_insert(0) += 1;
        lines.push(Entry {
            line: p.line.clone(),
            category,
            ours: describe(&ours),
            pob: applied,
        });
    }
    Report { counts, lines }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str, extra: serde_json::Value, mods: serde_json::Value) -> PobLine {
        PobLine {
            line: text.to_owned(),
            extra,
            mods,
        }
    }

    #[test]
    fn pob_should_apply_a_line_only_without_extra_text() {
        let m = serde_json::json!([{ "name": "Damage", "type": "MORE", "value": 20,
            "flags": 0, "keywordFlags": 0, "tags": {} }]);
        assert!(
            line("x", serde_json::json!(false), m.clone())
                .applied()
                .is_some()
        );
        assert!(line("x", serde_json::json!(""), m).applied().is_none());
        assert!(
            line("x", serde_json::json!(false), serde_json::json!(false))
                .applied()
                .is_none()
        );
    }

    #[test]
    fn a_pattern_should_agree_only_with_its_one_mod() {
        let m = |name: &str, kind: &str, value: f64| PobMod {
            name: name.to_owned(),
            kind: kind.to_owned(),
            value: serde_json::json!(value),
            flags: 0,
            keyword_flags: 0,
            tags: serde_json::json!({}),
        };
        assert!(pattern_agrees(
            "more-damage",
            "20",
            &[m("Damage", "MORE", 20.0)]
        ));
        assert!(pattern_agrees(
            "more-damage",
            "-15",
            &[m("Damage", "MORE", -15.0)]
        ));
        assert!(!pattern_agrees(
            "more-damage",
            "20",
            &[m("Damage", "INC", 20.0)]
        ));
        assert!(!pattern_agrees(
            "more-damage",
            "20",
            &[m("Damage", "MORE", 21.0)]
        ));
        assert!(!pattern_agrees("more-damage", "20", &[]));
        assert!(!pattern_agrees(
            "unknown",
            "20",
            &[m("Damage", "MORE", 20.0)]
        ));
    }

    /// The calculation's patterns and these are the same names.
    #[test]
    fn the_patterns_should_be_the_parser_and_the_calcs() {
        let parser: Vec<_> = loom_parser::import::poe1::custom::PATTERNS
            .iter()
            .map(|p| p.name)
            .collect();
        let here: Vec<_> = PATTERN_MODS.iter().map(|(p, ..)| *p).collect();
        let calc: Vec<_> = loom_calc::poe1::mapping::CUSTOM_PATTERNS
            .iter()
            .map(|(p, ..)| *p)
            .collect();
        assert_eq!(parser, here);
        assert_eq!(calc, here);
    }
}
