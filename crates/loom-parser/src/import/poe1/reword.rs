//! Reviewed rewordings for PoE1 item lines (ADR-015 Amendment 3).
//!
//! Item text in PoB builds is often older than the bundle's stat
//! descriptions ("+1 to Maximum Endurance Charge", where the game now says
//! "Charges"). A line no description reads is looked up here by its
//! template (each number, or each range `(a-b)`, written `#`); an accepted
//! pair gives the current wording, which the resolver reads with the same
//! numbers in the same order and marks [`loom_domain::Adapted::Reworded`].
//!
//! The table is `tools/rewordings/items.json`, compiled in: a pair is
//! accepted only when PoB v2.67.2 parses both wordings into identical mods
//! (`loom-parity-cli item-rewordings check`) and a review finds the change
//! one of wording alone. A rejected or unlisted line stays `no-template`.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;

/// The reviewed table, as committed.
pub const TABLE: &str = include_str!("../../../../../tools/rewordings/items.json");

/// One pair of the table.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Pair {
    /// The older template.
    pub old: String,
    /// The current template.
    pub current: String,
    /// Why it is accepted or rejected (absent for a candidate).
    #[serde(default)]
    pub why: String,
}

/// The table's lists.
#[derive(Debug, Clone, Deserialize)]
pub struct Table {
    /// The table's format (1).
    pub format: u32,
    /// Pairs applied as a tolerance.
    pub accepted: Vec<Pair>,
    /// Pairs looked at and not applied, with the reason.
    #[serde(default)]
    pub rejected: Vec<Pair>,
    /// Pairs not reviewed yet (never applied; checked against PoB only).
    #[serde(default)]
    pub candidates: Vec<Pair>,
}

impl Table {
    /// Parses a table.
    ///
    /// # Errors
    /// The JSON error, a format other than 1, or a pair whose two
    /// templates differ in their count of `#`.
    pub fn parse(text: &str) -> Result<Self, String> {
        let table: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if table.format != 1 {
            return Err(format!("format {} (1 expected)", table.format));
        }
        for p in table
            .accepted
            .iter()
            .chain(&table.rejected)
            .chain(&table.candidates)
        {
            if slots(&p.old) != slots(&p.current) {
                return Err(format!("{:?}: its two templates differ in numbers", p.old));
            }
        }
        Ok(table)
    }
}

/// The committed table's accepted pairs, by old template.
fn accepted() -> &'static BTreeMap<String, String> {
    static ACCEPTED: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    ACCEPTED.get_or_init(|| {
        // The table is compiled in and checked by this module's tests, so
        // a parse failure cannot reach a release; it would read as an
        // empty table (no line reworded) rather than stop an import.
        Table::parse(TABLE)
            .map(|t| t.accepted.into_iter().map(|p| (p.old, p.current)).collect())
            .unwrap_or_default()
    })
}

fn slots(template: &str) -> usize {
    template.matches('#').count()
}

/// The length of a number at the start of `s`: digits, with at most one
/// decimal part.
fn number_len(s: &str) -> usize {
    let b = s.as_bytes();
    let mut i = b.iter().take_while(|c| c.is_ascii_digit()).count();
    if i > 0 && b.get(i) == Some(&b'.') && b.get(i + 1).is_some_and(u8::is_ascii_digit) {
        let fraction = b.get(i + 1..).unwrap_or_default();
        i += 1 + fraction.iter().take_while(|c| c.is_ascii_digit()).count();
    }
    i
}

/// The length of a range `(a-b)` at the start of `s`, if one is there.
fn range_len(s: &str) -> Option<usize> {
    let rest = s.strip_prefix('(')?;
    let a = number_len(rest);
    let rest = rest.get(a..)?.strip_prefix('-')?;
    let b = number_len(rest);
    (a > 0 && b > 0 && rest.get(b..)?.starts_with(')')).then_some(a + b + 3)
}

/// A line's template (each number or range `(a-b)` as `#`) and its
/// numbers' text, in order. A `#` in the line itself is kept, so such a
/// line never matches a template slot it does not fill.
#[must_use]
pub fn template(text: &str) -> (String, Vec<&str>) {
    let mut out = String::with_capacity(text.len());
    let mut numbers = Vec::new();
    let mut i = 0;
    while let Some(rest) = text.get(i..).filter(|r| !r.is_empty()) {
        let len = range_len(rest).unwrap_or_else(|| number_len(rest));
        if len > 0 {
            out.push('#');
            numbers.push(&rest[..len]);
            i += len;
        } else {
            let c = rest.chars().next().unwrap_or_default();
            if c == '#' {
                // Never a slot: a literal `#` stays distinct.
                out.push_str("\\#");
            } else {
                out.push(c);
            }
            i += c.len_utf8();
        }
    }
    (out, numbers)
}

/// `template` with its `#` slots filled by `numbers` in order.
#[must_use]
pub fn fill(template: &str, numbers: &[&str]) -> String {
    let mut out = String::with_capacity(template.len() + 8);
    let mut numbers = numbers.iter();
    for c in template.chars() {
        match (c, numbers.len()) {
            ('#', n) if n > 0 => out.push_str(numbers.next().copied().unwrap_or_default()),
            _ => out.push(c),
        }
    }
    out
}

/// The current wording of `text` when the committed table accepts its
/// template, with its numbers in place.
#[must_use]
pub fn reword(text: &str) -> Option<String> {
    let (t, numbers) = template(text);
    accepted().get(&t).map(|current| fill(current, &numbers))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_committed_table_should_parse() {
        let table = Table::parse(TABLE).expect("tools/rewordings/items.json");
        assert!(!table.accepted.is_empty());
        for p in &table.accepted {
            assert!(!p.why.is_empty(), "{:?} has no reason", p.old);
            assert_ne!(p.old, p.current);
        }
        for p in &table.rejected {
            assert!(!p.why.is_empty(), "{:?} has no reason", p.old);
        }
    }

    #[test]
    fn a_template_should_be_unique_in_the_table() {
        let table = Table::parse(TABLE).expect("table");
        let mut olds: Vec<&str> = table
            .accepted
            .iter()
            .chain(&table.rejected)
            .chain(&table.candidates)
            .map(|p| p.old.as_str())
            .collect();
        let n = olds.len();
        olds.sort_unstable();
        olds.dedup();
        assert_eq!(olds.len(), n, "an old template is listed twice");
    }

    #[test]
    fn template_should_write_numbers_and_ranges_as_slots() {
        assert_eq!(
            template("+1 to Maximum Endurance Charge"),
            ("+# to Maximum Endurance Charge".to_owned(), vec!["1"])
        );
        assert_eq!(
            template("(38-40)% increased Mana Cost of 1.5 Skills"),
            (
                "#% increased Mana Cost of # Skills".to_owned(),
                vec!["(38-40)", "1.5"]
            )
        );
        assert_eq!(template("Wave's -6% to it").1, vec!["6"]);
        assert_eq!(template("A #1 line").0, "A \\## line");
    }

    #[test]
    fn reword_should_keep_the_numbers_in_order() {
        assert_eq!(
            reword("+1 to Maximum Endurance Charge").as_deref(),
            Some("+1 to Maximum Endurance Charges")
        );
        assert_eq!(
            reword("(10-15)% reduced Attack Speed during Flask effect").as_deref(),
            Some("(10-15)% reduced Attack Speed during Effect")
        );
        assert_eq!(
            reword("Chaos Damage does not bypass Energy Shield").as_deref(),
            Some("Chaos Damage taken does not bypass Energy Shield")
        );
    }

    #[test]
    fn reword_should_leave_an_unlisted_or_rejected_line() {
        assert_eq!(reword("+1 to Maximum Endurance Charges"), None);
        assert_eq!(reword("7% reduced Mana Reserved"), None);
        assert_eq!(
            reword("10% of Damage from Hits is taken from your Spectres' Life before you"),
            None
        );
        assert_eq!(reword("Essence of Envy"), None);
    }

    #[test]
    fn parse_should_refuse_a_pair_whose_numbers_differ() {
        let bad = r##"{"format":1,"accepted":[{"old":"# a","current":"b","why":"x"}]}"##;
        assert!(Table::parse(bad).is_err());
        assert!(Table::parse(r#"{"format":2,"accepted":[]}"#).is_err());
    }
}
