//! The divergence register (ADR-006): the versioned list of accepted
//! differences between the oracle and the successor, and its validator
//! (`divergence-check`). The format is documented in `docs/parity/README.md`
//! and `docs/parity/divergence-register.schema.json`; this module enforces
//! it (no JSON-schema crate, as for the oracle output).
//!
//! `oracle-compare` reads the register to tell expected differences
//! (covered by an accepted entry) from unexpected ones.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::oracle_compare::COMPARED;

/// The register format this reader knows.
pub const FORMAT: u32 = 1;

/// `docs/parity/divergence-register.json`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Register {
    /// Register format; 1.
    pub format: u32,
    /// The entries, in the order they were added.
    pub entries: Vec<Entry>,
}

/// One accepted (or retired) difference.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// `DIV-` and four digits; unique, never reused.
    pub id: String,
    /// The game mechanic the difference is about.
    pub mechanic: String,
    /// The fixture: its id (file name) or `sha256:<hex>` of its bytes.
    pub fixture: String,
    /// The field: a JSON pointer into the output document, under one of
    /// the compared roots ([`COMPARED`]).
    pub field: String,
    /// How the two values are compared.
    pub comparison: Comparison,
    /// The oracle's (left) value; `null` when the field is absent there.
    pub oracle_value: Value,
    /// The successor's (right) value; `null` when the field is absent
    /// there.
    pub successor_value: Value,
    /// Why the values differ.
    pub kind: Kind,
    /// Links or citations; for `game-truth`, the game evidence (ADR-001).
    pub evidence: Vec<String>,
    /// The test that pins the difference.
    pub test: String,
    /// Date the entry was added, `YYYY-MM-DD`.
    pub added: String,
    /// Whether the entry covers differences.
    pub status: Status,
}

/// How an entry compares the two values of its field.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum Comparison {
    /// The values must be exactly the registered ones. (A struct variant:
    /// serde ignores unknown fields next to the tag of a unit variant.)
    Exact {},
    /// Numbers whose absolute difference is at most `tolerance`.
    Abs {
        /// Largest accepted `|left - right|`.
        tolerance: f64,
    },
    /// Numbers whose relative difference is at most `tolerance`.
    Rel {
        /// Largest accepted `|left - right| / max(|left|, |right|)`.
        tolerance: f64,
    },
}

/// Why a difference exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// The game behaves as the successor computes; PoB differs.
    GameTruth,
    /// A bug in PoB.
    PobBug,
    /// Game data differs between the oracle and the successor.
    DataVersion,
    /// The harness, not PoB or the successor, causes the difference.
    Harness,
}

/// Whether an entry is in force.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Covers matching differences.
    Accepted,
    /// Kept for history; covers nothing.
    Retired,
}

/// What `divergence-check` prints.
#[derive(Debug, Serialize)]
pub struct CheckReport {
    /// `ok` or `invalid`.
    pub status: &'static str,
    /// Register format, when it parsed.
    pub format: Option<u32>,
    /// Entries, when it parsed.
    pub entries: Option<usize>,
    /// Accepted entries, when it parsed.
    pub accepted: Option<usize>,
    /// Every problem found.
    pub problems: Vec<String>,
}

impl Register {
    /// Parses and validates a register. `Err` holds every problem found.
    pub fn parse(bytes: &[u8]) -> Result<Self, Vec<String>> {
        let register: Self = serde_json::from_slice(bytes).map_err(|e| vec![e.to_string()])?;
        let problems = register.problems();
        if problems.is_empty() {
            Ok(register)
        } else {
            Err(problems)
        }
    }

    /// Every rule the schema cannot express on its own, plus the schema's.
    fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.format != FORMAT {
            problems.push(format!(
                "format: this reader knows {FORMAT}, found {}",
                self.format
            ));
        }
        let mut ids = BTreeSet::new();
        let mut covered = BTreeSet::new();
        for (i, entry) in self.entries.iter().enumerate() {
            let at = format!("entries[{}] ({})", i + 1, entry.id);
            for problem in entry.problems() {
                problems.push(format!("{at}: {problem}"));
            }
            if !ids.insert(entry.id.as_str()) {
                problems.push(format!("{at}: duplicate id"));
            }
            if entry.status == Status::Accepted
                && !covered.insert((entry.fixture.as_str(), entry.field.as_str()))
            {
                problems.push(format!(
                    "{at}: a second accepted entry for fixture {} and field {}",
                    entry.fixture, entry.field
                ));
            }
        }
        problems
    }

    /// Checks `bytes` and reports, for `divergence-check`.
    pub fn check(bytes: &[u8]) -> CheckReport {
        match Self::parse(bytes) {
            Ok(register) => CheckReport {
                status: "ok",
                format: Some(register.format),
                entries: Some(register.entries.len()),
                accepted: Some(
                    register
                        .entries
                        .iter()
                        .filter(|e| e.status == Status::Accepted)
                        .count(),
                ),
                problems: Vec::new(),
            },
            Err(problems) => CheckReport {
                status: "invalid",
                format: None,
                entries: None,
                accepted: None,
                problems,
            },
        }
    }
}

impl Entry {
    fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if !is_entry_id(&self.id) {
            problems.push("id: expected DIV- and four digits".to_owned());
        }
        for (name, text) in [("mechanic", &self.mechanic), ("test", &self.test)] {
            if text.trim().is_empty() {
                problems.push(format!("{name}: empty"));
            }
        }
        if !is_fixture_reference(&self.fixture) {
            problems.push(
                "fixture: expected a fixture id (a file name) or sha256:<64 hex digits>".to_owned(),
            );
        }
        match pointer_segments(&self.field) {
            Some(segments)
                if segments
                    .first()
                    .is_some_and(|r| COMPARED.contains(&r.as_str())) => {}
            _ => problems.push(format!(
                "field: expected a JSON pointer under one of /{}",
                COMPARED.join(", /")
            )),
        }
        match self.comparison {
            Comparison::Exact {} => {
                if same_value(&self.oracle_value, &self.successor_value) {
                    problems.push("oracle_value and successor_value: equal".to_owned());
                }
            }
            Comparison::Abs { tolerance } | Comparison::Rel { tolerance } => {
                if !(tolerance.is_finite() && tolerance > 0.0) {
                    problems.push("comparison.tolerance: not a positive number".to_owned());
                }
                if !(self.oracle_value.is_number() && self.successor_value.is_number()) {
                    problems.push(
                        "oracle_value and successor_value: a tolerance needs two numbers"
                            .to_owned(),
                    );
                }
            }
        }
        if self.evidence.is_empty() || self.evidence.iter().any(|e| e.trim().is_empty()) {
            problems.push("evidence: empty".to_owned());
        }
        if !is_date(&self.added) {
            problems.push("added: expected YYYY-MM-DD".to_owned());
        }
        problems
    }

    /// Whether this entry covers a difference of its fixture and field
    /// between `left` and `right` (`None`: absent on that side). `Err`
    /// says why an entry for the field does not cover these values.
    pub fn covers(&self, left: Option<&Value>, right: Option<&Value>) -> Result<(), String> {
        let null = Value::Null;
        let (left, right) = (left.unwrap_or(&null), right.unwrap_or(&null));
        match self.comparison {
            Comparison::Exact {} => {
                if same_value(left, &self.oracle_value) && same_value(right, &self.successor_value)
                {
                    Ok(())
                } else {
                    Err(format!(
                        "{} registers {} -> {}",
                        self.id, self.oracle_value, self.successor_value
                    ))
                }
            }
            Comparison::Abs { tolerance } | Comparison::Rel { tolerance } => {
                let (Some(l), Some(r)) = (left.as_f64(), right.as_f64()) else {
                    return Err(format!("{} compares numbers only", self.id));
                };
                let difference = (l - r).abs();
                let limit = match self.comparison {
                    Comparison::Rel { .. } => tolerance * l.abs().max(r.abs()),
                    _ => tolerance,
                };
                if difference <= limit {
                    Ok(())
                } else {
                    Err(format!(
                        "{}: difference {difference} is over the limit {limit}",
                        self.id
                    ))
                }
            }
        }
    }

    /// Whether this entry names `fixture` (by id or by `sha256:` hash).
    pub fn names_fixture(&self, id: &str, sha256: Option<&str>) -> bool {
        self.fixture == id
            || sha256.is_some_and(|h| self.fixture.strip_prefix("sha256:") == Some(h))
    }
}

/// Equal JSON values, with numbers compared as doubles (so `1` equals
/// `1.0`; `-0.0` equals `0`).
pub fn same_value(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| same_value(a, b))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| same_value(v, w)))
        }
        _ => a == b,
    }
}

/// The unescaped segments of a JSON pointer (RFC 6901); `None` if `text`
/// is not one or is the whole document (`""`).
pub fn pointer_segments(text: &str) -> Option<Vec<String>> {
    let rest = text.strip_prefix('/')?;
    rest.split('/')
        .map(|segment| {
            let mut out = String::new();
            let mut chars = segment.chars();
            while let Some(c) = chars.next() {
                match c {
                    '~' => match chars.next() {
                        Some('0') => out.push('~'),
                        Some('1') => out.push('/'),
                        _ => return None,
                    },
                    c => out.push(c),
                }
            }
            Some(out)
        })
        .collect()
}

/// `segment` escaped for a JSON pointer.
pub fn pointer_escape(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

fn is_entry_id(text: &str) -> bool {
    text.strip_prefix("DIV-")
        .is_some_and(|n| n.len() == 4 && n.bytes().all(|b| b.is_ascii_digit()))
}

/// A file name (no path separator, not `.` or `..`) or `sha256:<hex>`.
fn is_fixture_reference(text: &str) -> bool {
    if let Some(hash) = text.strip_prefix("sha256:") {
        return hash.len() == 64 && hash.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    }
    !text.trim().is_empty()
        && text != "."
        && text != ".."
        && !text.contains(['/', '\\'])
        && !text.chars().any(char::is_control)
}

fn is_date(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    let number = |s: &str, len: usize| {
        (s.len() == len && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse::<u32>().ok())?
    };
    match parts.as_slice() {
        [y, m, d] => {
            number(y, 4).is_some()
                && number(m, 2).is_some_and(|m| (1..=12).contains(&m))
                && number(d, 2).is_some_and(|d| (1..=31).contains(&d))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const COMMITTED: &[u8] = include_bytes!("../../../docs/parity/divergence-register.json");
    const SCHEMA: &str = include_str!("../../../docs/parity/divergence-register.schema.json");
    const DISCREPANCY: &[u8] = include_bytes!("../../oracle/fixtures/discrepancy-register.json");

    fn entry() -> Value {
        json!({
            "id": "DIV-0001",
            "mechanic": "movement speed",
            "fixture": "a.xml",
            "field": "/main_output/MovementSpeedMod",
            "comparison": {"type": "exact"},
            "oracle_value": 1,
            "successor_value": 1.1,
            "kind": "harness",
            "evidence": ["test register"],
            "test": "tools/parity-cli oracle_compare::tests",
            "added": "2026-11-10",
            "status": "accepted"
        })
    }

    fn register(entries: Vec<Value>) -> Vec<u8> {
        json!({"format": 1, "entries": entries})
            .to_string()
            .into_bytes()
    }

    fn problems_of(entry: Value) -> Vec<String> {
        Register::parse(&register(vec![entry])).unwrap_err()
    }

    #[test]
    fn committed_register_should_be_valid_and_empty() {
        let register = Register::parse(COMMITTED).unwrap();
        assert!(register.entries.is_empty());
        let report = Register::check(COMMITTED);
        assert_eq!(report.status, "ok");
        assert_eq!(report.entries, Some(0));
    }

    #[test]
    fn discrepancy_register_should_be_valid() {
        let register = Register::parse(DISCREPANCY).unwrap();
        assert_eq!(register.entries.len(), 2);
    }

    #[test]
    fn schema_should_list_the_fields_this_reader_knows() {
        let schema: Value = serde_json::from_str(SCHEMA).unwrap();
        let keys = |v: &Value| -> Vec<String> {
            let mut k: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
            k.sort();
            k
        };
        assert_eq!(keys(&schema["properties"]), ["entries", "format"]);
        // Every field of a valid entry, which the reader requires.
        let expected = keys(&entry());
        let defined = &schema["$defs"]["entry"];
        assert_eq!(keys(&defined["properties"]), expected);
        let mut required: Vec<String> = defined["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect();
        required.sort();
        assert_eq!(required, expected);
        assert_eq!(schema["additionalProperties"], json!(false));
        assert_eq!(defined["additionalProperties"], json!(false));
    }

    #[test]
    fn a_valid_entry_should_parse() {
        let register = Register::parse(&register(vec![entry()])).unwrap();
        assert_eq!(register.entries[0].comparison, Comparison::Exact {});
    }

    #[test]
    fn parse_should_reject_unknown_fields_and_values() {
        for (key, value) in [
            ("note", json!("x")),
            ("kind", json!("guess")),
            ("status", json!("open")),
            ("comparison", json!({"type": "close"})),
            ("comparison", json!({"type": "exact", "tolerance": 1})),
        ] {
            let mut e = entry();
            e[key] = value;
            let problems = problems_of(e);
            assert_eq!(problems.len(), 1, "{key}: {problems:?}");
        }
    }

    #[test]
    fn parse_should_report_every_rule_broken() {
        let cases = [
            ("id", json!("DIV-1"), "id:"),
            ("mechanic", json!(" "), "mechanic:"),
            ("test", json!(""), "test:"),
            ("fixture", json!("builds/a.xml"), "fixture:"),
            ("fixture", json!("sha256:abc"), "fixture:"),
            ("field", json!("main_output/Life"), "field:"),
            ("field", json!("/modules/1"), "field:"),
            ("field", json!("/main_output/a~2b"), "field:"),
            ("successor_value", json!(1.0), "equal"),
            ("evidence", json!([]), "evidence:"),
            ("evidence", json!([""]), "evidence:"),
            ("added", json!("2026-13-01"), "added:"),
            ("added", json!("10.11.2026"), "added:"),
        ];
        for (key, value, expected) in cases {
            let mut e = entry();
            e[key] = value;
            let problems = problems_of(e);
            assert!(
                problems.len() == 1 && problems[0].contains(expected),
                "{key}: {problems:?}"
            );
        }
    }

    #[test]
    fn parse_should_check_tolerances() {
        for (comparison, oracle, expected) in [
            (
                json!({"type": "abs", "tolerance": 0}),
                json!(1),
                "tolerance",
            ),
            (
                json!({"type": "rel", "tolerance": -0.1}),
                json!(1),
                "tolerance",
            ),
            (
                json!({"type": "abs", "tolerance": 0.5}),
                json!("1"),
                "two numbers",
            ),
        ] {
            let mut e = entry();
            e["comparison"] = comparison;
            e["oracle_value"] = oracle;
            let problems = problems_of(e);
            assert!(
                problems.len() == 1 && problems[0].contains(expected),
                "{problems:?}"
            );
        }
    }

    #[test]
    fn parse_should_reject_duplicates_and_other_formats() {
        let mut second = entry();
        second["id"] = "DIV-0002".into();
        let problems = Register::parse(&register(vec![entry(), second.clone()])).unwrap_err();
        assert!(
            problems[0].contains("second accepted entry"),
            "{problems:?}"
        );
        // A retired entry may repeat the field.
        second["status"] = "retired".into();
        Register::parse(&register(vec![entry(), second])).unwrap();
        let problems = Register::parse(&register(vec![entry(), entry()])).unwrap_err();
        assert!(
            problems.iter().any(|p| p.contains("duplicate id")),
            "{problems:?}"
        );
        let other = json!({"format": 2, "entries": []}).to_string();
        let problems = Register::parse(other.as_bytes()).unwrap_err();
        assert!(problems[0].starts_with("format"), "{problems:?}");
        let report = Register::check(b"{");
        assert_eq!(report.status, "invalid");
        assert_eq!(report.problems.len(), 1);
    }

    #[test]
    fn covers_should_apply_the_comparison() {
        let exact: Entry = serde_json::from_value(entry()).unwrap();
        assert!(exact.covers(Some(&json!(1.0)), Some(&json!(1.1))).is_ok());
        assert!(exact.covers(Some(&json!(1)), Some(&json!(1.2))).is_err());
        assert!(exact.covers(None, Some(&json!(1.1))).is_err());
        let mut absent = entry();
        absent["oracle_value"] = Value::Null;
        let absent: Entry = serde_json::from_value(absent).unwrap();
        assert!(absent.covers(None, Some(&json!(1.1))).is_ok());
        let mut abs = entry();
        abs["comparison"] = json!({"type": "abs", "tolerance": 0.25});
        let abs: Entry = serde_json::from_value(abs).unwrap();
        assert!(abs.covers(Some(&json!(1)), Some(&json!(1.25))).is_ok());
        assert!(abs.covers(Some(&json!(1)), Some(&json!(1.3))).is_err());
        assert!(abs.covers(Some(&json!(1)), Some(&json!("1"))).is_err());
        let mut rel = entry();
        rel["comparison"] = json!({"type": "rel", "tolerance": 0.1});
        let rel: Entry = serde_json::from_value(rel).unwrap();
        assert!(rel.covers(Some(&json!(100)), Some(&json!(110))).is_ok());
        assert!(rel.covers(Some(&json!(100)), Some(&json!(112))).is_err());
    }

    #[test]
    fn names_fixture_should_match_id_or_hash() {
        let mut e: Entry = serde_json::from_value(entry()).unwrap();
        assert!(e.names_fixture("a.xml", None));
        assert!(!e.names_fixture("b.xml", Some("ab")));
        e.fixture = format!("sha256:{}", "a".repeat(64));
        assert!(e.names_fixture("b.xml", Some(&"a".repeat(64))));
        assert!(!e.names_fixture("b.xml", None));
    }

    #[test]
    fn pointers_should_round_trip_escapes() {
        assert_eq!(
            pointer_segments("/main_output/a~1b~0c"),
            Some(vec!["main_output".to_owned(), "a/b~c".to_owned()])
        );
        assert_eq!(pointer_escape("a/b~c"), "a~1b~0c");
        assert_eq!(pointer_segments(""), None);
        assert_eq!(pointer_segments("/a~"), None);
    }

    #[test]
    fn same_value_should_compare_numbers_as_doubles() {
        assert!(same_value(&json!(1), &json!(1.0)));
        assert!(same_value(&json!({"a": [0.0]}), &json!({"a": [-0.0]})));
        assert!(!same_value(&json!({"a": 1}), &json!({"a": 1, "b": 2})));
        assert!(!same_value(&json!("1"), &json!(1)));
    }
}
