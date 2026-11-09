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
//! - Categories are provisional (B4b); ambiguity is not filtered (B4a).
//!
//! **Scrubbing (ADR-007):** builds are named by the SHA-256 of their
//! content; item names, `{custom}` text, and unrecognized lines are never
//! written (only a hash of the text). Mod lines from game text are written.

use std::collections::{BTreeMap, BTreeSet};

use loom_data::stat_desc::{Hand, Scope};
use loom_parser::pob_item::{self, BaseMatch, Item, LineKind, Modifier, Rarity};
use loom_parser::resolve::{
    Candidate, Index, IndexStats, Outcome, RawValue, Resolution, Unresolved,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::extract::{Extract, Inputs, Read, Table};
use crate::lookup::{LookupReport, TableLookup};

/// The resolution rate the Spike B gate asks for.
pub const GATE_RATE: f64 = 0.98;

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

/// `BaseItemTypes` names and the scopes their rows give.
#[derive(Debug, Default)]
pub struct Bases {
    scopes: BTreeMap<String, BTreeSet<&'static str>>,
}

impl Bases {
    /// Reads `BaseItemTypes` (`Name`, `ModDomain`).
    ///
    /// # Errors
    /// A message when the table or a column is missing.
    pub fn load(read: Read<'_>) -> Result<Self, String> {
        let table = Table::require(read, "BaseItemTypes")?;
        let (name, domain) = (table.column("Name")?, table.column("ModDomain")?);
        let mut bases = Self::default();
        for row in &table.rows {
            if let Some(n) = Table::text(row, name).filter(|n| !n.is_empty()) {
                bases
                    .scopes
                    .entry(n.to_owned())
                    .or_default()
                    .insert(scope_of_domain(Table::int(row, domain)));
            }
        }
        Ok(bases)
    }

    #[cfg(test)]
    pub fn from_pairs(pairs: &[(&str, i64)]) -> Self {
        let mut bases = Self::default();
        for (n, d) in pairs {
            bases
                .scopes
                .entry((*n).to_owned())
                .or_default()
                .insert(scope_of_domain(Some(*d)));
        }
        bases
    }

    /// The item's scope, or why there is none; `true` when the base was
    /// found only after removing a PoB-only ` (...)` suffix (PoB names the
    /// three `Two-Toned Boots` rows `Two-Toned Boots (Armour/Energy
    /// Shield)` and so on; `v2.67.2:src/Classes/Item.lua` `ParseRaw` has a
    /// "Hack for Two-Toned Boots", and removes ` %(.+%)` from magic names).
    fn scope(&self, item: &Item) -> Result<(&'static str, bool), String> {
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
        let stripped = name
            .strip_suffix(')')
            .and_then(|n| n.rsplit_once(" ("))
            .map(|(n, _)| n);
        let (scopes, suffix) = match (self.scopes.get(&name), stripped) {
            (Some(s), _) => (s, false),
            (None, Some(n)) => (
                self.scopes
                    .get(n)
                    .ok_or_else(|| "not-in-base-item-types".to_owned())?,
                true,
            ),
            (None, None) => return Err("not-in-base-item-types".to_owned()),
        };
        match scopes.iter().collect::<Vec<_>>().as_slice() {
            [one] => Ok((one, suffix)),
            _ => Err("scope-conflict".to_owned()),
        }
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
    /// Unresolved texts by outcome. Text is written only for outcomes where
    /// a template matched (`no-variant-fits`, `only-shadowed`), so it is
    /// game text; other text is a hash (it may be the author's own words).
    pub unresolved: BTreeMap<String, BTreeMap<String, Distinct>>,
    /// Items whose base was found only after removing a PoB-only
    /// ` (...)` suffix.
    pub pob_base_suffix_removed: usize,
    /// Measurements for B4b's categories, not categories: mark to
    /// `<group>/<outcome>` to lines not resolved (see [`marks`]).
    pub marks: BTreeMap<&'static str, BTreeMap<String, usize>>,
}

/// Outcomes whose text a template matched.
const TEMPLATE_MATCHED: [&str; 2] = ["unresolved/no-variant-fits", "unresolved/only-shadowed"];

/// Shapes of a line's text that B4b's categories will need evidence for:
/// - `color-code`: starts with a PoB colour escape (`^1`..`^9`, `^x`),
///   text the build's author wrote;
/// - `spec-shaped`: a spec line `ParseRaw` knows (`Quality (... Modifiers):
///   +20%`, `Memory Strands: 63`) in the modifier section, pasted from the
///   game's item text (`ParseRaw` also reads it as a modifier there);
/// - `small-passive-grant`: `Added Small Passive Skills grant: ...`, a
///   cluster jewel's `local_jewel_expansion_passive_node_index` line, whose
///   only variant is `{0}` through `tree_expansion_jewel_passive` (not
///   supported, see `lookup.rs`).
fn marks(text: &str) -> Vec<&'static str> {
    let mut out = Vec::new();
    let mut chars = text.chars();
    if chars.next() == Some('^') && chars.next().is_some_and(|c| c.is_ascii_digit() || c == 'x') {
        out.push("color-code");
    }
    if pob_item::spec_key(text).is_some_and(|k| {
        pob_item::KNOWN_SPECS.contains(&k)
            || (k.starts_with("Quality (") && k.ends_with(" Modifiers)"))
    }) {
        out.push("spec-shaped");
    }
    if text.starts_with("Added Small Passive Skills grant: ") {
        out.push("small-passive-grant");
    }
    out
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
#[derive(Debug, Clone, Serialize)]
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
struct Lines<'i> {
    records: Vec<(LineRecord, bool)>,
    run: Vec<(usize, &'i Modifier)>,
}

impl<'i> Lines<'i> {
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
        let outcome = outcome_name(&resolution.outcome);
        let detail = detail(&resolution.outcome);
        for k in 0..resolution.lines {
            if let Some((line, _)) = run.get(at + k) {
                self.records.push((
                    LineRecord {
                        line: *line,
                        section,
                        outcome: outcome.clone(),
                        text: resolution.line.clone(),
                        joined: resolution.lines,
                        detail: detail.clone(),
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
        },
        false,
    )
}

/// Resolves one item's lines.
fn item_lines(
    item: &Item,
    index: Option<&Index<'_>>,
    scope_err: Option<&str>,
) -> Vec<(LineRecord, bool)> {
    let mut lines = Lines {
        records: Vec::new(),
        run: Vec::new(),
    };
    for (n, line) in item.lines.iter().enumerate() {
        match &line.kind {
            LineKind::Modifier(m) if m.selected && !m.has("custom") => {
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
            LineKind::Modifier(m) => {
                excluded(n, Some(m.section.name()), "pob-custom", scrub(&m.text))
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
                },
                false,
            ),
            _ => continue,
        };
        lines.records.push(record);
    }
    lines.flush(index, scope_err);
    lines.records.sort_by_key(|(r, _)| r.line);
    lines.records
}

/// Runs the corpus over a loaded extract.
///
/// # Errors
/// A message when the extract is not PoE1 or a scope does not build.
pub fn run(
    extract: &Extract,
    lookup: &TableLookup,
    lookup_report: LookupReport,
    bases: &Bases,
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

    let mut s = Summary {
        lookup: lookup_report,
        scopes: indexes
            .iter()
            .map(|(n, i)| (*n, i.stats().into()))
            .collect(),
        ..Summary::default()
    };
    let mut report = Report::default();
    let mut sorted: Vec<&Build> = builds.iter().collect();
    sorted.sort_by(|a, b| a.hash.cmp(&b.hash));
    let mut distinct: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();

    for build in sorted {
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
            let text: String = node
                .descendants()
                .filter(roxmltree::Node::is_text)
                .filter_map(|t| t.text())
                .collect();
            let ranges = node
                .children()
                .filter(|c| c.has_tag_name("ModRange"))
                .count();
            let item = pob_item::read(&text);
            let record = count_item(&mut s, &mut distinct, n, &item, ranges, bases, &indexes);
            items.push(record);
        }
        report.builds.insert(build.hash.clone(), items);
    }
    s.inputs.tool = tool;
    s.inputs.extract = extract.inputs.clone();
    finish_rate(&mut s, &distinct);
    Ok((s, report))
}

fn count_item(
    s: &mut Summary,
    distinct: &mut BTreeMap<(String, String), BTreeSet<String>>,
    n: usize,
    item: &Item,
    ranges: usize,
    bases: &Bases,
    indexes: &BTreeMap<&'static str, Index<'_>>,
) -> ItemRecord {
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
            LineKind::Spec { key, known } => {
                catalyst |= key == "Catalyst";
                if !known {
                    *s.unknown_spec_keys.entry(key.clone()).or_default() += 1;
                }
            }
            LineKind::Modifier(m) => {
                for p in &m.prefixes {
                    *s.prefix_keys.entry(p.key.clone()).or_default() += 1;
                }
                for k in m.unknown_prefixes() {
                    *s.unknown_prefix_keys.entry(k.to_owned()).or_default() += 1;
                }
            }
            _ => {}
        }
    }
    if catalyst {
        s.catalyst_items += 1;
    }
    let scope = bases.scope(item);
    let scope_name = match &scope {
        Ok((name, suffix)) => {
            if *suffix {
                s.pob_base_suffix_removed += 1;
            }
            (*name).to_owned()
        }
        Err(reason) => format!("unknown-base/{reason}"),
    };
    *s.items_by_scope.entry(scope_name.clone()).or_default() += 1;
    let index = scope.as_ref().ok().and_then(|(name, _)| indexes.get(name));
    let records = item_lines(item, index, scope.as_ref().err().map(String::as_str));
    let unique = item.rarity.is_none_or(Rarity::is_unique);
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
        if !excluded && record.outcome != "resolved" {
            for mark in marks(&record.text) {
                let group = if unique { "unique" } else { "non_unique" };
                *s.marks
                    .entry(mark)
                    .or_default()
                    .entry(format!("{group}/{}", record.outcome))
                    .or_default() += 1;
            }
        }
        let entry = match record.outcome.as_str() {
            "resolved" => None,
            "ambiguous" => Some(s.ambiguous.entry(record.text.clone()).or_default()),
            o if o.starts_with("excluded/") => None,
            o => {
                // Only text a template matched is known to be game text;
                // other text may be written by the build's author.
                let text = if TEMPLATE_MATCHED.contains(&o) {
                    record.text.clone()
                } else {
                    scrub(&record.text)
                };
                Some(
                    s.unresolved
                        .entry(o.to_owned())
                        .or_default()
                        .entry(text)
                        .or_default(),
                )
            }
        };
        if let Some(entry) = entry {
            if unique {
                entry.unique += 1;
            } else {
                entry.non_unique += 1;
            }
            entry.detail.extend(record.detail.iter().cloned());
        }
    }
    ItemRecord {
        item: n,
        rarity,
        scope: scope_name,
        lines: records.into_iter().map(|(r, _)| r).collect(),
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
    r.meets_rate = r.denominator > 0 && r.rate >= GATE_RATE;
}

#[cfg(test)]
mod tests;
