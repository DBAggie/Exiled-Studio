//! `resolve-corpus` (Spike B, PR B3): reads every `<Item>` of the corpus
//! builds with `loom_parser::pob_item`, resolves each modifier line in its
//! item's stat-description scope, and counts every line under exactly one
//! outcome (ADR-015: nothing is dropped).
//!
//! - **Scope** (Spike B plan, "Scope by mod domain"): the base's
//!   `BaseItemTypes.ModDomain`. PoE1 values, from the vendored schema's
//!   `ModDomains` enum (`indexing: 1`) and checked on the rows of
//!   PoE1 26530219: 34 = `TINCTURE` (the 18 tincture bases, e.g. `Ironwood
//!   Tincture`) uses `tincture_stat_descriptions`, 38 = `GRAFT` (the 17
//!   graft bases, e.g. `Fleshgraft`) uses `graft_stat_descriptions`, every
//!   other domain uses `stat_descriptions`.
//! - **Base:** the base line of rare, unique, and relic items after
//!   `pob_item::base_name`; for normal and magic items (and items without
//!   a base line) `pob_item::magic_base` over every `BaseItemTypes.Name`.
//!   No base, a tie, a name not in `BaseItemTypes`, or names whose rows
//!   point to different scopes give `unknown-base/<reason>` for every
//!   modifier line of the item.
//! - **Lines:** consecutive included modifier lines of one section are
//!   resolved together (`Index::resolve_lines`, for multi-line templates).
//!   Excluded with a named rule (the three the plan allows): a line of a
//!   variant the item does not select (`not-selected-variant`), a
//!   `{custom}` line (`pob-custom`), a flag or influence line
//!   (`not-a-modifier`). Lines the reader could not place are
//!   `unrecognized-line`. Header and spec lines are item structure, counted
//!   by kind and not item lines.
//! - **Ambiguity (B4a):** an ambiguous line goes through the `Mods` filter
//!   (`mod_filter`, rules in its docs) with the item's base rows (domain,
//!   class, tags from `item_tags`, implicits), rarity, influences, and the
//!   line's section and flags. One candidate left: `resolved`, with the
//!   filter's steps in `filter`; otherwise `ambiguous`, with the reason.
//! - **`mod-backed` (B4a, reported, not gated):** every resolved non-unique
//!   line is backed or not by a fitting `Mods` row (`mod_filter::backing`).
//! - **Categories (B4b):** every line that is not resolved gets one
//!   category with its evidence (`categorize`); `special-case/author-note`
//!   and `data-gap/legacy-text` lines are excluded (maintainer decisions
//!   2026-11-10, B4b and B5b), and class
//!   requirement lines are excluded as `not-a-modifier`. The gate is
//!   computed in [`Gate`].
//!
//! **Scrubbing (ADR-007):** builds are named by the SHA-256 of their
//! content; item names, `{custom}` text, and unrecognized lines are never
//! written (only a hash of the text). Mod lines from game text are written.
//! The hash is SHA-256 cut to 64 bits and not salted, so a short, guessable
//! text could be confirmed by guessing; it hides, it does not encrypt.
//! Spec and prefix keys are written only when they are on the reader's
//! fixed lists (else hashed), so author text cannot leak through them.
//!
//! **Item text:** `v2.67.2:src/Classes/ItemsTab.lua` (load) calls
//! `item:ParseRaw(child)` for **each** string child of `<Item>`, and each
//! call starts the item over, so the last string child is the item; legacy
//! `<ModRange>` children only set ranges. The harness takes the last
//! non-blank direct text child the same way and counts items with more than
//! one (`items_with_several_texts`).

use std::collections::{BTreeMap, BTreeSet};

use loom_data::stat_desc::{Hand, Scope};
use loom_parser::pob_item::{self, BaseMatch, Item, LineKind, Modifier, Rarity, Section};
use loom_parser::resolve::{
    Candidate, Index, IndexStats, Outcome, RawValue, Resolution, Unresolved,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::categorize::{self, GameText, LineFacts};
use crate::extract::{Extract, Inputs, Read, Table};
use crate::item_tags::ItemTags;
use crate::lookup::{LookupReport, TableLookup};
use crate::mod_filter::{self, BaseRow, Context, LineContext, Mods};

/// Scope root files by name, in the order domains are tested.
const SCOPES: [&str; 3] = [
    "stat_descriptions",
    "tincture_stat_descriptions",
    "graft_stat_descriptions",
];

/// `ModDomain` values with their own scope (PoE1; see the module docs).
const DOMAIN_SCOPES: [(i64, &str); 2] = [
    (34, "tincture_stat_descriptions"),
    (38, "graft_stat_descriptions"),
];

fn scope_of_domain(domain: Option<i64>) -> &'static str {
    DOMAIN_SCOPES
        .iter()
        .find(|(d, _)| Some(*d) == domain)
        .map_or("stat_descriptions", |(_, s)| s)
}

/// Every PoB base name with a ` (...)` suffix (`itemBases["... (...)"]` in
/// `v2.67.2:src/Data/Bases/*.lua`, listed 2026-11-09): PoB splits one game
/// base name over several rows (`BaseItemTypes` has three `Two-Stone Ring`
/// rows, for example) and tells them apart by the suffix.
const POB_SUFFIXED_BASES: [&str; 13] = [
    "Avian Twins Talisman (Cold-To-Fire)",
    "Avian Twins Talisman (Cold-To-Lightning)",
    "Avian Twins Talisman (Fire-To-Cold)",
    "Avian Twins Talisman (Fire-To-Lightning)",
    "Avian Twins Talisman (Lightning-To-Cold)",
    "Monkey Paw Talisman (Frenzy)",
    "Monkey Paw Talisman (Power)",
    "Two-Stone Ring (Cold/Lightning)",
    "Two-Stone Ring (Fire/Cold)",
    "Two-Stone Ring (Fire/Lightning)",
    "Two-Toned Boots (Armour/Energy Shield)",
    "Two-Toned Boots (Armour/Evasion)",
    "Two-Toned Boots (Evasion/Energy Shield)",
];

/// `BaseItemTypes` names, the scopes their rows give, and the rows.
#[derive(Debug, Default)]
pub struct Bases {
    scopes: BTreeMap<String, BTreeSet<&'static str>>,
    rows: BTreeMap<String, Vec<BaseRow>>,
    /// Named rows whose tags are unknown (`item_tags`).
    pub tags_unknown: usize,
}

impl Bases {
    /// Reads `BaseItemTypes` (`Name`, `ModDomain`, `ItemClassesKey`,
    /// `InheritsFrom`, `TagsKeys`, `Implicit_ModsKeys`), `Tags`, and the
    /// item type files of every `InheritsFrom`.
    ///
    /// # Errors
    /// A message when a table or a column is missing.
    pub fn load(read: Read<'_>) -> Result<Self, String> {
        let tag_ids = mod_filter::tag_ids(read)?;
        let table = Table::require(read, "BaseItemTypes")?;
        let col = |n: &str| table.column(n);
        let (name, domain, class) = (col("Name")?, col("ModDomain")?, col("ItemClassesKey")?);
        let (inherits, tags, implicits) = (
            col("InheritsFrom")?,
            col("TagsKeys")?,
            col("Implicit_ModsKeys")?,
        );
        let index_list = |row: &[serde_json::Value], c: usize| -> Vec<usize> {
            row.get(c)
                .and_then(serde_json::Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(serde_json::Value::as_u64)
                        .filter_map(|v| usize::try_from(v).ok())
                        .collect()
                })
                .unwrap_or_default()
        };
        let mut item_tags = ItemTags::default();
        let mut bases = Self::default();
        for row in &table.rows {
            let Some(n) = Table::text(row, name).filter(|n| !n.is_empty()) else {
                continue;
            };
            let row_domain = Table::int(row, domain);
            bases
                .scopes
                .entry(n.to_owned())
                .or_default()
                .insert(scope_of_domain(row_domain));
            let row_tags = Table::text(row, inherits)
                .and_then(|path| item_tags.tags(read, path))
                .map(|mut t| {
                    t.extend(
                        index_list(row, tags)
                            .into_iter()
                            .filter_map(|i| tag_ids.get(i).cloned()),
                    );
                    t
                });
            if row_tags.is_none() {
                bases.tags_unknown += 1;
            }
            bases.rows.entry(n.to_owned()).or_default().push(BaseRow {
                domain: row_domain.unwrap_or(0),
                class: Table::int(row, class),
                tags: row_tags,
                implicits: index_list(row, implicits),
            });
        }
        Ok(bases)
    }

    #[cfg(test)]
    pub fn from_pairs(pairs: &[(&str, i64)]) -> Self {
        let rows: Vec<(&str, BaseRow)> = pairs
            .iter()
            .map(|(n, d)| {
                (
                    *n,
                    BaseRow {
                        domain: *d,
                        ..BaseRow::default()
                    },
                )
            })
            .collect();
        Self::from_rows(rows)
    }

    #[cfg(test)]
    pub fn from_rows(rows: Vec<(&str, BaseRow)>) -> Self {
        let mut bases = Self::default();
        for (n, row) in rows {
            bases
                .scopes
                .entry(n.to_owned())
                .or_default()
                .insert(scope_of_domain(Some(row.domain)));
            bases.rows.entry(n.to_owned()).or_default().push(row);
        }
        bases
    }

    /// The item's scope, or why there is none; `true` when the base is one
    /// of [`POB_SUFFIXED_BASES`] and was found after removing its ` (...)`
    /// suffix. Any other name is looked up as written.
    #[cfg(test)]
    fn scope(&self, item: &Item) -> Result<(&'static str, bool), String> {
        self.find(item).map(|f| (f.scope, f.suffix_removed))
    }

    /// The item's base: its rows and scope, or why there is none.
    fn find(&self, item: &Item) -> Result<Found<'_>, String> {
        let rarity = item.rarity.unwrap_or(Rarity::Unique);
        let name = match (&item.base, &item.name) {
            (Some(base), _) if !rarity.names_base_in_name() => pob_item::base_name(base).to_owned(),
            (_, Some(name)) => {
                match pob_item::magic_base(name, self.scopes.keys().map(String::as_str)) {
                    BaseMatch::One(base) => base,
                    BaseMatch::None => return Err("no-base-in-name".to_owned()),
                    BaseMatch::Tie(_) => return Err("tie".to_owned()),
                }
            }
            _ => return Err("no-name".to_owned()),
        };
        let stripped = POB_SUFFIXED_BASES
            .contains(&name.as_str())
            .then(|| name.rsplit_once(" (").map(|(n, _)| n))
            .flatten();
        let (key, suffix_removed) = match (self.scopes.contains_key(&name), stripped) {
            (true, _) => (name.as_str(), false),
            (false, Some(n)) if self.scopes.contains_key(n) => (n, true),
            _ => return Err("not-in-base-item-types".to_owned()),
        };
        let scopes: Option<Vec<&'static str>> =
            self.scopes.get(key).map(|s| s.iter().copied().collect());
        match scopes.as_deref() {
            Some([one]) => Ok(Found {
                scope: one,
                suffix_removed,
                rows: self.rows.get(key).map_or(&[], Vec::as_slice),
            }),
            _ => Err("scope-conflict".to_owned()),
        }
    }
}

/// An item's base as [`Bases::find`] found it.
struct Found<'b> {
    scope: &'static str,
    suffix_removed: bool,
    rows: &'b [BaseRow],
}

/// One build file: its content hash and text.
pub struct Build {
    /// SHA-256 of the file's bytes, hex.
    pub hash: String,
    /// The file's text, `None` when it is not UTF-8.
    pub text: Option<String>,
}

impl Build {
    /// Hashes and decodes a file's bytes.
    #[must_use]
    pub fn new(bytes: Vec<u8>) -> Self {
        let hash = hex(&Sha256::digest(&bytes));
        Self {
            hash,
            text: String::from_utf8(bytes).ok(),
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A spec or prefix key as written if it is on the reader's fixed lists
/// (`KNOWN_SPECS`, line flags, value prefixes); otherwise a hash, since
/// author text (`My note: ...`, `{anything}`) can reach these maps.
fn safe_key(key: &str) -> String {
    if pob_item::KNOWN_SPECS.contains(&key) || pob_item::is_known_prefix(key) {
        key.to_owned()
    } else {
        scrub(key)
    }
}

/// A short hash of text that must not be written (names, custom lines).
fn scrub(text: &str) -> String {
    format!(
        "sha256:{}",
        hex(&Sha256::digest(text.as_bytes()))
            .get(..16)
            .unwrap_or("")
    )
}

/// The tool's commit, for the summary.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Tool {
    /// `git rev-parse HEAD`.
    pub commit: Option<String>,
    /// Whether `git status --porcelain` printed anything.
    pub dirty: Option<bool>,
}

/// Recorded inputs.
#[derive(Debug, Default, Serialize)]
pub struct RunInputs {
    /// The extract.
    pub extract: Inputs,
    /// The tool.
    pub tool: Tool,
    /// Content hash of every build, sorted.
    pub builds: Vec<String>,
    /// PoB text sources for `data-gap/legacy-text` (B4b).
    pub pob: Vec<categorize::PobSource>,
}

/// Counts of one group of lines.
#[derive(Debug, Default, Serialize)]
pub struct LineCounts {
    /// Every line.
    pub lines: usize,
    /// Outcome to lines.
    pub outcomes: BTreeMap<String, usize>,
    /// Section to outcome to lines (modifier lines only).
    pub by_section: BTreeMap<&'static str, BTreeMap<String, usize>>,
    /// Lines whose resolution lists shadowed matches.
    pub with_shadowed_matches: usize,
}

impl LineCounts {
    fn add(&mut self, record: &LineRecord, shadowed: bool) {
        self.lines += 1;
        *self.outcomes.entry(record.outcome.clone()).or_default() += 1;
        if let Some(section) = record.section {
            *self
                .by_section
                .entry(section)
                .or_default()
                .entry(record.outcome.clone())
                .or_default() += 1;
        }
        if shadowed {
            self.with_shadowed_matches += 1;
        }
    }
}

/// The first resolution rate (provisional; the gate is B4b's).
#[derive(Debug, Default, Serialize)]
pub struct Rate {
    /// Non-unique lines that are not excluded.
    pub denominator: usize,
    /// Of those, resolved to one description.
    pub resolved: usize,
    /// `resolved / denominator`.
    pub rate: f64,
    /// Non-unique excluded lines.
    pub excluded: usize,
    /// `excluded / (denominator + excluded)`.
    pub excluded_share: f64,
    /// Distinct (scope, text) of the denominator's lines.
    pub distinct: usize,
    /// Of those, resolved.
    pub distinct_resolved: usize,
    /// `distinct_resolved / distinct`.
    pub distinct_rate: f64,
    /// Distinct texts whose occurrences had different outcomes.
    pub distinct_with_mixed_outcomes: usize,
    /// `rate >= 0.98`.
    pub meets_rate: bool,
}

/// One distinct text with a given outcome.
#[derive(Debug, Default, Serialize)]
pub struct Distinct {
    /// Occurrences in non-unique items.
    pub non_unique: usize,
    /// Occurrences in unique items.
    pub unique: usize,
    /// Candidates (ambiguous) or rejection reasons (unresolved).
    pub detail: BTreeSet<String>,
    /// Why the line is still ambiguous or not mod-backed (B4a).
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub reasons: BTreeSet<String>,
}

/// The `Mods` filter's decisions (B4a), by group.
#[derive(Debug, Default, Serialize)]
pub struct FilterCounts {
    /// Decision (`resolved/<steps>`, `ambiguous/<reason>`) to non-unique
    /// lines.
    pub non_unique: BTreeMap<String, usize>,
    /// Decision to unique lines.
    pub unique: BTreeMap<String, usize>,
    /// `<group>/<note>` to lines.
    pub notes: BTreeMap<String, usize>,
    /// Texts the filter resolved: detail is `<steps>: <candidate>`.
    pub resolved: BTreeMap<String, Distinct>,
}

/// `mod-backed` over resolved non-unique lines (B4a; reported, not gated).
#[derive(Debug, Default, Serialize)]
pub struct ModBacked {
    /// Resolved non-unique lines with a known base.
    pub resolved: usize,
    /// Of those, backed by a fitting row with overlapping values.
    pub backed: usize,
    /// `backed / resolved`.
    pub share: f64,
    /// Of `resolved`, lines the resolver resolved alone (not through the
    /// filter): the independent check, since a line the filter resolved was
    /// chosen because a fitting row backs it.
    pub template_resolved: usize,
    /// Of those, backed.
    pub template_backed: usize,
    /// `template_backed / template_resolved`.
    pub template_share: f64,
    /// Backed lines with a row whose stats equal the candidate's.
    pub equal: usize,
    /// Backed lines with hybrid rows only.
    pub hybrid: usize,
    /// Reason to lines not backed.
    pub not_backed: BTreeMap<String, usize>,
    /// Texts not backed (game text: a template matched), with stats and
    /// reasons.
    pub not_backed_texts: BTreeMap<String, Distinct>,
}

/// `summary.json`.
#[derive(Debug, Default, Serialize)]
pub struct Summary {
    /// Recorded inputs.
    pub inputs: RunInputs,
    /// Lookup tables.
    pub lookup: LookupReport,
    /// Scope to its index counts.
    pub scopes: BTreeMap<&'static str, IndexFields>,
    /// Builds that are not UTF-8 or not XML, by hash, with the error.
    pub unreadable_builds: BTreeMap<String, String>,
    /// `<Item>` elements.
    pub items: usize,
    /// Rarity name (`none` without a `Rarity:` line) to items.
    pub items_by_rarity: BTreeMap<&'static str, usize>,
    /// Items with `<ModRange>` children, and the children.
    pub mod_range_items: usize,
    /// `<ModRange>` elements.
    pub mod_range_elements: usize,
    /// Items with a `Catalyst:` spec.
    pub catalyst_items: usize,
    /// Scope or `unknown-base/<reason>` to items.
    pub items_by_scope: BTreeMap<String, usize>,
    /// Line kind to lines, over all items (structure included).
    pub line_kinds: BTreeMap<&'static str, usize>,
    /// Spec keys `ParseRaw` does not know, with counts.
    pub unknown_spec_keys: BTreeMap<String, usize>,
    /// Modifier prefix keys to lines.
    pub prefix_keys: BTreeMap<String, usize>,
    /// Prefix keys that are neither line flags nor value prefixes.
    pub unknown_prefix_keys: BTreeMap<String, usize>,
    /// Item lines of normal, magic, and rare items.
    pub non_unique: LineCounts,
    /// Item lines of unique and relic items (and items without a rarity,
    /// which PoB reads as unique).
    pub unique: LineCounts,
    /// The first rate.
    pub rate: Rate,
    /// Ambiguous texts: candidates as `stat,stat` lists.
    pub ambiguous: BTreeMap<String, Distinct>,
    /// Group (`non_unique`, `unique`) to category to lines, over every line
    /// that is not resolved (B4b; `categorize`). Excluded lines carry their
    /// rule as the category.
    pub categories: BTreeMap<&'static str, BTreeMap<String, usize>>,
    /// Category to text to counts, with the evidence in `detail`. Text is
    /// written only when game or PoB text matched it (every category but
    /// `special-case/author-note`, `special-case/pob-custom`,
    /// `special-case/unknown-base`, and `uncategorized`, whose text is a
    /// hash: it may be the author's own words).
    pub categorized: BTreeMap<String, BTreeMap<String, Distinct>>,
    /// Whether any stat description or `ClientStrings` text holds a PoB
    /// colour escape (the `color-code` rule needs `false`).
    pub colour_code_in_game_text: bool,
    /// The gate (plan, "Gate").
    pub gate: Gate,
    /// Items whose base was found only after removing the suffix of one of
    /// `POB_SUFFIXED_BASES`.
    pub pob_base_suffix_removed: usize,
    /// Build files whose content equals an earlier file's (counted once).
    pub duplicate_builds: usize,
    /// Items with more than one non-blank text child (the last is read).
    pub items_with_several_texts: usize,
    /// `ParseRaw` features the reader does not reproduce, to lines showing
    /// them (`pob_item::unsupported_features`); must be empty for the
    /// reader's rules to cover the corpus.
    pub unsupported_features: BTreeMap<&'static str, usize>,
    /// Class-requirement lines (`Requires Class ...`; excluded as
    /// `not-a-modifier`, the class kept on the item record).
    pub class_requirement_lines: usize,
    /// `Mods` rows read.
    pub mods_rows: usize,
    /// `Mods` rows whose spawn tag and weight arrays differ in length.
    pub mods_bad_spawn_rows: usize,
    /// Essence-only `Mods` rows whose item classes `Essences` gives.
    pub essence_rows_known: usize,
    /// Essence-only `Mods` rows with no known classes (no `Essences` table,
    /// or no read column names them: legacy essence mods).
    pub essence_rows_unknown: usize,
    /// Named `BaseItemTypes` rows whose tags are unknown.
    pub bases_with_unknown_tags: usize,
    /// The `Mods` filter on ambiguous lines.
    pub filter: FilterCounts,
    /// `mod-backed` of resolved non-unique lines.
    pub mod_backed: ModBacked,
}

/// Categories whose text is not written to the summary (it may be the
/// author's own words); see [`Summary::categorized`].
const HASHED_CATEGORIES: [&str; 4] = [
    "special-case/author-note",
    "special-case/pob-custom",
    "special-case/unknown-base",
    "uncategorized",
];

/// The Spike B gate (plan, "Gate"), computed from this run. Determinism
/// (item 3) is checked by comparing two runs' files, outside the run.
#[derive(Debug, Default, Serialize)]
pub struct Gate {
    /// Item 1: `rate.rate >= 0.98`.
    pub resolution: bool,
    /// Unresolved non-unique lines the rate allows: `floor(0.02 *
    /// denominator)`.
    pub unresolved_allowed: usize,
    /// Unresolved non-unique lines.
    pub unresolved: usize,
    /// Item 2: lines (unique included) whose category is `uncategorized`.
    pub uncategorized: usize,
    /// Item 4: the extract manifest hash, the Steam build, a hash per
    /// build, the tool commit, a clean tool tree, and a PoB source are
    /// recorded.
    pub inputs_recorded: bool,
    /// Items 1, 2, and 4 hold.
    pub met: bool,
}

/// [`IndexStats`] as JSON.
#[derive(Debug, Default, Serialize)]
pub struct IndexFields {
    indexed: usize,
    multi_line: usize,
    lookup: usize,
    keys: usize,
    max_lines: usize,
    gem_quality: usize,
    table_only: usize,
    bad_template: usize,
    empty_text: usize,
    lookup_unavailable: usize,
    lookup_multi: usize,
}

impl From<IndexStats> for IndexFields {
    fn from(s: IndexStats) -> Self {
        Self {
            indexed: s.indexed,
            multi_line: s.multi_line,
            lookup: s.lookup,
            keys: s.keys,
            max_lines: s.max_lines,
            gem_quality: s.gem_quality,
            table_only: s.table_only,
            bad_template: s.bad_template,
            empty_text: s.empty_text,
            lookup_unavailable: s.lookup_unavailable,
            lookup_multi: s.lookup_multi,
        }
    }
}

/// One item line in `report.json`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct LineRecord {
    /// Index of the line among the item's non-blank lines.
    pub line: usize,
    /// Modifier section, if a modifier.
    pub section: Option<&'static str>,
    /// Outcome.
    pub outcome: String,
    /// Text (resolution text for resolved lines; scrubbed where needed).
    pub text: String,
    /// Input lines the resolution covers (multi-line templates).
    pub joined: usize,
    /// Resolved stats (`stat=value`), or candidates (`stat,stat`), or
    /// rejection reasons.
    pub detail: Vec<String>,
    /// The `Mods` filter's decision on an ambiguous line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    /// The filter's notes.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub filter_notes: Vec<&'static str>,
    /// Candidates the filter removed.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dropped: Vec<String>,
    /// `mod-backed` of a resolved non-unique line: `equal`, `hybrid`, or
    /// `not/<reason>`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backed: Option<String>,
    /// The backing row's `Mods.Id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backed_by: Option<String>,
    /// An unresolved line that resolves together with this adjacent line of
    /// another section (index among the item's lines).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub split_with: Option<usize>,
    /// Category of a line that is not resolved (B4b).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// The category's evidence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

/// One item in `report.json`.
#[derive(Debug, Serialize)]
pub struct ItemRecord {
    /// Position of the `<Item>` in the build.
    pub item: usize,
    /// Rarity name.
    pub rarity: &'static str,
    /// Scope or `unknown-base/<reason>`.
    pub scope: String,
    /// Classes of `Requires Class <class>` lines, kept on the item (the
    /// lines are excluded as `not-a-modifier`; maintainer decision
    /// 2026-11-09).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub class_requirement: Vec<String>,
    /// Item lines (structure lines are not listed).
    pub lines: Vec<LineRecord>,
}

/// `report.json`: every item line, by build hash.
#[derive(Debug, Default, Serialize)]
pub struct Report {
    /// Build hash to items.
    pub builds: BTreeMap<String, Vec<ItemRecord>>,
}

fn value_text(value: &RawValue) -> String {
    let v = |s: &loom_data::stat_desc::StatValue| {
        if s.min == s.max {
            s.min.to_string()
        } else {
            format!("{}..{}", s.min, s.max)
        }
    };
    match value {
        RawValue::Exact(s) => v(s),
        RawValue::Ranged(s) => format!("({})", v(s)),
        RawValue::Hidden { assumed, .. } => {
            format!(
                "hidden:{}",
                assumed.map_or("?".to_owned(), |a| a.to_string())
            )
        }
        RawValue::Lookup(keys) => {
            let keys: Vec<String> = keys.iter().map(i64::to_string).collect();
            format!("keys:{}", keys.join("|"))
        }
    }
}

fn candidate_text(c: &Candidate) -> String {
    let stats: Vec<&str> = c.stats.iter().map(|s| s.stat.as_str()).collect();
    let hand = if c.at.hand == Hand::Off {
        " (off hand)"
    } else {
        ""
    };
    format!("{}{hand}", stats.join(","))
}

fn outcome_name(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Resolved(_) => "resolved".to_owned(),
        Outcome::Ambiguous(_) => "ambiguous".to_owned(),
        Outcome::Unresolved(u) => format!("unresolved/{}", u.name()),
    }
}

fn detail(outcome: &Outcome) -> Vec<String> {
    match outcome {
        Outcome::Resolved(c) => c
            .stats
            .iter()
            .map(|s| format!("{}={}", s.stat, value_text(&s.value)))
            .collect(),
        Outcome::Ambiguous(cs) => cs.iter().map(candidate_text).collect(),
        Outcome::Unresolved(Unresolved::NoVariantFits(rejected)) => rejected
            .iter()
            .map(|r| r.reason.name().to_owned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        Outcome::Unresolved(_) => Vec::new(),
    }
}

/// Item lines of one item, before counting.
struct Lines<'i, 'c> {
    records: Vec<(LineRecord, bool)>,
    run: Vec<(usize, &'i Modifier)>,
    ctx: Option<&'c Context<'c>>,
}

impl<'i> Lines<'i, '_> {
    fn flush(&mut self, index: Option<&Index<'_>>, scope_err: Option<&str>) {
        if self.run.is_empty() {
            return;
        }
        let run = std::mem::take(&mut self.run);
        let section = |i: usize| run.get(i).map(|(_, m)| m.section.name());
        match (index, scope_err) {
            (Some(index), None) => {
                let texts: Vec<&str> = run.iter().map(|(_, m)| m.text.as_str()).collect();
                let mut at = 0;
                for resolution in index.resolve_lines(&texts) {
                    self.push_resolution(&run, at, &resolution, section(at));
                    at += resolution.lines;
                }
                // `resolve_lines` covers every input line; if it ever does
                // not, the uncovered lines are recorded, never lost.
                for (n, (line, m)) in run.iter().enumerate().skip(at) {
                    self.records.push((
                        LineRecord {
                            line: *line,
                            section: section(n),
                            outcome: "internal/not-covered".to_owned(),
                            text: m.text.clone(),
                            joined: 1,
                            detail: Vec::new(),
                            ..LineRecord::default()
                        },
                        false,
                    ));
                }
            }
            _ => {
                let reason = scope_err.unwrap_or("no-index");
                for (n, (line, m)) in run.iter().enumerate() {
                    self.records.push((
                        LineRecord {
                            line: *line,
                            section: section(n),
                            outcome: format!("unknown-base/{reason}"),
                            text: m.text.clone(),
                            joined: 1,
                            detail: Vec::new(),
                            ..LineRecord::default()
                        },
                        false,
                    ));
                }
            }
        }
    }

    fn push_resolution(
        &mut self,
        run: &[(usize, &Modifier)],
        at: usize,
        resolution: &Resolution,
        section: Option<&'static str>,
    ) {
        let mut record = LineRecord {
            section,
            outcome: outcome_name(&resolution.outcome),
            text: resolution.line.clone(),
            joined: resolution.lines,
            detail: detail(&resolution.outcome),
            ..LineRecord::default()
        };
        let line = run.get(at).map(|(_, m)| LineContext::of(m));
        let mut resolved = match &resolution.outcome {
            Outcome::Resolved(c) => Some(c),
            _ => None,
        };
        if let (Some(ctx), Some(line), Outcome::Ambiguous(cs)) =
            (self.ctx, line, &resolution.outcome)
        {
            let d = mod_filter::filter(ctx, line, cs);
            record.filter = Some(d.name());
            record.filter_notes = d.notes.iter().copied().collect();
            record.dropped = cs
                .iter()
                .enumerate()
                .filter(|(i, _)| !d.kept.contains(i))
                .map(|(_, c)| candidate_text(c))
                .collect();
            if let [one] = d.kept.as_slice() {
                resolved = cs.get(*one);
                if let Some(c) = resolved {
                    record.outcome = "resolved".to_owned();
                    record.detail = detail(&Outcome::Resolved(c.clone()));
                }
            } else {
                record.detail = d
                    .kept
                    .iter()
                    .filter_map(|i| cs.get(*i))
                    .map(candidate_text)
                    .collect();
            }
        }
        if let (Some(ctx), Some(line), Some(c)) = (self.ctx, line, resolved)
            && !ctx.unique
        {
            match mod_filter::backing(ctx, line, &c.stats) {
                Ok((equal, id)) => {
                    record.backed = Some(if equal { "equal" } else { "hybrid" }.to_owned());
                    record.backed_by = Some(id.to_owned());
                }
                Err(reason) => record.backed = Some(format!("not/{reason}")),
            }
        }
        for k in 0..resolution.lines {
            if let Some((n, _)) = run.get(at + k) {
                self.records.push((
                    LineRecord {
                        line: *n,
                        ..record.clone()
                    },
                    !resolution.shadowed.is_empty(),
                ));
            }
        }
    }
}

fn excluded(
    line: usize,
    section: Option<&'static str>,
    rule: &str,
    text: String,
) -> (LineRecord, bool) {
    (
        LineRecord {
            line,
            section,
            outcome: format!("excluded/{rule}"),
            text,
            joined: 1,
            detail: Vec::new(),
            ..LineRecord::default()
        },
        false,
    )
}

/// Resolves one item's lines.
fn item_lines(
    item: &Item,
    index: Option<&Index<'_>>,
    scope_err: Option<&str>,
    ctx: Option<&Context<'_>>,
) -> Vec<(LineRecord, bool)> {
    let mut lines = Lines {
        records: Vec::new(),
        run: Vec::new(),
        ctx,
    };
    for (n, line) in item.lines.iter().enumerate() {
        match &line.kind {
            LineKind::Modifier(m) if resolvable(m) => {
                if lines
                    .run
                    .last()
                    .is_some_and(|(_, prev)| prev.section != m.section)
                {
                    lines.flush(index, scope_err);
                }
                lines.run.push((n, m));
                continue;
            }
            _ => lines.flush(index, scope_err),
        }
        let record = match &line.kind {
            LineKind::Modifier(m) if !m.selected => excluded(
                n,
                Some(m.section.name()),
                "not-selected-variant",
                m.text.clone(),
            ),
            LineKind::Modifier(m) if m.has("custom") => {
                excluded(n, Some(m.section.name()), "pob-custom", scrub(&m.text))
            }
            // Class requirement (plan step 4b, maintainer decision
            // 2026-11-09): `v2.67.2:src/Classes/Item.lua` keeps these lines
            // in `classRequirementModLines` and sets `classRestriction`;
            // they are not modifiers. The class is kept on the item record.
            LineKind::Modifier(m) => {
                excluded(n, Some(m.section.name()), "not-a-modifier", m.text.clone())
            }
            LineKind::Flag | LineKind::Influence => {
                excluded(n, None, "not-a-modifier", line.raw.clone())
            }
            LineKind::Unrecognized => (
                LineRecord {
                    line: n,
                    section: None,
                    outcome: "unrecognized-line".to_owned(),
                    text: scrub(&line.raw),
                    joined: 1,
                    detail: Vec::new(),
                    ..LineRecord::default()
                },
                false,
            ),
            _ => continue,
        };
        lines.records.push(record);
    }
    lines.flush(index, scope_err);
    lines.records.sort_by_key(|(r, _)| r.line);
    if let Some(index) = index {
        mark_split_lines(item, index, &mut lines.records);
    }
    lines.records
}

/// A modifier line the harness resolves: selected, not `{custom}`, not a
/// class requirement.
fn resolvable(m: &Modifier) -> bool {
    m.selected && !m.has("custom") && m.section != Section::ClassRequirement
}

/// Sets `split_with` on a `no-template` line when it and the adjacent
/// resolvable line of another section resolve as one multi-line
/// description (`parser-gap/split-section`): PoB's `Implicits:` count can
/// put the two lines of one description in two sections, and the harness
/// resolves sections apart.
fn mark_split_lines(item: &Item, index: &Index<'_>, records: &mut [(LineRecord, bool)]) {
    let modifier = |n: usize| match item.lines.get(n).map(|l| &l.kind) {
        Some(LineKind::Modifier(m)) if resolvable(m) => Some(m),
        _ => None,
    };
    for (record, _) in records.iter_mut() {
        if record.outcome != "unresolved/no-template" {
            continue;
        }
        let n = record.line;
        let Some(this) = modifier(n) else {
            continue;
        };
        let neighbours = [n.checked_sub(1), n.checked_add(1)];
        for other in neighbours.into_iter().flatten() {
            let Some(m) = modifier(other).filter(|m| m.section != this.section) else {
                continue;
            };
            let pair = if other < n {
                [m.text.as_str(), this.text.as_str()]
            } else {
                [this.text.as_str(), m.text.as_str()]
            };
            let joined = index
                .resolve_lines(&pair)
                .first()
                .is_some_and(|r| r.lines == 2 && matches!(r.outcome, Outcome::Resolved(_)));
            if joined {
                record.split_with = Some(other);
                break;
            }
        }
    }
}

/// Runs the corpus over a loaded extract.
///
/// # Errors
/// A message when the extract is not PoE1 or a scope does not build.
#[expect(
    clippy::too_many_arguments,
    reason = "the harness's loaded inputs, each built by main; a struct would only rename them"
)]
pub fn run(
    extract: &Extract,
    lookup: &TableLookup,
    lookup_report: LookupReport,
    bases: &Bases,
    mods: &Mods,
    game: &GameText,
    builds: &[Build],
    tool: Tool,
) -> Result<(Summary, Report), String> {
    if extract.inputs.game != "poe1" {
        return Err(format!(
            "resolve-corpus reads PoE1 builds; the extract is {}",
            extract.inputs.game
        ));
    }
    let mut scopes: BTreeMap<&'static str, Scope<'_>> = BTreeMap::new();
    for name in SCOPES {
        let file = format!("{name}.txt");
        let path = extract
            .files
            .iter()
            .find(|p| {
                p.rsplit('/')
                    .next()
                    .is_some_and(|f| f.eq_ignore_ascii_case(&file))
            })
            .ok_or_else(|| format!("{file}: not in the extract"))?;
        let scope = extract
            .set
            .scope(path)
            .map_err(|e| format!("{path}: {} ({e})", e.code()))?;
        scopes.insert(name, scope);
    }
    let indexes: BTreeMap<&'static str, Index<'_>> = scopes
        .iter()
        .map(|(name, scope)| (*name, Index::build_with(scope, lookup)))
        .collect();
    let chains: BTreeMap<&'static str, BTreeSet<String>> = scopes
        .iter()
        .map(|(name, scope)| {
            (
                *name,
                scope.chain().iter().map(|f| f.source.clone()).collect(),
            )
        })
        .collect();
    let corpus = Corpus {
        bases,
        mods,
        indexes: &indexes,
        chains: &chains,
        game,
    };

    let (essence_rows_known, essence_rows_unknown) = mods.essence_rows();
    let mut s = Summary {
        lookup: lookup_report,
        scopes: indexes
            .iter()
            .map(|(n, i)| (*n, i.stats().into()))
            .collect(),
        mods_rows: mods.row_count(),
        mods_bad_spawn_rows: mods.bad_spawn_rows,
        essence_rows_known,
        essence_rows_unknown,
        bases_with_unknown_tags: bases.tags_unknown,
        colour_code_in_game_text: game.colour_code_in_game_text,
        ..Summary::default()
    };
    let mut report = Report::default();
    // One build per content hash: a copied file counts once.
    let mut unique_builds: BTreeMap<&str, &Build> = BTreeMap::new();
    for build in builds {
        if unique_builds.insert(&build.hash, build).is_some() {
            s.duplicate_builds += 1;
        }
    }
    let mut distinct: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();

    for build in unique_builds.into_values() {
        s.inputs.builds.push(build.hash.clone());
        let Some(text) = &build.text else {
            s.unreadable_builds
                .insert(build.hash.clone(), "not UTF-8".to_owned());
            continue;
        };
        let doc = match roxmltree::Document::parse(text) {
            Ok(doc) => doc,
            Err(e) => {
                s.unreadable_builds
                    .insert(build.hash.clone(), e.to_string());
                continue;
            }
        };
        let mut items = Vec::new();
        for (n, node) in doc
            .descendants()
            .filter(|n| n.has_tag_name("Item"))
            .enumerate()
        {
            let texts: Vec<&str> = node
                .children()
                .filter(roxmltree::Node::is_text)
                .filter_map(|t| t.text())
                .filter(|t| !t.trim().is_empty())
                .collect();
            if texts.len() > 1 {
                s.items_with_several_texts += 1;
            }
            let text = texts.last().copied().unwrap_or("");
            let ranges = node
                .children()
                .filter(|c| c.has_tag_name("ModRange"))
                .count();
            let item = pob_item::read(text);
            let record = count_item(&mut s, &mut distinct, (n, &item, ranges), &corpus);
            items.push(record);
        }
        report.builds.insert(build.hash.clone(), items);
    }
    s.inputs.tool = tool;
    s.inputs.extract = extract.inputs.clone();
    s.inputs.pob = game.pob_sources();
    finish_rate(&mut s, &distinct);
    Ok((s, report))
}

/// What every item of the run is resolved and categorized with.
struct Corpus<'c, 'i> {
    bases: &'c Bases,
    mods: &'c Mods,
    indexes: &'c BTreeMap<&'static str, Index<'i>>,
    /// Scope name to the sources of its chain's files.
    chains: &'c BTreeMap<&'static str, BTreeSet<String>>,
    game: &'c GameText,
}

/// The category of a line that is not resolved, from its outcome; excluded
/// categories turn the outcome into `excluded/<rule>`.
fn categorize_line(record: &mut LineRecord, scope: &BTreeSet<String>, game: &GameText) {
    let base_rule = match record.outcome.as_str() {
        "resolved" => return,
        "excluded/pob-custom" => Some("special-case/pob-custom"),
        "excluded/not-a-modifier" => Some("special-case/not-a-modifier"),
        "excluded/not-selected-variant" => Some("excluded/not-selected-variant"),
        _ => None,
    };
    if let Some(rule) = base_rule {
        record.category = Some(rule.to_owned());
        return;
    }
    let mut detail = record.detail.clone();
    if record.outcome == "ambiguous" {
        detail = record.filter.iter().cloned().collect();
        detail.extend(record.filter_notes.iter().map(|n| (*n).to_owned()));
    }
    let c = categorize::categorize(
        game,
        LineFacts {
            outcome: &record.outcome,
            text: &record.text,
            detail: &detail,
            scope,
            split_with: record.split_with,
        },
    );
    if c.excluded() {
        let rule = c.name.rsplit('/').next().unwrap_or(c.name);
        record.outcome = format!("excluded/{rule}");
    }
    record.category = Some(c.name.to_owned());
    record.evidence = Some(c.evidence);
}

fn count_item(
    s: &mut Summary,
    distinct: &mut BTreeMap<(String, String), BTreeSet<String>>,
    (n, item, ranges): (usize, &Item, usize),
    corpus: &Corpus<'_, '_>,
) -> ItemRecord {
    let (bases, mods, indexes) = (corpus.bases, corpus.mods, corpus.indexes);
    s.items += 1;
    let rarity = item.rarity.map_or("none", Rarity::name);
    *s.items_by_rarity.entry(rarity).or_default() += 1;
    if ranges > 0 {
        s.mod_range_items += 1;
        s.mod_range_elements += ranges;
    }
    for (feature, n) in pob_item::unsupported_features(item) {
        *s.unsupported_features.entry(feature).or_default() += n;
    }
    let mut catalyst = false;
    for line in &item.lines {
        *s.line_kinds.entry(line.kind.name()).or_default() += 1;
        match &line.kind {
            LineKind::Spec { key, known } => {
                catalyst |= key == "Catalyst";
                if !known {
                    *s.unknown_spec_keys.entry(safe_key(key)).or_default() += 1;
                }
            }
            LineKind::Modifier(m) => {
                for p in &m.prefixes {
                    *s.prefix_keys.entry(safe_key(&p.key)).or_default() += 1;
                }
                for k in m.unknown_prefixes() {
                    *s.unknown_prefix_keys.entry(safe_key(k)).or_default() += 1;
                }
            }
            _ => {}
        }
    }
    if catalyst {
        s.catalyst_items += 1;
    }
    let found = bases.find(item);
    let scope_name = match &found {
        Ok(f) => {
            if f.suffix_removed {
                s.pob_base_suffix_removed += 1;
            }
            f.scope.to_owned()
        }
        Err(reason) => format!("unknown-base/{reason}"),
    };
    *s.items_by_scope.entry(scope_name.clone()).or_default() += 1;
    let unique = item.rarity.is_none_or(Rarity::is_unique);
    let index = found.as_ref().ok().and_then(|f| indexes.get(f.scope));
    let ctx = found.as_ref().ok().map(|f| Context {
        mods,
        bases: f.rows,
        unique,
        influences: item
            .lines
            .iter()
            .filter(|l| l.kind == LineKind::Influence)
            .filter_map(|l| mod_filter::influence_type(&l.raw))
            .collect(),
    });
    let mut records = item_lines(
        item,
        index,
        found.as_ref().err().map(String::as_str),
        ctx.as_ref(),
    );
    let empty = BTreeSet::new();
    let chain = found
        .as_ref()
        .ok()
        .and_then(|f| corpus.chains.get(f.scope))
        .unwrap_or(&empty);
    for (record, _) in &mut records {
        categorize_line(record, chain, corpus.game);
    }
    let class_requirement: Vec<String> = item
        .modifiers()
        .filter(|m| m.section == Section::ClassRequirement)
        .map(|m| {
            m.text
                .split_once("Requires Class ")
                .map_or(m.text.as_str(), |(_, c)| c)
                .trim()
                .to_owned()
        })
        .collect();
    s.class_requirement_lines += class_requirement.len();
    let group = if unique { "unique" } else { "non_unique" };
    for (record, shadowed) in &records {
        let counts = if unique {
            &mut s.unique
        } else {
            &mut s.non_unique
        };
        counts.add(record, *shadowed);
        let excluded = record.outcome.starts_with("excluded/");
        if !unique && !excluded {
            distinct
                .entry((scope_name.clone(), record.text.clone()))
                .or_default()
                .insert(record.outcome.clone());
        }
        count_b4a(s, record, unique);
        if record.outcome == "ambiguous" {
            let entry = s.ambiguous.entry(record.text.clone()).or_default();
            if unique {
                entry.unique += 1;
            } else {
                entry.non_unique += 1;
            }
            entry.detail.extend(record.detail.iter().cloned());
            entry.reasons.extend(record.filter.iter().cloned());
            entry
                .reasons
                .extend(record.filter_notes.iter().map(|n| (*n).to_owned()));
        }
        count_category(s, record, group);
    }
    ItemRecord {
        item: n,
        rarity,
        scope: scope_name,
        class_requirement,
        lines: records.into_iter().map(|(r, _)| r).collect(),
    }
}

/// Counts the category of a line that is not resolved (B4b).
fn count_category(s: &mut Summary, record: &LineRecord, group: &'static str) {
    let Some(category) = &record.category else {
        return;
    };
    *s.categories
        .entry(group)
        .or_default()
        .entry(category.clone())
        .or_default() += 1;
    if category == "excluded/not-selected-variant" || category == "special-case/not-a-modifier" {
        // Counted only: these lines are many and are listed in the report.
        return;
    }
    let text = if HASHED_CATEGORIES.contains(&category.as_str()) {
        scrub(&record.text)
    } else {
        record.text.clone()
    };
    let entry = s
        .categorized
        .entry(category.clone())
        .or_default()
        .entry(text)
        .or_default();
    if group == "unique" {
        entry.unique += 1;
    } else {
        entry.non_unique += 1;
    }
    entry.detail.extend(record.evidence.iter().cloned());
}

/// Counts the filter's decision and `mod-backed` of one line (B4a).
fn count_b4a(s: &mut Summary, record: &LineRecord, unique: bool) {
    let group = if unique { "unique" } else { "non_unique" };
    if let Some(decision) = &record.filter {
        let counts = if unique {
            &mut s.filter.unique
        } else {
            &mut s.filter.non_unique
        };
        *counts.entry(decision.clone()).or_default() += 1;
        for note in &record.filter_notes {
            *s.filter.notes.entry(format!("{group}/{note}")).or_default() += 1;
        }
        if let Some(steps) = decision.strip_prefix("resolved/") {
            let entry = s.filter.resolved.entry(record.text.clone()).or_default();
            if unique {
                entry.unique += 1;
            } else {
                entry.non_unique += 1;
            }
            entry
                .detail
                .extend(record.detail.iter().map(|d| format!("{steps}: {d}")));
        }
    }
    let Some(backed) = &record.backed else {
        return;
    };
    let m = &mut s.mod_backed;
    let template = record.filter.is_none();
    m.resolved += 1;
    m.template_resolved += usize::from(template);
    match backed.as_str() {
        "equal" | "hybrid" => {
            m.backed += 1;
            m.template_backed += usize::from(template);
            if backed == "equal" {
                m.equal += 1;
            } else {
                m.hybrid += 1;
            }
        }
        other => {
            let reason = other.strip_prefix("not/").unwrap_or(other).to_owned();
            *m.not_backed.entry(reason.clone()).or_default() += 1;
            let entry = m.not_backed_texts.entry(record.text.clone()).or_default();
            entry.non_unique += 1;
            entry.detail.extend(record.detail.iter().cloned());
            entry.reasons.insert(reason);
        }
    }
}

#[expect(clippy::cast_precision_loss, reason = "line counts are far below 2^52")]
fn ratio(a: usize, b: usize) -> f64 {
    if b == 0 { 0.0 } else { a as f64 / b as f64 }
}

fn finish_rate(s: &mut Summary, distinct: &BTreeMap<(String, String), BTreeSet<String>>) {
    let r = &mut s.rate;
    for (outcome, n) in &s.non_unique.outcomes {
        if outcome.starts_with("excluded/") {
            r.excluded += n;
        } else {
            r.denominator += n;
            if outcome == "resolved" {
                r.resolved += n;
            }
        }
    }
    r.rate = ratio(r.resolved, r.denominator);
    r.excluded_share = ratio(r.excluded, r.denominator + r.excluded);
    r.distinct = distinct.len();
    r.distinct_resolved = distinct
        .values()
        .filter(|o| o.len() == 1 && o.contains("resolved"))
        .count();
    r.distinct_with_mixed_outcomes = distinct.values().filter(|o| o.len() > 1).count();
    r.distinct_rate = ratio(r.distinct_resolved, r.distinct);
    // Integer form of `rate >= 0.98`, the same test as the gate's
    // allowance: unresolved * 100 <= 2 * denominator.
    r.meets_rate = r.denominator > 0 && (r.denominator - r.resolved) * 100 <= 2 * r.denominator;
    let m = &mut s.mod_backed;
    m.share = ratio(m.backed, m.resolved);
    m.template_share = ratio(m.template_backed, m.template_resolved);
    let i = &s.inputs;
    let g = &mut s.gate;
    g.resolution = s.rate.meets_rate;
    g.unresolved = s.rate.denominator - s.rate.resolved;
    // 2% of the denominator, rounded down: the integer form of the rate.
    g.unresolved_allowed = s.rate.denominator * 2 / 100;
    g.uncategorized = s
        .categories
        .values()
        .filter_map(|c| c.get("uncategorized"))
        .sum();
    g.inputs_recorded = !i.extract.manifest_sha256.is_empty()
        && i.extract.steam_build_id.is_some()
        && !i.builds.is_empty()
        && i.tool.commit.is_some()
        && i.tool.dirty == Some(false)
        && !i.pob.is_empty();
    g.met = g.resolution && g.uncategorized == 0 && g.inputs_recorded;
}

#[cfg(test)]
mod tests;
