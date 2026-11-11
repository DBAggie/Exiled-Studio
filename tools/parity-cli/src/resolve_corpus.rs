//! `resolve-corpus` (Spike B, PR B3): reads every `<Item>` of the corpus
//! builds with `loom_parser::pob_item`, resolves each modifier line in its
//! item's stat-description scope, and counts every line under exactly one
//! outcome (ADR-015: nothing is dropped).
//!
//! - **Scope** (Spike B plan, "Scope by mod domain"): the base's
//!   `BaseItemTypes.ModDomain` through the PoE1 item rules (`poe1`) and
//!   `loom_parser::import::find_base` (since P2-2b).
//! - **Base:** the base line of rare, unique, and relic items after
//!   `pob_item::base_name`; for normal and magic items (and items without
//!   a base line) `pob_item::magic_base` over every `BaseItemTypes.Name`;
//!   PoB's suffixed base names fall back to the game name
//!   (`pob_item::suffix_removed`).
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
//! - **Composed lines (B5c):** lines the client wraps in a `ClientStrings`
//!   text (`categorize::COMPOSITIONS`) are read through the wrapper
//!   (`Index::resolve_lines_composed`); the record names the wrapper.
//! - **Categories (B4b):** every line that is not resolved gets one
//!   category with its evidence (`categorize`); `special-case/author-note`
//!   and `data-gap/legacy-text` lines are excluded (maintainer decisions
//!   2026-11-10, B4b and B5b), and class requirement lines are excluded as
//!   `not-a-modifier`. The gate is computed in [`Gate`].
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
use loom_domain::{ItemBody as Item, LineKind, LineResolution, ModifierLine as Modifier, Rarity};
use loom_parser::import::{
    BaseError, BaseQuery, BaseRow, BundleData, FoundBase, ItemBaseName, ItemData, ItemRules,
    Resolver, find_base,
};
use loom_parser::resolve::compose::Composition;
use loom_parser::resolve::{
    Candidate, Index, IndexStats, Outcome, RawValue, Resolution, Unresolved,
};
use loom_serialization::legacy::pob_item;
use serde::Serialize;
use sha2::{Digest, Sha256};

mod build_check;

pub use build_check::BuildCheck;

use crate::categorize::{self, GameText, LineFacts};
use crate::extract::{Extract, Inputs, Read, Table};
use crate::grafts::{self, Grafts};
use crate::item_tags::ItemTags;
use crate::lookup::{LookupReport, TableLookup};
use crate::mod_filter::{self, Context, LineContext, Mods};
use crate::poe1;
use crate::properties;
use loom_parser::import::properties::Properties;
use loom_parser::import::tolerance;

/// `BaseItemTypes` rows by name, with the PoE1 item rules: the harness's
/// [`ItemData`].
#[derive(Debug)]
pub struct Bases {
    rules: ItemRules,
    rows: BTreeMap<String, Vec<BaseRow>>,
    /// Named rows whose tags are unknown (`item_tags`).
    pub tags_unknown: usize,
    /// Graft bases and their granted skills (B5g).
    pub grafts: Grafts,
    /// Item property texts and catalyst quality types (B5d).
    pub properties: Properties,
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
        let mut bases = Self::new();
        for row in &table.rows {
            let Some(n) = Table::text(row, name).filter(|n| !n.is_empty()) else {
                continue;
            };
            let row_domain = Table::int(row, domain);
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
        bases.grafts = grafts::load(read)?;
        bases.properties = properties::load(read)?;
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
        let mut bases = Self::new();
        for (n, row) in rows {
            bases.rows.entry(n.to_owned()).or_default().push(row);
        }
        bases
    }

    /// The bases, granted skills, and properties of a bundle (`--bundle`,
    /// plan step 6), with the bundle's item rules.
    pub fn from_bundle(data: &BundleData<'_>) -> Self {
        Self {
            rules: data.item_rules().clone(),
            rows: data
                .base_names()
                .filter_map(|n| data.base(n))
                .map(|(n, rows)| (n.to_owned(), rows.to_vec()))
                .collect(),
            tags_unknown: data.tags_unknown(),
            grafts: data.grafts().clone(),
            properties: data.properties().clone(),
        }
    }

    /// No rows, with the PoE1 item rules.
    fn new() -> Self {
        Self {
            rules: poe1::item_rules(),
            rows: BTreeMap::new(),
            tags_unknown: 0,
            grafts: Grafts::default(),
            properties: Properties::default(),
        }
    }

    /// The item's scope, or why there is none; `true` when the base is one
    /// of PoB's suffixed base names and was found after removing its
    /// ` (...)` suffix. Any other name is looked up as written.
    #[cfg(test)]
    fn scope(&self, item: &Item) -> Result<(&str, bool), String> {
        self.find(item).map(|f| (f.scope, f.fallback_used))
    }

    /// The item's base: its rows and scope, or why there is none. The name
    /// follows PoB's rules (`pob_item`); the lookup is the shared one.
    fn find(&self, item: &Item) -> Result<FoundBase<'_>, String> {
        let (name, fallback) =
            pob_item::item_base(item, self.base_names()).map_err(|e| e.name().to_owned())?;
        let query = BaseQuery {
            name: &name,
            fallback: fallback.as_deref(),
        };
        find_base(self, query).map_err(|e| {
            // The Spike B report's names for the shared lookup's errors.
            match e {
                BaseError::NotABase => "not-in-base-item-types",
                BaseError::ScopeConflict => "scope-conflict",
            }
            .to_owned()
        })
    }
}

impl ItemData for Bases {
    fn item_rules(&self) -> &ItemRules {
        &self.rules
    }

    fn base_names(&self) -> impl Iterator<Item = &str> {
        self.rows.keys().map(String::as_str)
    }

    fn base(&self, name: &str) -> Option<(&str, &[BaseRow])> {
        self.rows
            .get_key_value(name)
            .map(|(k, v)| (k.as_str(), v.as_slice()))
    }
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
    /// The data bundle the pipeline read (`--bundle`), if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle: Option<BundleInputs>,
    /// The tool.
    pub tool: Tool,
    /// Content hash of every build, sorted.
    pub builds: Vec<String>,
    /// PoB text sources for `data-gap/legacy-text` (B4b).
    pub pob: Vec<categorize::PobSource>,
}

/// The bundle a `--bundle` run read.
#[derive(Debug, Clone, Serialize)]
pub struct BundleInputs {
    /// `poe1` or `poe2`.
    pub game: String,
    /// The bundle's Steam build.
    pub steam_build: Option<u64>,
    /// SHA-256 of the install's bundle index, as the manifest records it.
    pub index_sha256: String,
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
    /// Of `resolved`, lines resolved to a description's stat IDs (no
    /// `kind`; B5g): the stat-only figure beside the headline.
    pub stat_resolved: usize,
    /// `stat_resolved / denominator`.
    pub stat_rate: f64,
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
    pub scopes: BTreeMap<String, IndexFields>,
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
    /// `ClientStrings` ids of the wrappers the resolver read through, with
    /// their stats (B5c; `categorize::COMPOSITIONS`).
    pub compositions: Vec<(String, String)>,
    /// Graft bases with a granted skill (`grafts`, B5g).
    pub graft_bases: usize,
    /// What the graft rule read: the wrapper found, the level range, and
    /// base names dropped because their rows disagree on the skill.
    pub graft_inputs: grafts::Inputs,
    /// Resolved lines whose result is not stat IDs, by group and kind
    /// (`non_unique/granted-skill`); counted as resolved in the rate.
    pub resolved_kinds: BTreeMap<String, usize>,
    /// Resolved lines that needed an import tolerance, by group and
    /// tolerance (`non_unique/case-folded`, B5e).
    pub adapted: BTreeMap<String, usize>,
    /// Item property rule inputs (B5d): `(ClientStrings templates,
    /// labels, AlternateQualityTypes rows)`.
    pub property_inputs: (usize, usize, usize),
    /// Property ids not read and quality descriptions dropped (B5d).
    pub property_missing: Vec<String>,
    /// `categorize::COMPOSITIONS` ids not read from `ClientStrings`.
    pub compositions_missing: Vec<String>,
    /// Per wrapper stat, `(Mods rows with it, of them eldritch implicits)`:
    /// the pairing holds when the two are equal and not zero.
    pub composition_rows: BTreeMap<String, (usize, usize)>,
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
    /// What a resolved line resolved to when it is not (only) a
    /// description's stat IDs: `granted-skill` (B5g; `detail` names the
    /// graft, skill, and level), `item-property` and `catalyst-quality`
    /// (B5d; `properties`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<&'static str>,
    /// An import tolerance the reading needed (B5e): `case-folded` (the
    /// text differs from game text in letter case only) or `split-section`
    /// (the line resolves with an adjacent line of another section).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapted: Option<&'static str>,
    /// The `ClientStrings` wrapper the line was read through (B5c).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composed: Option<String>,
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
    compositions: &'c [Composition],
}

impl<'i> Lines<'i, '_> {
    fn flush(&mut self, index: Option<&Index<'_>>, scope_err: Option<&str>) {
        if self.run.is_empty() {
            return;
        }
        let run = std::mem::take(&mut self.run);
        let section = |i: usize| {
            run.get(i)
                .map(|(_, m)| mod_filter::section_name(&m.section))
        };
        match (index, scope_err) {
            (Some(index), None) => {
                let texts: Vec<&str> = run.iter().map(|(_, m)| m.text.as_str()).collect();
                let mut at = 0;
                for composed in index.resolve_lines_composed(&texts, self.compositions) {
                    let mut resolution = composed.resolution;
                    let via = composed
                        .composition
                        .and_then(|n| self.compositions.get(n))
                        .map(|c| c.id.clone());
                    // A line game text holds in another letter case (B5e;
                    // `loom_parser::import::tolerance`).
                    let mut adapted = None;
                    if let Some(folded) = texts
                        .get(at)
                        .and_then(|t| tolerance::case_folded(index, t, &resolution, via.is_some()))
                    {
                        resolution = folded;
                        adapted = Some("case-folded");
                    }
                    let lines = resolution.lines;
                    self.push_resolution(&run, at, &resolution, section(at), via);
                    if let Some(a) = adapted
                        && let Some((last, _)) = self.records.last_mut()
                    {
                        last.adapted = Some(a);
                    }
                    at += lines;
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
        composed: Option<String>,
    ) {
        let mut record = LineRecord {
            section,
            composed,
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
            (self.ctx, line.as_ref(), &resolution.outcome)
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
        if let (Some(ctx), Some(line), Some(c)) = (self.ctx, line.as_ref(), resolved)
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
    compositions: &[Composition],
) -> Vec<(LineRecord, bool)> {
    let mut lines = Lines {
        records: Vec::new(),
        run: Vec::new(),
        ctx,
        compositions,
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
                Some(mod_filter::section_name(&m.section)),
                "not-selected-variant",
                m.text.clone(),
            ),
            LineKind::Modifier(m) if m.has("custom") => excluded(
                n,
                Some(mod_filter::section_name(&m.section)),
                "pob-custom",
                scrub(&m.text),
            ),
            // Class requirement (plan step 4b, maintainer decision
            // 2026-11-09): `v2.67.2:src/Classes/Item.lua` keeps these lines
            // in `classRequirementModLines` and sets `classRestriction`;
            // they are not modifiers. The class is kept on the item record.
            LineKind::Modifier(m) => excluded(
                n,
                Some(mod_filter::section_name(&m.section)),
                "not-a-modifier",
                m.text.clone(),
            ),
            LineKind::Flag | LineKind::Influence(_) => {
                excluded(n, None, "not-a-modifier", line.source.clone())
            }
            LineKind::Unrecognized => (
                LineRecord {
                    line: n,
                    section: None,
                    outcome: "unrecognized-line".to_owned(),
                    text: scrub(&line.source),
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
/// class requirement (the domain item marks the others
/// [`LineResolution::Excluded`], `pob_item::to_domain`).
fn resolvable(m: &Modifier) -> bool {
    matches!(m.resolution, LineResolution::Pending)
}

/// Sets `split_with` on a `no-template` line when it and the adjacent
/// resolvable line of another section resolve as one multi-line
/// description: PoB's `Implicits:` count can put the two lines of one
/// description in two sections, and the harness resolves sections apart.
/// Since B5e the line is resolved with the pair's reading (`adapted:
/// split-section`; not in `mod-backed`, the sections differ); before, it was
/// `parser-gap/split-section`.
fn mark_split_lines(item: &Item, index: &Index<'_>, records: &mut [(LineRecord, bool)]) {
    for (record, _) in records.iter_mut() {
        if record.outcome != "unresolved/no-template" {
            continue;
        }
        if let Some((other, r)) = tolerance::split_section(index, item, record.line) {
            // The pair is one description: both lines take its reading
            // (B5e), as the lines of any multi-line template do. `backed`
            // stays `None`: the halves sit in two sections, so no one
            // section's `Mods` fit applies; the line is left out of
            // `mod-backed` on purpose.
            record.split_with = Some(other);
            record.outcome = "resolved".to_owned();
            record.detail = detail(&r.outcome);
            record.adapted = Some("split-section");
            record.evidence = Some(format!("resolves with item line {other}"));
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
    bundle: Option<&BundleData<'_>>,
) -> Result<(Summary, Report, Option<BuildCheck>), String> {
    if extract.inputs.game != "poe1" {
        return Err(format!(
            "resolve-corpus reads PoE1 builds; the extract is {}",
            extract.inputs.game
        ));
    }
    let (indexes, chains, compositions, compositions_missing) = match bundle {
        Some(data) => bundle_inputs(data)?,
        None => extract_inputs(extract, lookup, bases, game)?,
    };
    let resolver = bundle
        .map(Resolver::new)
        .transpose()
        .map_err(|e| format!("{} ({e})", e.code()))?;
    let mut check = resolver.as_ref().map(|_| BuildCheck::default());
    let corpus = Corpus {
        bases,
        mods,
        indexes: &indexes,
        chains: &chains,
        compositions: &compositions,
        game,
        resolver: resolver.as_ref(),
    };

    let (essence_rows_known, essence_rows_unknown) = mods.essence_rows();
    let mut s = Summary {
        lookup: lookup_report,
        scopes: indexes
            .iter()
            .map(|(n, i)| ((*n).to_owned(), i.stats().into()))
            .collect(),
        mods_rows: mods.row_count(),
        mods_bad_spawn_rows: mods.bad_spawn_rows,
        essence_rows_known,
        essence_rows_unknown,
        bases_with_unknown_tags: bases.tags_unknown,
        colour_code_in_game_text: game.colour_code_in_game_text,
        compositions: compositions
            .iter()
            .map(|c| (c.id.clone(), c.stat.clone()))
            .collect(),
        compositions_missing,
        graft_bases: bases.grafts.len(),
        graft_inputs: grafts::inputs(&bases.grafts),
        property_inputs: bases.properties.inputs(),
        property_missing: properties::missing(&bases.properties),
        resolved_kinds: BTreeMap::new(),
        adapted: BTreeMap::new(),
        composition_rows: compositions
            .iter()
            .map(|c| (c.stat.clone(), mod_filter::eldritch_share(mods, &c.stat)))
            .collect(),
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
            let pob = pob_item::read(text);
            for (feature, n) in pob_item::unsupported_features(&pob) {
                *s.unsupported_features.entry(feature).or_default() += n;
            }
            let item = pob_item::to_domain(&pob);
            let record = count_item(
                &mut s,
                &mut distinct,
                (n, &item, ranges),
                &corpus,
                check.as_mut().map(|c| (c, build.hash.as_str())),
            );
            items.push(record);
        }
        report.builds.insert(build.hash.clone(), items);
    }
    s.inputs.tool = tool;
    s.inputs.extract = extract.inputs.clone();
    s.inputs.bundle = bundle.map(|d| {
        let m = &d.bundle().manifest;
        BundleInputs {
            game: m.game.clone(),
            steam_build: m.source.steam_build,
            index_sha256: m.source.index_sha256.clone(),
        }
    });
    s.inputs.pob = game.pob_sources();
    finish_rate(&mut s, &distinct);
    Ok((s, report, check))
}

/// Scope name to index, scope name to its chain's sources, the
/// compositions, and the composition IDs not read.
type PipelineInputs<'i> = (
    BTreeMap<&'i str, Index<'i>>,
    BTreeMap<&'i str, BTreeSet<String>>,
    Vec<Composition>,
    Vec<String>,
);

/// The pipeline's indexes and texts from the extract (Spike B's inputs).
fn extract_inputs<'i>(
    extract: &'i Extract,
    lookup: &TableLookup,
    bases: &'i Bases,
    game: &GameText,
) -> Result<PipelineInputs<'i>, String> {
    let mut scopes: BTreeMap<&str, Scope<'_>> = BTreeMap::new();
    for name in bases.item_rules().scopes() {
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
    let indexes = scopes
        .iter()
        .map(|(name, scope)| (*name, Index::build_with(scope, lookup)))
        .collect();
    let chains = scopes
        .iter()
        .map(|(name, scope)| {
            (
                *name,
                scope.chain().iter().map(|f| f.source.clone()).collect(),
            )
        })
        .collect();
    Ok((
        indexes,
        chains,
        game.compositions.clone(),
        game.compositions_missing.clone(),
    ))
}

/// The pipeline's indexes and texts from a bundle (`--bundle`).
fn bundle_inputs<'i>(data: &BundleData<'i>) -> Result<PipelineInputs<'i>, String> {
    let b = data.bundle();
    let indexes = data.indexes().map_err(|e| format!("{} ({e})", e.code()))?;
    let mut chains = BTreeMap::new();
    for name in b.item_rules.scopes() {
        let chain = b
            .scope_root(name)
            .ok()
            .and_then(|root| b.stat_descriptions.scope(root).ok())
            .ok_or_else(|| format!("{name}: no scope chain"))?;
        chains.insert(
            name,
            chain.chain().iter().map(|f| f.source.clone()).collect(),
        );
    }
    Ok((
        indexes,
        chains,
        data.compositions().to_vec(),
        data.compositions_missing()
            .into_iter()
            .map(str::to_owned)
            .collect(),
    ))
}

/// What every item of the run is resolved and categorized with.
struct Corpus<'c, 'i> {
    bases: &'c Bases,
    mods: &'c Mods,
    indexes: &'c BTreeMap<&'c str, Index<'i>>,
    /// Scope name to the sources of its chain's files.
    chains: &'c BTreeMap<&'c str, BTreeSet<String>>,
    /// The composition wrappers the pipeline reads.
    compositions: &'c [Composition],
    game: &'c GameText,
    /// The core pipeline, for the cross-check (`--bundle` only).
    resolver: Option<&'c Resolver<'c, 'i>>,
}

/// Resolves a graft's `Uses level {1} {0}` line as a granted skill
/// (`grafts`, B5g); other lines are left as they are.
fn granted_skill(record: &mut LineRecord, grafts: &Grafts, base: &str) {
    if record.outcome != "unresolved/no-template" {
        return;
    }
    if let Some(g) = grafts.resolve(base, &record.text) {
        record.outcome = "resolved".to_owned();
        record.kind = Some("granted-skill");
        record.composed = Some(grafts::WRAPPER_ID.to_owned());
        record.detail = vec![
            format!("graft={}", g.graft.id),
            format!("skill={}", g.graft.skill),
            format!("level={}", g.level),
        ];
    }
}

/// Resolves an item property line (`properties`, B5d); other lines are
/// left as they are.
fn item_property(record: &mut LineRecord, properties: &Properties) {
    if record.outcome != "unresolved/no-template" {
        return;
    }
    if let Some(p) = properties::resolve(properties, &record.text) {
        record.outcome = "resolved".to_owned();
        record.kind = Some(p.kind);
        record.detail = p.detail;
    }
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
    check: Option<(&mut BuildCheck, &str)>,
) -> ItemRecord {
    let (bases, mods, indexes) = (corpus.bases, corpus.mods, corpus.indexes);
    s.items += 1;
    let rarity = item.rarity.map_or("none", Rarity::name);
    *s.items_by_rarity.entry(rarity).or_default() += 1;
    if ranges > 0 {
        s.mod_range_items += 1;
        s.mod_range_elements += ranges;
    }
    let mut catalyst = false;
    for line in &item.lines {
        *s.line_kinds.entry(line.kind.name()).or_default() += 1;
        match &line.kind {
            LineKind::Spec { key, known, .. } => {
                catalyst |= key == "Catalyst";
                if !known {
                    *s.unknown_spec_keys.entry(safe_key(key)).or_default() += 1;
                }
            }
            LineKind::Modifier(m) => {
                for p in &m.prefixes {
                    *s.prefix_keys.entry(safe_key(&p.key)).or_default() += 1;
                }
                for p in m
                    .prefixes
                    .iter()
                    .filter(|p| !pob_item::is_known_prefix(&p.key))
                {
                    *s.unknown_prefix_keys.entry(safe_key(&p.key)).or_default() += 1;
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
            if f.fallback_used {
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
        rules: &bases.item_rules().mods,
        mods,
        bases: f.rows,
        unique,
        influences: item
            .lines
            .iter()
            .filter_map(|l| match &l.kind {
                LineKind::Influence(id) => bases.item_rules().mods.influence_value(id),
                _ => None,
            })
            .collect(),
    });
    let mut records = item_lines(
        item,
        index,
        found.as_ref().err().map(String::as_str),
        ctx.as_ref(),
        corpus.compositions,
    );
    if let Ok(f) = &found {
        for (record, _) in &mut records {
            granted_skill(record, &bases.grafts, f.name);
        }
    }
    for (record, _) in &mut records {
        item_property(record, &bases.properties);
    }
    if let (Some((check, build)), Some(resolver)) = (check, corpus.resolver) {
        let mut body = item.clone();
        let base = pob_item::item_base(item, bases.base_names())
            .map(|(name, fallback)| ItemBaseName { name, fallback })
            .map_err(pob_item::NoBaseName::name);
        resolver.resolve_item(&mut body, base.as_ref().map_err(|e| *e));
        let lines: Vec<LineRecord> = records.iter().map(|(r, _)| r.clone()).collect();
        check.compare(build, n, &body, &lines);
    }
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
        .map(|(_, m)| m)
        .filter(|m| mod_filter::section_name(&m.section) == "class-requirement")
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
        if let Some(kind) = record.kind {
            *s.resolved_kinds
                .entry(format!("{group}/{kind}"))
                .or_default() += 1;
        }
        if let Some(a) = record.adapted
            && record.outcome == "resolved"
        {
            *s.adapted.entry(format!("{group}/{a}")).or_default() += 1;
        }
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
    let kinds: usize = s
        .resolved_kinds
        .iter()
        .filter(|(k, _)| k.starts_with("non_unique/"))
        .map(|(_, n)| n)
        .sum();
    r.stat_resolved = r.resolved.saturating_sub(kinds);
    r.stat_rate = ratio(r.stat_resolved, r.denominator);
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
