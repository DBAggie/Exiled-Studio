//! The categorization rules of `data-parity` (format 1): each names the
//! differences it covers (table, field patterns, key patterns), their
//! category, and the evidence. A difference gets the first rule that
//! matches it, in file order; one that no rule matches is
//! `uncategorized`. A rule that matches nothing is stale and fails the run.
//!
//! Patterns are exact text where `*` stands for any run of characters.

use serde::{Deserialize, Serialize};

/// The rules format this reader understands.
pub const FORMAT: u32 = 1;

/// The start of a GGG patch-note thread URL (`docs/parity/README.md`).
pub const PATCH_NOTES: &str = "https://www.pathofexile.com/forum/view-thread/";

/// Why the bundle and PoB differ (plan step 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    /// The game changed between PoB's data and the bundle's (a patch note
    /// cited).
    PatchDelta,
    /// PoB computes or edits the value instead of reading it (its source
    /// line cited).
    PobDerivation,
    /// The same game value in another form (a PoB source line or a game
    /// data row cited).
    Representation,
    /// The game has the value in a column the bundle does not carry (the
    /// game data column cited). Not a difference between the game and
    /// PoB: a calculation reaching it must report the field `Partial`.
    NotCarried,
}

impl Category {
    /// Whether the calc overlay takes PoB's value for this category: a
    /// different value, not another form of the same one, and one the
    /// bundle has a field for.
    pub const fn overlays(self) -> bool {
        matches!(self, Self::PatchDelta | Self::PobDerivation)
    }

    /// The category's name in the files.
    pub const fn name(self) -> &'static str {
        match self {
            Self::PatchDelta => "patch-delta",
            Self::PobDerivation => "pob-derivation",
            Self::Representation => "representation",
            Self::NotCarried => "not-carried",
        }
    }
}

/// The rules file.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    /// [`FORMAT`].
    pub format: u32,
    /// The game the rules are for.
    pub game: String,
    /// The rules, in matching order.
    pub rules: Vec<Rule>,
}

/// One rule.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    /// Unique, non-empty.
    pub id: String,
    /// The category it gives.
    pub category: Category,
    /// The table (one of [`super::TABLES`]).
    pub table: String,
    /// Field patterns; a difference matches when one matches its field.
    pub fields: Vec<String>,
    /// Key patterns; a difference matches when one matches its key.
    pub keys: Vec<String>,
    /// What the difference is and why it is not a bug.
    pub evidence: String,
    /// Citations: a patch-note thread URL for `patch-delta`; a PoB file
    /// and line (`v2.67.2:src/...:<line>`, or `pob@<commit>:src/...:<line>`
    /// for a later PoB) or a game data column (`game:<table>.<column>`)
    /// otherwise; `not-carried` needs the game column.
    pub sources: Vec<String>,
}

impl Rule {
    /// Whether the rule covers a difference.
    pub fn matches(&self, table: &str, key: &str, field: &str) -> bool {
        self.table == table
            && self.keys.iter().any(|p| glob(p, key))
            && self.fields.iter().any(|p| glob(p, field))
    }
}

/// `*` matches any run of characters (also none); everything else
/// matches itself.
pub fn glob(pattern: &str, text: &str) -> bool {
    let mut parts = pattern.split('*');
    let Some(first) = parts.next() else {
        return text.is_empty();
    };
    let Some(mut rest) = text.strip_prefix(first) else {
        return false;
    };
    let parts: Vec<&str> = parts.collect();
    let Some((last, middle)) = parts.split_last() else {
        // No `*`: the whole text must be the pattern.
        return rest.is_empty();
    };
    for part in middle {
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    rest.ends_with(last)
}

impl Rules {
    /// Reads and checks a rules file.
    ///
    /// # Errors
    /// A message naming the first problem: format, game, an empty or
    /// duplicate ID, an unknown table, no field or key pattern, no
    /// evidence or source, a `patch-delta` rule without a patch-note URL,
    /// or another category without a PoB or game citation.
    pub fn parse(bytes: &[u8], game: &str) -> Result<Self, String> {
        let rules: Self =
            serde_json::from_slice(bytes).map_err(|e| format!("the rules file: {e}"))?;
        if rules.format != FORMAT {
            return Err(format!(
                "the rules file: format {} (this tool reads {FORMAT})",
                rules.format
            ));
        }
        if rules.game != game {
            return Err(format!(
                "the rules file is for {}, the bundle for {game}",
                rules.game
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        for rule in &rules.rules {
            let id = &rule.id;
            if id.trim().is_empty() {
                return Err("a rule has an empty id".to_owned());
            }
            if !seen.insert(id.as_str()) {
                return Err(format!("rule {id}: duplicate id"));
            }
            if !super::TABLES.contains(&rule.table.as_str()) {
                return Err(format!("rule {id}: unknown table {}", rule.table));
            }
            if rule.fields.is_empty() || rule.keys.is_empty() {
                return Err(format!("rule {id}: no field or no key pattern"));
            }
            if rule.evidence.trim().is_empty() || rule.sources.is_empty() {
                return Err(format!("rule {id}: no evidence or no source"));
            }
            let cites = |prefixes: &[&str]| {
                rule.sources
                    .iter()
                    .any(|s| prefixes.iter().any(|p| s.starts_with(p)))
            };
            let (ok, needs) = match rule.category {
                Category::PatchDelta => (
                    cites(&[PATCH_NOTES]),
                    "a patch-note thread URL (https://www.pathofexile.com/forum/view-thread/...)",
                ),
                Category::PobDerivation | Category::Representation => (
                    cites(&["v2.67.2:", "pob@", "game:"]),
                    "a PoB citation (v2.67.2:<path>:<line>) or a game data column (game:<table>.<column>)",
                ),
                Category::NotCarried => (
                    cites(&["game:"]),
                    "the game data column (game:<table>.<column>)",
                ),
            };
            if !ok {
                return Err(format!(
                    "rule {id}: a {} rule needs {needs}",
                    rule.category.name()
                ));
            }
        }
        Ok(rules)
    }

    /// The first rule covering a difference.
    pub fn classify(&self, table: &str, key: &str, field: &str) -> Option<&Rule> {
        self.rules.iter().find(|r| r.matches(table, key, field))
    }
}
