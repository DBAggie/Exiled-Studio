//! Item property lines (Phase 2 plan, step 2d; lifted from Spike B's
//! `properties.rs`, PR B5d; ADR-015 Amendment 1, kinds `item-property` and
//! `catalyst-quality`).
//!
//! Some item lines are display properties, not modifiers. Each is read
//! from game data:
//! - a client text with numeric placeholders, named in
//!   [`TextRules::property_ids`] (`Lasts {0} Seconds`): the text's ID and
//!   each placeholder's value;
//! - a client-text label in the form `<label>: <number>`, named in
//!   [`TextRules::label_ids`]: the label's ID and the value;
//! - a quality type, `<description>: +N%`: the type's fixed stats and the
//!   quality. A type without stats is a property only.
//!
//! Values are read, not checked against the base.
//!
//! [`TextRules::property_ids`]: super::TextRules::property_ids
//! [`TextRules::label_ids`]: super::TextRules::label_ids

use std::collections::BTreeMap;

use super::TextRules;

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

/// A property a line shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyMatch {
    /// A property text with placeholders.
    Template {
        /// The text's ID.
        property: String,
        /// `(placeholder, value)` in text order.
        values: Vec<(u8, String)>,
    },
    /// A labelled value.
    Label {
        /// The label's ID.
        property: String,
        /// The value's text.
        value: String,
    },
    /// A quality type and the quality shown.
    Quality {
        /// The quality type's ID.
        quality_type: String,
        /// The type's fixed stats.
        stats: Vec<(String, i64)>,
        /// The quality's text (a plain number).
        quality: String,
    },
}

impl PropertyMatch {
    /// The `kind` (ADR-015 Amendment 1): `catalyst-quality` for a quality
    /// type with stats, else `item-property`.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Quality { stats, .. } if !stats.is_empty() => "catalyst-quality",
            _ => "item-property",
        }
    }
}

/// One quality type of the game data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualityType {
    /// ID.
    pub id: String,
    /// The description the line shows.
    pub description: String,
    /// Its fixed stats (a value the type's row sets to one number).
    pub stats: Vec<(String, i64)>,
}

/// Property texts, labels, and quality types read from the game data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Properties {
    templates: Vec<(String, Template)>,
    labels: Vec<(String, String)>,
    qualities: BTreeMap<String, (String, Vec<(String, i64)>)>,
    missing: Vec<String>,
}

impl Properties {
    /// From the game data: the client texts (`(id, text)`, any order) and
    /// the quality types (`None` when the data has none). A named ID the
    /// texts lack (or give in another shape) and a description two types
    /// share with different stats are recorded in [`Properties::missing`].
    #[must_use]
    pub fn new<'t>(
        rules: &TextRules,
        texts: impl IntoIterator<Item = (&'t str, &'t str)>,
        qualities: Option<Vec<QualityType>>,
    ) -> Self {
        let mut out = Self::default();
        for (i, t) in texts {
            if rules.property_ids.iter().any(|p| p == i) {
                if let Some(tpl) = Template::new(t) {
                    out.templates.push((i.to_owned(), tpl));
                }
            } else if rules.label_ids.iter().any(|l| l == i) && !t.contains('{') {
                out.labels.push((i.to_owned(), t.to_owned()));
            }
        }
        out.templates.sort_by(|a, b| a.0.cmp(&b.0));
        out.labels.sort();
        for id in rules.property_ids.iter().chain(&rules.label_ids) {
            let read = out.templates.iter().any(|(i, _)| i == id)
                || out.labels.iter().any(|(i, _)| i == id);
            if !read {
                out.missing.push(id.clone());
            }
        }
        let mut duplicates = Vec::new();
        for q in qualities.into_iter().flatten() {
            match out.qualities.get(&q.description) {
                Some((_, old)) if *old != q.stats => duplicates.push(q.description),
                Some(_) => {}
                None => {
                    out.qualities.insert(q.description, (q.id, q.stats));
                }
            }
        }
        duplicates.sort();
        duplicates.dedup();
        for d in duplicates {
            out.qualities.remove(&d);
            out.missing.push(format!("quality-type:{d}"));
        }
        out
    }

    /// Named IDs not read and duplicate quality descriptions dropped.
    #[must_use]
    pub fn missing(&self) -> &[String] {
        &self.missing
    }

    /// What was read: `(templates, labels, quality types)`.
    #[must_use]
    pub fn inputs(&self) -> (usize, usize, usize) {
        (
            self.templates.len(),
            self.labels.len(),
            self.qualities.len(),
        )
    }

    /// The property a line shows, if any (module docs).
    #[must_use]
    pub fn resolve(&self, line: &str) -> Option<PropertyMatch> {
        let line = line.trim();
        for (id, tpl) in &self.templates {
            if let Some(values) = tpl.read(line) {
                return Some(PropertyMatch::Template {
                    property: id.clone(),
                    values,
                });
            }
        }
        let (label, value) = line.split_once(": ")?;
        if let Some((id, _)) = self.labels.iter().find(|(_, t)| t == label)
            && number(value)
        {
            return Some(PropertyMatch::Label {
                property: id.clone(),
                value: value.to_owned(),
            });
        }
        let (qid, stats) = self.qualities.get(label)?;
        let n = value.strip_prefix('+')?.strip_suffix('%')?;
        number(n).then(|| PropertyMatch::Quality {
            quality_type: qid.clone(),
            stats: stats.clone(),
            quality: n.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> TextRules {
        TextRules {
            property_ids: vec!["Lasts".to_owned(), "Uses".to_owned()],
            label_ids: vec!["Strands".to_owned()],
            ..TextRules::default()
        }
    }

    fn props() -> Properties {
        Properties::new(
            &rules(),
            [
                ("Lasts", "Lasts {0} Seconds"),
                ("Uses", "Consumes {0} of {1} Charges on use"),
                ("Strands", "Memory Strands"),
                ("Other", "Lasts {0} Minutes"),
            ],
            Some(vec![
                QualityType {
                    id: "Attr".to_owned(),
                    description: "Quality (Attribute Modifiers)".to_owned(),
                    stats: vec![("attribute_quality".to_owned(), 1)],
                },
                QualityType {
                    id: "Plain".to_owned(),
                    description: "Quality (Plain)".to_owned(),
                    stats: Vec::new(),
                },
            ]),
        )
    }

    #[test]
    fn resolve_should_read_template_values_by_placeholder() {
        assert_eq!(
            props().resolve("Consumes 10 of 40 Charges on use"),
            Some(PropertyMatch::Template {
                property: "Uses".to_owned(),
                values: vec![(0, "10".to_owned()), (1, "40".to_owned())],
            })
        );
    }

    #[test]
    fn resolve_should_read_a_labelled_number() {
        assert_eq!(
            props().resolve("Memory Strands: 63"),
            Some(PropertyMatch::Label {
                property: "Strands".to_owned(),
                value: "63".to_owned(),
            })
        );
    }

    #[test]
    fn resolve_should_read_a_quality_type_and_its_kind() {
        let p = props();
        let kinds = [
            p.resolve("Quality (Attribute Modifiers): +20%")
                .map(|m| m.kind()),
            p.resolve("Quality (Plain): +5%").map(|m| m.kind()),
        ];
        assert_eq!(kinds, [Some("catalyst-quality"), Some("item-property")]);
    }

    #[test]
    fn resolve_should_refuse_text_that_is_not_a_number() {
        assert_eq!(props().resolve("Lasts four Seconds"), None);
    }

    #[test]
    fn new_should_record_named_ids_the_texts_lack() {
        let p = Properties::new(&rules(), [("Lasts", "Lasts {0} Seconds")], None);
        assert_eq!(p.missing(), ["Uses".to_owned(), "Strands".to_owned()]);
    }

    #[test]
    fn new_should_drop_a_description_two_types_share_with_different_stats() {
        let q = |id: &str, v: i64| QualityType {
            id: id.to_owned(),
            description: "Same".to_owned(),
            stats: vec![("s".to_owned(), v)],
        };
        let p = Properties::new(&rules(), [], Some(vec![q("A", 1), q("B", 2)]));
        assert!(p.missing().contains(&"quality-type:Same".to_owned()));
    }

    #[test]
    fn number_should_take_plain_digits_with_an_optional_decimal_part() {
        for ok in ["0", "40", "3.70"] {
            assert!(number(ok), "{ok}");
        }
        for bad in ["", "3.", ".5", "1,000", "-1", "+1", "x"] {
            assert!(!number(bad), "{bad}");
        }
    }

    #[test]
    fn template_should_refuse_adjacent_or_odd_placeholders() {
        assert!(Template::new("{0}{1}").is_none());
        assert!(Template::new("Has {x}").is_none());
        assert!(Template::new("Has {0}").is_some());
    }
}
