//! Item property lines (Spike B, PR B5d; area 4 of the B4b verdict,
//! `docs/spikes/spike-b-resolution.md` section 7a). Since P2-2d the reading
//! is `loom_parser::import::properties` and the named ids are PoE1 data in
//! `crate::poe1`; this module loads the tables and formats the report's
//! `detail`.
//!
//! PoB items sometimes carry lines of the game's item display that are
//! properties, not modifiers. Each is read from game data and resolved as
//! a typed property (kind `item-property` or `catalyst-quality`):
//! - a `ClientStrings` property text with numeric placeholders
//!   ([`crate::poe1::PROPERTY_IDS`]: `Lasts {0} Seconds`, `Currently has {0} Charges`,
//!   `Consumes {0} of {1} Charges on use`, ...): detail `property=<id>` and
//!   each placeholder's value;
//! - a `ClientStrings` label in PoB's spec form `<label>: <number>`
//!   ([`crate::poe1::LABEL_IDS`]: `Memory Strands: 63`): detail `property=<id>`, `value`;
//! - catalyst quality `<AlternateQualityTypes.Description>: +N%`
//!   (`Quality (Attribute Modifiers): +20%`): the stats of the type's
//!   `CatalystMod` row (`jewellery_attribute_quality=1`, a stat ID) and
//!   `quality=N`.
//!
//! Values are read, not checked against the base (flask base durations and
//! charges are not extracted). A line that matches nothing keeps its
//! category.

use loom_parser::import::properties::{Missing, Properties, PropertyMatch, QualityType};
use serde_json::Value;

use crate::extract::{Read, Table};
use crate::poe1;

/// A resolved property line, as the report writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    /// `item-property` or `catalyst-quality`.
    pub kind: &'static str,
    /// `key=value` entries (stat IDs first for catalyst quality).
    pub detail: Vec<String>,
}

/// The report form of a property match.
#[must_use]
pub fn property(m: &PropertyMatch) -> Property {
    let detail = match m {
        PropertyMatch::Template { property, values } => {
            let mut d = vec![format!("property={property}")];
            d.extend(values.iter().map(|(slot, v)| format!("{{{slot}}}={v}")));
            d
        }
        PropertyMatch::Label { property, value } => {
            vec![format!("property={property}"), format!("value={value}")]
        }
        PropertyMatch::Quality {
            quality_type,
            stats,
            quality,
        } => {
            let mut d: Vec<String> = stats.iter().map(|(s, v)| format!("{s}={v}")).collect();
            d.push(format!("quality_type={quality_type}"));
            d.push(format!("quality={quality}"));
            d
        }
    };
    Property {
        kind: m.kind(),
        detail,
    }
}

/// The property a line shows, in the report's form.
#[must_use]
pub fn resolve(properties: &Properties, line: &str) -> Option<Property> {
    properties.resolve(line).map(|m| property(&m))
}

/// Named ids not read and duplicate quality descriptions dropped, as the
/// Spike B summary names them (`AlternateQualityTypes:<description>`).
#[must_use]
pub fn missing(properties: &Properties) -> Vec<String> {
    properties
        .missing()
        .iter()
        .map(|m| match m {
            Missing::Text(id) => id.clone(),
            Missing::QualityType(d) => format!("AlternateQualityTypes:{d}"),
        })
        .collect()
}

/// Reads `ClientStrings` and, when present, `AlternateQualityTypes` with
/// `Mods` and `Stats`. A missing `ClientStrings` gives no rules.
///
/// # Errors
/// A message when a table or column the others need is missing.
pub fn load(read: Read<'_>) -> Result<Properties, String> {
    let rules = poe1::item_rules().text;
    let mut texts: Vec<(String, String)> = Vec::new();
    if let Some(cs) = Table::read(read, "ClientStrings")? {
        let (id, text) = (cs.column("Id")?, cs.column("Text")?);
        for r in &cs.rows {
            if let (Some(i), Some(t)) = (Table::text(r, id), Table::text(r, text)) {
                texts.push((i.to_owned(), t.to_owned()));
            }
        }
    }
    let pairs = texts.iter().map(|(i, t)| (i.as_str(), t.as_str()));
    Ok(Properties::new(&rules, pairs, quality_types(read)?))
}

/// `AlternateQualityTypes` with each catalyst mod's fixed stats, in table
/// order; `None` without the table.
///
/// # Errors
/// A message when a table or column the others need is missing.
pub fn quality_types(read: Read<'_>) -> Result<Option<Vec<QualityType>>, String> {
    let Some(aq) = Table::read(read, "AlternateQualityTypes")? else {
        return Ok(None);
    };
    let mods = Table::require(read, "Mods")?;
    let stats = Table::require(read, "Stats")?;
    let stat_id = stats.column("Id")?;
    let (aq_id, desc, catalyst) = (
        aq.column("Id")?,
        aq.column("Description")?,
        aq.column("CatalystMod")?,
    );
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
    let mut qualities = Vec::new();
    for r in &aq.rows {
        let (Some(qid), Some(d)) = (Table::text(r, aq_id), Table::text(r, desc)) else {
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
        qualities.push(QualityType {
            id: qid.to_owned(),
            description: d.to_owned(),
            stats: list,
        });
    }
    Ok(Some(qualities))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props() -> Properties {
        Properties::new(
            &poe1::item_rules().text,
            [
                ("ItemDisplayFlaskBuffDurationNSeconds", "Lasts {0} Seconds"),
                (
                    "ItemDisplayChargesConsumptionOnUseNCharges",
                    "Consumes {0} of {1} Charges on use",
                ),
                ("ItemDisplayFlaskBuffDuration1Second", "Lasts {0} Second"),
                (
                    "ItemDisplayChargesCurrentOfMaxN",
                    "Currently has {0} of {1} Charges",
                ),
                ("ZanaInfluencedItem", "Memory Strands"),
            ],
            Some(vec![QualityType {
                id: "JewelleryAttributeQuality".to_owned(),
                description: "Quality (Attribute Modifiers)".to_owned(),
                stats: vec![("jewellery_attribute_quality".to_owned(), 1)],
            }]),
        )
    }

    #[test]
    fn resolve_should_read_property_templates_with_numbers() {
        let p = props();
        assert_eq!(
            resolve(&p, "Lasts 3.70 Seconds").unwrap().detail,
            ["property=ItemDisplayFlaskBuffDurationNSeconds", "{0}=3.70"]
        );
        assert_eq!(
            resolve(&p, "Consumes 20 of 40 Charges on use")
                .unwrap()
                .detail,
            [
                "property=ItemDisplayChargesConsumptionOnUseNCharges",
                "{0}=20",
                "{1}=40"
            ]
        );
        assert_eq!(resolve(&p, "Lasts many Seconds"), None);
        assert_eq!(resolve(&p, "Lasts 3 Seconds longer"), None);
    }

    #[test]
    fn resolve_should_read_labels_and_catalyst_quality() {
        let p = props();
        assert_eq!(
            resolve(&p, "Memory Strands: 63").unwrap().detail,
            ["property=ZanaInfluencedItem", "value=63"]
        );
        let q = resolve(&p, "Quality (Attribute Modifiers): +20%").unwrap();
        assert_eq!(q.kind, "catalyst-quality");
        assert_eq!(
            q.detail,
            [
                "jewellery_attribute_quality=1",
                "quality_type=JewelleryAttributeQuality",
                "quality=20"
            ]
        );
        assert_eq!(resolve(&p, "Quality (Unknown): +20%"), None);
        assert_eq!(resolve(&p, "Memory Strands: many"), None);
    }

    #[test]
    fn resolve_should_read_singular_and_current_of_max_templates() {
        let p = props();
        assert_eq!(
            resolve(&p, "Lasts 1 Second").unwrap().detail,
            ["property=ItemDisplayFlaskBuffDuration1Second", "{0}=1"]
        );
        assert_eq!(
            resolve(&p, "Currently has 20 of 40 Charges")
                .unwrap()
                .detail,
            [
                "property=ItemDisplayChargesCurrentOfMaxN",
                "{0}=20",
                "{1}=40"
            ]
        );
    }

    fn table(name: &str, columns: &[&str], rows: &str) -> String {
        let cols: Vec<String> = columns
            .iter()
            .map(|c| format!(r#"{{"name": "{c}", "references": null}}"#))
            .collect();
        format!(
            r#"{{"table": "{name}", "columns": [{}], "rows": {rows}}}"#,
            cols.join(", ")
        )
    }

    fn read(path: &str) -> Result<Vec<u8>, String> {
        let mut mod_cols = vec!["Id".to_owned()];
        for n in 1..=8 {
            mod_cols.extend([
                format!("StatsKey{n}"),
                format!("Stat{n}Min"),
                format!("Stat{n}Max"),
            ]);
        }
        let mod_cols: Vec<&str> = mod_cols.iter().map(String::as_str).collect();
        let mut row = vec![r#""JewelleryAttributeQuality""#.to_owned()];
        for n in 1..=8 {
            row.extend(
                match n {
                    1 => ["0", "1", "1"],
                    _ => ["null", "0", "0"],
                }
                .map(str::to_owned),
            );
        }
        let text = match path {
            "tables/ClientStrings.json" => table(
                "ClientStrings",
                &["Id", "Text"],
                r#"[["ItemDisplayFlaskBuffDurationNSeconds", "Lasts {0} Seconds"],
                    ["ZanaInfluencedItem", "Memory Strands"], ["Other", "Lasts {0} Seconds"]]"#,
            ),
            "tables/AlternateQualityTypes.json" => table(
                "AlternateQualityTypes",
                &["Id", "Description", "CatalystMod"],
                r#"[["JewelleryAttributeQuality", "Quality (Attribute Modifiers)", 0],
                    ["MapRarityQuality", "Quality (Rarity)", null],
                    ["Twin1", "Quality (Twin)", 0], ["Twin2", "Quality (Twin)", null]]"#,
            ),
            "tables/Mods.json" => table("Mods", &mod_cols, &format!("[[{}]]", row.join(", "))),
            "tables/Stats.json" => table("Stats", &["Id"], r#"[["jewellery_attribute_quality"]]"#),
            _ => return Err(format!("{path}: not found")),
        };
        Ok(text.into_bytes())
    }

    #[test]
    fn load_should_read_named_texts_and_catalyst_stats() {
        let p = load(&read).unwrap();
        assert_eq!(p.inputs(), (1, 1, 2));
        let q = resolve(&p, "Quality (Attribute Modifiers): +20%").unwrap();
        assert_eq!(
            (q.kind, q.detail.as_slice()),
            (
                "catalyst-quality",
                [
                    "jewellery_attribute_quality=1",
                    "quality_type=JewelleryAttributeQuality",
                    "quality=20"
                ]
                .map(str::to_owned)
                .as_slice()
            )
        );
        // No catalyst stat: a property with the type's identity.
        let r = resolve(&p, "Quality (Rarity): +10%").unwrap();
        assert_eq!(
            (r.kind, r.detail.as_slice()),
            (
                "item-property",
                ["quality_type=MapRarityQuality", "quality=10"]
                    .map(str::to_owned)
                    .as_slice()
            )
        );
        // Two rows with one description and different stats: dropped, recorded.
        assert_eq!(resolve(&p, "Quality (Twin): +5%"), None);
        assert!(missing(&p).contains(&"AlternateQualityTypes:Quality (Twin)".to_owned()));
        // Named ids absent from the fixture's `ClientStrings` are recorded.
        assert!(missing(&p).contains(&"ItemDisplayChargesNCharges".to_owned()));
        assert!(resolve(&p, "Lasts 2 Seconds").is_some());
        let none = |_: &str| -> Result<Vec<u8>, String> { Err("absent".to_owned()) };
        assert_eq!(load(&none).unwrap().inputs(), (0, 0, 0));
    }
}
