//! `docs/phase-3/slice-fields.json`: the slice fields, each with its PoB
//! key, comparison rule, and breakdown (Phase 3 plan, decision 4), and
//! the mapping from PoB's field ids to our field names (decision 2: PoB's
//! names appear only in this tool).

use std::collections::BTreeSet;

use serde::Deserialize;

use super::breakdown::Format;
use crate::divergence::Comparison;

/// The slice-file format this reader knows.
pub const FORMAT: u32 = 1;

/// The slice-file revision this reader knows: revision 2 added `graph`
/// to every breakdown term and `node` to slot rows (P3-5).
pub const REVISION: u32 = 2;

/// PoB's field id to our field name (`loom_calc::Field`'s `Display`),
/// for every field of `loom_calc::poe1::SLICE`.
pub const OUR_NAMES: [(&str, &str); 24] = [
    ("Str", "strength"),
    ("Dex", "dexterity"),
    ("Int", "intelligence"),
    ("Life", "life"),
    ("Mana", "mana"),
    ("EnergyShield", "energy-shield"),
    ("Armour", "armour"),
    ("Evasion", "evasion"),
    ("FireResist", "resistance.fire"),
    ("ColdResist", "resistance.cold"),
    ("LightningResist", "resistance.lightning"),
    ("ChaosResist", "resistance.chaos"),
    ("BlockChance", "block-chance"),
    ("SpellBlockChance", "poe1.spell-block-chance"),
    ("SpellSuppressionChance", "poe1.spell-suppression-chance"),
    ("LifeUnreserved", "life-unreserved"),
    ("ManaUnreserved", "mana-unreserved"),
    ("AverageDamage", "average-damage"),
    ("Speed", "speed"),
    ("HitSpeed", "hit-speed"),
    ("HitChance", "hit-chance"),
    ("CritChance", "crit-chance"),
    ("CritMultiplier", "crit-multiplier"),
    ("TotalDPS", "hit-dps"),
];

/// The slice file.
#[expect(
    dead_code,
    reason = "some fields are documentation for people; reading them checks the file's shape"
)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SliceFile {
    /// [`FORMAT`].
    pub format: u32,
    /// [`REVISION`].
    pub revision: u32,
    /// `proposed` or `confirmed`.
    pub status: String,
    /// The oracle it was written against.
    pub oracle: String,
    /// Notes.
    pub notes: Vec<String>,
    /// The fields.
    pub fields: Vec<SliceField>,
    /// What is out of the slice, and why.
    pub not_in_slice: Vec<serde_json::Value>,
}

/// One slice field.
#[expect(
    dead_code,
    reason = "some fields are documentation for people; reading them checks the file's shape"
)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SliceField {
    /// PoB's id (as `OUR_NAMES` lists it).
    pub id: String,
    /// `character`, `defence`, or `hit`.
    pub group: String,
    /// The `main_output` key.
    pub pob_key: String,
    /// `always`, `attack`, or `spell` (informational: a field the oracle
    /// does not write and we do not compute is listed as absent).
    pub applies: String,
    /// How the value is compared.
    pub comparison: Rule,
    /// A note.
    #[serde(default)]
    pub note: Option<String>,
    /// The oracle's breakdown of the field, when it has one.
    #[serde(default)]
    pub breakdown: Option<BreakdownSpec>,
}

/// A field's comparison rule.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(tag = "rule", rename_all = "lowercase", deny_unknown_fields)]
pub enum Rule {
    /// Equal doubles. (A struct variant: serde ignores unknown fields next
    /// to the tag of a unit variant.)
    Exact {},
    /// `|a - b| <= tolerance`.
    Abs {
        /// The tolerance.
        tolerance: f64,
        /// Why (ADR-006: no broad close-enough rule).
        #[serde(rename = "reason")]
        _reason: NonEmpty,
    },
    /// `|a - b| <= tolerance * max(|a|, |b|)`.
    Rel {
        /// The tolerance.
        tolerance: f64,
        /// Why.
        #[serde(rename = "reason")]
        _reason: NonEmpty,
    },
}

/// A text that must not be empty (checked by [`SliceFile::read`]).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(from = "String")]
pub struct NonEmpty(bool);

impl From<String> for NonEmpty {
    fn from(text: String) -> Self {
        Self(!text.trim().is_empty())
    }
}

impl Rule {
    /// The register's comparison with the same meaning.
    pub const fn comparison(self) -> Comparison {
        match self {
            Self::Exact {} => Comparison::Exact {},
            Self::Abs { tolerance, .. } => Comparison::Abs { tolerance },
            Self::Rel { tolerance, .. } => Comparison::Rel { tolerance },
        }
    }

    fn valid(self) -> bool {
        match self {
            Self::Exact {} => true,
            Self::Abs { tolerance, _reason } | Self::Rel { tolerance, _reason } => {
                tolerance.is_finite() && tolerance > 0.0 && _reason.0
            }
        }
    }
}

/// A breakdown's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Shape {
    /// Lines of terms: base, factors, total.
    Chain,
    /// Rows per slot.
    Slots,
    /// `Min:`, `Max:`, `Total:` lines.
    MinMaxTotal,
}

/// A field's breakdown.
#[expect(
    dead_code,
    reason = "some fields are documentation for people; reading them checks the file's shape"
)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BreakdownSpec {
    /// The breakdown's key in the oracle's `breakdowns`.
    pub pob_key: String,
    /// The shape.
    pub shape: Shape,
    /// PoB's source of the lines.
    pub source: String,
    /// When PoB writes no breakdown.
    #[serde(default)]
    pub absent_when: Option<String>,
    /// The terms (`chain`, `min-max-total`).
    #[serde(default)]
    pub terms: Vec<TermSpec>,
    /// The row (`slots`).
    #[serde(default)]
    pub row: Option<RowSpec>,
}

/// One term of a breakdown.
#[expect(
    dead_code,
    reason = "some fields are documentation for people; reading them checks the file's shape"
)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TermSpec {
    /// The term's name, as the parser names lines.
    pub term: String,
    /// The node of our graph it is compared with.
    pub graph: String,
    /// PoB's line format.
    pub line: String,
    /// What it sums or multiplies, in words.
    pub contributions: Vec<String>,
    /// When PoB leaves the line out (the term is then 1).
    #[serde(default)]
    pub optional: Option<String>,
    /// A note.
    #[serde(default)]
    pub note: Option<String>,
}

/// A slots row.
#[expect(
    dead_code,
    reason = "some fields are documentation for people; reading them checks the file's shape"
)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowSpec {
    /// The row's node (`step.slot.<source>`).
    pub node: String,
    /// What a row's source is.
    pub source: String,
    /// The base.
    pub base: PartSpec,
    /// The increased factor.
    pub inc: PartSpec,
    /// The more factor.
    pub more: PartSpec,
    /// The row's total.
    pub total: PartSpec,
}

/// One part of a slots row.
#[expect(
    dead_code,
    reason = "some fields are documentation for people; reading them checks the file's shape"
)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartSpec {
    /// The node of our graph below the row's node.
    pub graph: String,
    /// PoB's format.
    pub line: String,
    /// What it is, in words.
    pub contributions: Vec<String>,
}

/// The terms the parser gives per shape.
const CHAIN_TERMS: [&str; 8] = [
    "base",
    "increased",
    "more",
    "average",
    "rate",
    "dps-multiplier",
    "quantity-multiplier",
    "total",
];
const MIN_MAX_TOTAL_TERMS: [&str; 3] = ["min", "max", "total"];

/// A graph reference: `base`, `increased`, `more`, `result`, `row`,
/// `step.<name>`, or `field:<id>`.
fn graph_reference(text: &str, ids: &BTreeSet<&str>) -> bool {
    match text {
        "base" | "increased" | "more" | "result" | "row" => true,
        _ => {
            text.strip_prefix("step.")
                .is_some_and(|s| !s.is_empty() && !s.contains(char::is_whitespace))
                || text
                    .strip_prefix("field:")
                    .is_some_and(|id| ids.contains(id))
        }
    }
}

impl SliceFile {
    /// Reads and checks a slice file: format, revision, unique ids that
    /// are `OUR_NAMES`' ids, valid rules, breakdowns per shape with known
    /// terms, a format per line, and graph references.
    pub fn read(bytes: &[u8]) -> Result<Self, String> {
        let file: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if file.format != FORMAT || file.revision != REVISION {
            return Err(format!(
                "format {} revision {}: this reader knows format {FORMAT} revision {REVISION}",
                file.format, file.revision
            ));
        }
        let ids: BTreeSet<&str> = file.fields.iter().map(|f| f.id.as_str()).collect();
        if ids.len() != file.fields.len() {
            return Err("a field id is listed twice".to_owned());
        }
        let known: BTreeSet<&str> = OUR_NAMES.iter().map(|(id, _)| *id).collect();
        if ids != known {
            return Err(format!(
                "the fields must be exactly the slice's {} fields",
                known.len()
            ));
        }
        for f in &file.fields {
            let bad = |what: String| Err(format!("{}: {what}", f.id));
            if !f.comparison.valid() {
                return bad("an abs or rel rule needs a positive tolerance and a reason".to_owned());
            }
            let Some(b) = &f.breakdown else { continue };
            match (b.shape, b.terms.is_empty(), &b.row) {
                (Shape::Slots, true, Some(row)) => {
                    if row.node != "step.slot.<source>" {
                        return bad(format!("row node {:?}", row.node));
                    }
                    for (name, part, graph) in [
                        ("base", &row.base, "base"),
                        ("inc", &row.inc, "increased"),
                        ("more", &row.more, "more"),
                        ("total", &row.total, "row"),
                    ] {
                        if part.graph != graph {
                            return bad(format!("row {name}: graph must be {graph}"));
                        }
                        if Format::of_line(&part.line).is_none() {
                            return bad(format!("row {name}: no format in {:?}", part.line));
                        }
                    }
                }
                (Shape::Chain | Shape::MinMaxTotal, false, None) => {
                    let names: &[&str] = if b.shape == Shape::Chain {
                        &CHAIN_TERMS
                    } else {
                        &MIN_MAX_TOTAL_TERMS
                    };
                    let mut seen = BTreeSet::new();
                    for t in &b.terms {
                        if !names.contains(&t.term.as_str()) || !seen.insert(t.term.as_str()) {
                            return bad(format!("term {:?} unknown or twice", t.term));
                        }
                        if Format::of_line(&t.line).is_none() {
                            return bad(format!("term {}: no format in {:?}", t.term, t.line));
                        }
                        if t.graph == "row" || !graph_reference(&t.graph, &ids) {
                            return bad(format!("term {}: graph {:?}", t.term, t.graph));
                        }
                    }
                }
                _ => return bad("a breakdown's terms or row do not fit its shape".to_owned()),
            }
        }
        Ok(file)
    }

    /// Our field name for a PoB id.
    pub fn our_name(id: &str) -> Option<&'static str> {
        OUR_NAMES.iter().find(|(p, _)| *p == id).map(|(_, o)| *o)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const COMMITTED: &[u8] = include_bytes!("../../../../docs/phase-3/slice-fields.json");

    #[test]
    fn our_names_should_be_the_engine_slice() {
        let ours: BTreeSet<String> = OUR_NAMES.iter().map(|(_, o)| (*o).to_owned()).collect();
        let engine: BTreeSet<String> = loom_calc::poe1::SLICE
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(ours, engine);
        assert_eq!(SliceFile::our_name("TotalDPS"), Some("hit-dps"));
        assert_eq!(SliceFile::our_name("CombinedDPS"), None);
    }

    #[test]
    fn the_committed_slice_file_should_read() {
        let file = SliceFile::read(COMMITTED).unwrap();
        assert_eq!(file.fields.len(), 24);
        let with_breakdown = file.fields.iter().filter(|f| f.breakdown.is_some()).count();
        assert_eq!(with_breakdown, 10);
    }

    fn with(change: impl FnOnce(&mut Value)) -> Result<SliceFile, String> {
        let mut v: Value = serde_json::from_slice(COMMITTED).unwrap();
        change(&mut v);
        SliceFile::read(v.to_string().as_bytes())
    }

    fn field<'v>(v: &'v mut Value, id: &str) -> &'v mut Value {
        v["fields"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|f| f["id"] == id)
            .unwrap()
    }

    /// A change to the committed slice file's JSON.
    type Change = Box<dyn FnOnce(&mut Value)>;

    #[test]
    fn read_should_refuse_a_broken_file() {
        let cases: Vec<(Change, &str)> = vec![
            (Box::new(|v| v["revision"] = 1.into()), "revision"),
            (Box::new(|v| v["extra"] = 1.into()), "unknown field"),
            (
                Box::new(|v| field(v, "Life")["id"] = "Life2".into()),
                "exactly",
            ),
            (
                Box::new(|v| {
                    field(v, "Str")["comparison"] =
                        json!({"rule": "abs", "tolerance": 0.1, "reason": " "})
                }),
                "tolerance and a reason",
            ),
            (
                Box::new(|v| {
                    field(v, "Str")["comparison"] =
                        json!({"rule": "rel", "tolerance": 0, "reason": "x"})
                }),
                "tolerance and a reason",
            ),
            (
                Box::new(|v| {
                    field(v, "Str")["comparison"] = json!({"rule": "exact", "tolerance": 1})
                }),
                "unknown field",
            ),
            (
                Box::new(|v| field(v, "Life")["breakdown"]["terms"][0]["term"] = "flat".into()),
                "unknown or twice",
            ),
            (
                Box::new(|v| {
                    field(v, "Life")["breakdown"]["terms"][1]["line"] = "x ^8(increased)".into()
                }),
                "no format",
            ),
            (
                Box::new(|v| {
                    field(v, "Life")["breakdown"]["terms"][0]["graph"] = "field:Nope".into()
                }),
                "graph",
            ),
            (
                Box::new(|v| field(v, "Life")["breakdown"]["terms"][0]["graph"] = "row".into()),
                "graph",
            ),
            (
                Box::new(|v| {
                    field(v, "Armour")["breakdown"]["row"]["total"]["graph"] = "result".into()
                }),
                "graph must be row",
            ),
            (
                Box::new(|v| field(v, "Armour")["breakdown"]["shape"] = "chain".into()),
                "fit its shape",
            ),
        ];
        for (change, expected) in cases {
            let err = with(change).unwrap_err();
            assert!(err.contains(expected), "{expected}: {err}");
        }
    }
}
