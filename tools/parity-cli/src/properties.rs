//! Item property lines (Spike B, PR B5d; area 4 of the B4b verdict,
//! `docs/spikes/spike-b-resolution.md` section 7a).
//!
//! PoB items sometimes carry lines of the game's item display that are
//! properties, not modifiers. Each is read from game data and resolved as
//! a typed property (kind `item-property` or `catalyst-quality`):
//! - a `ClientStrings` property text with numeric placeholders
//!   ([`PROPERTY_IDS`]: `Lasts {0} Seconds`, `Currently has {0} Charges`,
//!   `Consumes {0} of {1} Charges on use`, ...): detail `property=<id>` and
//!   each placeholder's value;
//! - a `ClientStrings` label in PoB's spec form `<label>: <number>`
//!   ([`LABEL_IDS`]: `Memory Strands: 63`): detail `property=<id>`, `value`;
//! - catalyst quality `<AlternateQualityTypes.Description>: +N%`
//!   (`Quality (Attribute Modifiers): +20%`): the stats of the type's
//!   `CatalystMod` row (`jewellery_attribute_quality=1`, a stat ID) and
//!   `quality=N`.
//!
//! Values are read, not checked against the base (flask base durations and
//! charges are not extracted). A line that matches nothing keeps its
//! category.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::extract::{Read, Table};

/// `ClientStrings` ids of item-display property texts (the categorizer's
/// `item-property` ids with numeric placeholders).
pub const PROPERTY_IDS: [&str; 8] = [
    "ItemDisplayFlaskBuffDuration1Second",
    "ItemDisplayFlaskBuffDurationNSeconds",
    "ItemDisplayChargesConsumptionOnUse1Charge",
    "ItemDisplayChargesConsumptionOnUseNCharges",
    "ItemDisplayCharges1Charge",
    "ItemDisplayChargesNCharges",
    "ItemDisplayChargesCurrentOfMax1",
    "ItemDisplayChargesCurrentOfMaxN",
];

/// `ClientStrings` ids of property labels PoB writes as `<label>: <number>`.
pub const LABEL_IDS: [&str; 1] = ["ZanaInfluencedItem"];

/// A text split at its placeholders: literal parts and placeholder numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Template {
    /// `parts.len() == slots.len() + 1`.
    parts: Vec<String>,
    slots: Vec<u8>,
}

impl Template {
    /// `None` when a placeholder is not `{0}`..`{9}` or two are adjacent.
    fn new(text: &str) -> Option<Self> {
        let (mut parts, mut slots) = (vec![String::new()], Vec::new());
        let mut rest = text;
        while let Some((before, after)) = rest.split_once('{') {
            parts.last_mut()?.push_str(before);
            let (slot, tail) = after.split_once('}')?;
            let [d @ b'0'..=b'9'] = slot.as_bytes() else {
                return None;
            };
            slots.push(d - b'0');
            parts.push(String::new());
            rest = tail;
        }
        parts.last_mut()?.push_str(rest);
        // A literal must separate two values.
        if parts.iter().skip(1).rev().skip(1).any(String::is_empty) {
            return None;
        }
        Some(Self { parts, slots })
    }

    /// The numeric values of a line, by placeholder, in text order.
    fn read(&self, line: &str) -> Option<Vec<(u8, String)>> {
        let mut rest = line.trim().strip_prefix(self.parts.first()?.as_str())?;
        let mut out = Vec::new();
        for (slot, next) in self.slots.iter().zip(self.parts.iter().skip(1)) {
            let (value, tail) = if next.is_empty() {
                (rest, "")
            } else {
                rest.split_once(next.as_str())?
            };
            if !number(value) {
                return None;
            }
            out.push((*slot, value.to_owned()));
            rest = tail;
        }
        rest.is_empty().then_some(out)
    }
}

/// A plain number: digits with an optional decimal part.
fn number(v: &str) -> bool {
    let (whole, frac) = v.split_once('.').unwrap_or((v, "0"));
    !whole.is_empty()
        && !frac.is_empty()
        && whole.bytes().all(|b| b.is_ascii_digit())
        && frac.bytes().all(|b| b.is_ascii_digit())
}

/// A resolved property line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    /// `item-property` or `catalyst-quality`.
    pub kind: &'static str,
    /// `key=value` entries (stat IDs first for catalyst quality).
    pub detail: Vec<String>,
}

/// Property texts and catalyst quality types read from the extract.
#[derive(Debug, Clone, Default)]
pub struct Properties {
    templates: Vec<(String, Template)>,
    labels: Vec<(String, String)>,
    /// `AlternateQualityTypes.Description` to its `CatalystMod` stats.
    qualities: BTreeMap<String, Vec<(String, i64)>>,
}

impl Properties {
    /// Reads `ClientStrings` and, when present, `AlternateQualityTypes`
    /// with `Mods` and `Stats`. A missing `ClientStrings` gives no rules.
    ///
    /// # Errors
    /// A message when a table or column the others need is missing.
    pub fn load(read: Read<'_>) -> Result<Self, String> {
        let mut out = Self::default();
        if let Some(cs) = Table::read(read, "ClientStrings")? {
            let (id, text) = (cs.column("Id")?, cs.column("Text")?);
            for r in &cs.rows {
                let (Some(i), Some(t)) = (Table::text(r, id), Table::text(r, text)) else {
                    continue;
                };
                if PROPERTY_IDS.contains(&i) {
                    if let Some(tpl) = Template::new(t) {
                        out.templates.push((i.to_owned(), tpl));
                    }
                } else if LABEL_IDS.contains(&i) && !t.contains('{') {
                    out.labels.push((i.to_owned(), t.to_owned()));
                }
            }
            out.templates.sort_by(|a, b| a.0.cmp(&b.0));
            out.labels.sort();
        }
        let Some(aq) = Table::read(read, "AlternateQualityTypes")? else {
            return Ok(out);
        };
        let mods = Table::require(read, "Mods")?;
        let stats = Table::require(read, "Stats")?;
        let stat_id = stats.column("Id")?;
        let (desc, catalyst) = (aq.column("Description")?, aq.column("CatalystMod")?);
        let mut slots = Vec::new();
        for n in 1..=8 {
            slots.push((
                mods.column(&format!("StatsKey{n}"))?,
                mods.column(&format!("Stat{n}Min"))?,
                mods.column(&format!("Stat{n}Max"))?,
            ));
        }
        let at = |rows: &[Vec<Value>], n: Option<i64>| {
            n.and_then(|n| usize::try_from(n).ok())
                .and_then(|n| rows.get(n).cloned())
        };
        for r in &aq.rows {
            let Some(d) = Table::text(r, desc) else {
                continue;
            };
            let mut list = Vec::new();
            if let Some(m) = at(&mods.rows, Table::int(r, catalyst)) {
                for &(k, lo, hi) in &slots {
                    let Some(s) = at(&stats.rows, Table::int(&m, k))
                        .and_then(|s| Table::text(&s, stat_id).map(str::to_owned))
                    else {
                        continue;
                    };
                    // A fixed value on the row (`(1, 1)`); a range is not a
                    // value the line shows, so it is left out.
                    if let (Some(a), Some(b)) = (Table::int(&m, lo), Table::int(&m, hi))
                        && a == b
                        && a != 0
                    {
                        list.push((s, a));
                    }
                }
            }
            out.qualities.insert(d.to_owned(), list);
        }
        Ok(out)
    }

    /// The property a line shows, if any (module docs).
    #[must_use]
    pub fn resolve(&self, line: &str) -> Option<Property> {
        let line = line.trim();
        for (id, tpl) in &self.templates {
            if let Some(values) = tpl.read(line) {
                let mut detail = vec![format!("property={id}")];
                detail.extend(values.iter().map(|(slot, v)| format!("{{{slot}}}={v}")));
                return Some(Property {
                    kind: "item-property",
                    detail,
                });
            }
        }
        let (label, value) = line.split_once(": ")?;
        if let Some((id, _)) = self.labels.iter().find(|(_, t)| t == label)
            && number(value)
        {
            return Some(Property {
                kind: "item-property",
                detail: vec![format!("property={id}"), format!("value={value}")],
            });
        }
        let stats = self.qualities.get(label)?;
        let n = value.strip_prefix('+')?.strip_suffix('%')?;
        if !number(n) {
            return None;
        }
        let mut detail: Vec<String> = stats.iter().map(|(s, v)| format!("{s}={v}")).collect();
        detail.push(format!("quality={n}"));
        Some(Property {
            kind: "catalyst-quality",
            detail,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props() -> Properties {
        let tpl = |id: &str, t: &str| (id.to_owned(), Template::new(t).unwrap());
        Properties {
            templates: vec![
                tpl("ItemDisplayFlaskBuffDurationNSeconds", "Lasts {0} Seconds"),
                tpl(
                    "ItemDisplayChargesConsumptionOnUseNCharges",
                    "Consumes {0} of {1} Charges on use",
                ),
            ],
            labels: vec![("ZanaInfluencedItem".to_owned(), "Memory Strands".to_owned())],
            qualities: [(
                "Quality (Attribute Modifiers)".to_owned(),
                vec![("jewellery_attribute_quality".to_owned(), 1)],
            )]
            .into(),
        }
    }

    #[test]
    fn resolve_should_read_property_templates_with_numbers() {
        let p = props();
        assert_eq!(
            p.resolve("Lasts 3.70 Seconds").unwrap().detail,
            ["property=ItemDisplayFlaskBuffDurationNSeconds", "{0}=3.70"]
        );
        assert_eq!(
            p.resolve("Consumes 20 of 40 Charges on use")
                .unwrap()
                .detail,
            [
                "property=ItemDisplayChargesConsumptionOnUseNCharges",
                "{0}=20",
                "{1}=40"
            ]
        );
        assert_eq!(p.resolve("Lasts many Seconds"), None);
        assert_eq!(p.resolve("Lasts 3 Seconds longer"), None);
    }

    #[test]
    fn resolve_should_read_labels_and_catalyst_quality() {
        let p = props();
        assert_eq!(
            p.resolve("Memory Strands: 63").unwrap().detail,
            ["property=ZanaInfluencedItem", "value=63"]
        );
        let q = p.resolve("Quality (Attribute Modifiers): +20%").unwrap();
        assert_eq!(q.kind, "catalyst-quality");
        assert_eq!(q.detail, ["jewellery_attribute_quality=1", "quality=20"]);
        assert_eq!(p.resolve("Quality (Unknown): +20%"), None);
        assert_eq!(p.resolve("Memory Strands: many"), None);
    }

    #[test]
    fn template_should_refuse_adjacent_or_odd_placeholders() {
        assert!(Template::new("{0}{1}").is_none());
        assert!(Template::new("Has {x}").is_none());
        assert!(Template::new("Has {0}").is_some());
    }
}
