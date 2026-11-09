//! `render-check <extract> <pob-dir> --out <dir>` (Spike B, PR B2b): an
//! independent check of the renderer. The round trip tests the renderer
//! against the resolver; this compares it with PoB's exported mod text.
//!
//! For every PoE1 `Mods` row whose `Id` PoB exports, the row's stats are
//! rendered with their `(min, max)` values and the lines are compared with
//! PoB's text for that `Id` (`src/Data/Mod*.lua` of the PoB data export in
//! `local/pob/`, the same export A6a's cross-check uses; cross-check only,
//! ADR-005). The stats follow PoB's `describeMod`
//! (`v2.67.2:src/Export/statdesc.lua`): stat slots 1 to 6, without the
//! stats of the row's buff templates. The scope follows
//! `v2.67.2:src/Export/Scripts/mods.lua`, which loads
//! `tincture_stat_descriptions` (it includes `stat_descriptions`) and then
//! `graft_stat_descriptions` over it: a stat that `graft` describes itself
//! is rendered there, every other stat in the `tincture` scope. Lookup
//! handlers write the raw value, as PoB's `describeStats` does (it names
//! none of them; `mod_value_to_item_class` aside, which no compared row
//! reaches).
//!
//! Every compared row ends in one outcome; a difference gets the first
//! [`Category`] whose rule holds on the row, else `uncategorized`.

use std::collections::{BTreeMap, BTreeSet};

use loom_data::stat_desc::{
    Condition, DescRef, Description, Handler, Lookup, LookupHandler, NotRenderedReason, Scope,
    StatValue, VariantFilter, render_description,
};
use serde::Serialize;
use serde_json::Value;

use crate::extract::{Extract, Inputs, Read, Table};

/// Examples kept per outcome in the summary (all are in the report).
const EXAMPLES: usize = 5;

/// PoB files under `src/Data` that start with `Mod` but are not keyed by
/// mod `Id` (A6a, `tools/extractor/src/crosscheck.rs` `NON_MOD_ID_FILES`).
pub const NON_MOD_ID_FILES: [&str; 4] = [
    "ModCache.lua",
    "ModFoulbornMap.lua",
    "ModRunes.lua",
    "ModScalability.lua",
];

/// Counts of one run. Field order is the JSON order.
#[derive(Debug, Default, Serialize)]
pub struct Summary {
    /// Recorded inputs.
    pub inputs: Inputs,
    /// PoB mod files read, with the entries read from each.
    pub pob_files: BTreeMap<String, usize>,
    /// PoB entry lines that start like an entry but could not be read.
    pub pob_unparsed: usize,
    /// Distinct `Id`s in PoB's mod files.
    pub pob_ids: usize,
    /// `Id`s with more than one distinct text in PoB's mod files.
    pub pob_ids_with_several_texts: usize,
    /// PoB stat-description files read (for `patch-delta`), with the
    /// descriptions read from each.
    pub pob_stat_description_files: BTreeMap<String, usize>,
    /// `Mods` rows.
    pub mods_rows: usize,
    /// Rows not compared: PoB exports no entry for the `Id` (A6a's
    /// cross-check categorizes them).
    pub not_in_pob: usize,
    /// PoB `Id`s that no `Mods` row has.
    pub pob_only: usize,
    /// Compared rows by outcome:
    /// - `match`: the same lines in the same order;
    /// - `order-only`: the same lines in another order;
    /// - `differs/<category>`: see [`Category`].
    pub outcomes: BTreeMap<String, usize>,
    /// Numeric handlers (by name) used in rows that `match` or differ only
    /// in order: the handler semantics PoB's independent text confirms,
    /// with the number of rows.
    pub numeric_handlers_confirmed: BTreeMap<&'static str, usize>,
    /// Numeric handlers used in rows that differ, with the number of rows.
    pub numeric_handlers_in_differing_rows: BTreeMap<&'static str, usize>,
    /// The first examples per outcome other than `match`.
    pub examples: BTreeMap<String, Vec<Difference>>,
}

/// One compared row that is not a `match`.
#[derive(Debug, Clone, Serialize)]
pub struct Difference {
    /// Outcome name.
    pub outcome: String,
    /// `Mods.Id`.
    pub id: String,
    /// Stats as rendered: `[stat, min, max]`.
    pub stats: Vec<(String, i64, i64)>,
    /// Our lines.
    pub ours: Vec<String>,
    /// PoB's lines (the first of several texts).
    pub pob: Vec<String>,
    /// Stats the renderer did not render (value 0 left out), with the
    /// reason.
    pub not_rendered: Vec<(String, String)>,
    /// Why the category applies.
    pub evidence: String,
}

/// Every non-`match` row.
#[derive(Debug, Default, Serialize)]
pub struct Report {
    /// In `Mods` row order.
    pub differences: Vec<Difference>,
}

/// What the check reads from the PoB export.
#[derive(Debug, Default)]
pub struct Pob {
    /// `Id` to each distinct text (lines), in file order.
    pub texts: BTreeMap<String, Vec<Vec<String>>>,
    /// Mod files and the entries read from each.
    pub files: BTreeMap<String, usize>,
    /// Entry-like lines that did not parse.
    pub unparsed: usize,
    /// Stat list (joined by spaces) to every variant text PoB's exported
    /// stat descriptions hold for it.
    pub stat_texts: BTreeMap<String, BTreeSet<String>>,
    /// Stat-description files and the descriptions read from each.
    pub stat_files: BTreeMap<String, usize>,
}

impl Pob {
    /// Reads one PoB mod file. Entries are lines
    /// `\t["Id"] = { [type = "...", ][affix = "...", ]"line", ..., key = ...`
    /// as `Export/Scripts/mods.lua` writes them (one tab; PoB writes the
    /// text between quotes without escaping). Other lines are skipped; a
    /// one-tab `["` line that does not fit is counted as unparsed.
    pub fn add_mod_file(&mut self, name: &str, text: &str) {
        let mut entries = 0;
        for line in text.lines() {
            let Some(rest) = line.strip_prefix("\t[\"") else {
                continue;
            };
            match entry(rest) {
                Some((id, lines)) => {
                    entries += 1;
                    let texts = self.texts.entry(id.to_owned()).or_default();
                    if !texts.contains(&lines) {
                        texts.push(lines);
                    }
                }
                None => self.unparsed += 1,
            }
        }
        self.files.insert(name.to_owned(), entries);
    }

    /// Reads one exported stat-description file
    /// (`src/Data/StatDescriptions/*.lua`): per description block, every
    /// `text="..."` line and the `stats={ [n]="..." }` list after them.
    pub fn add_stat_file(&mut self, name: &str, text: &str) {
        let mut texts: Vec<String> = Vec::new();
        let mut stats: Option<Vec<String>> = None;
        let mut blocks = 0;
        for line in text.lines() {
            let line = line.trim();
            if let Some(stats_now) = stats.as_mut() {
                if line.starts_with('}') {
                    let key = stats_now.join(" ");
                    self.stat_texts
                        .entry(key)
                        .or_default()
                        .extend(texts.drain(..));
                    stats = None;
                    blocks += 1;
                } else if let Some(stat) = line
                    .split_once("]=\"")
                    .and_then(|(_, s)| s.strip_suffix("\",").or_else(|| s.strip_suffix('"')))
                {
                    stats_now.push(stat.to_owned());
                }
            } else if line == "stats={" {
                stats = Some(Vec::new());
            } else if let Some(t) = line
                .strip_prefix("text=\"")
                .and_then(|s| s.strip_suffix("\",").or_else(|| s.strip_suffix('"')))
            {
                texts.push(t.to_owned());
            }
        }
        self.stat_files.insert(name.to_owned(), blocks);
    }
}

/// `Id"] = { ...` to the `Id` and its text lines; `None` if the shape
/// does not fit or there is no text.
fn entry(rest: &str) -> Option<(&str, Vec<String>)> {
    let (id, body) = rest.split_once("\"] = { ")?;
    let mut body = body;
    for key in ["type = \"", "affix = \""] {
        if let Some(after) = body.strip_prefix(key) {
            let (_, tail) = after.split_once("\", ")?;
            body = tail;
        }
    }
    let mut lines = Vec::new();
    while let Some(after) = body.strip_prefix('"') {
        let (text, tail) = after.split_once("\", ")?;
        lines.push(text.to_owned());
        body = tail;
    }
    (!lines.is_empty()).then_some((id, lines))
}

/// Why a compared row differs. Each rule is checked on the row, in this
/// order; a row no rule explains is `uncategorized`. All rules but
/// `patch-delta` reproduce PoB's export code
/// (`v2.67.2:src/Export/statdesc.lua`, `Export/Scripts/mods.lua`) on our
/// rendering and require the result to equal PoB's lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    /// `mods.lua` rewrites the text after describing it: the
    /// `EldritchImplicitUniquePresence` / `PinnaclePresence` prefix, or
    /// `JewelExpansionPassiveNodes`' second stat set to `2-12`.
    PobRewritten,
    /// `describeStats` does not stop at a handler it does not know: it logs
    /// it and writes the value unchanged. Our renderer reports
    /// `unknown-handler`; rendering the description without its unknown
    /// handlers gives PoB's lines.
    PobIgnoresUnknownHandler,
    /// `describeStats` retries a description with no matching variant
    /// after moving a `0` minimum to 1 (a `0` maximum to -1), then writes
    /// the original values with the variant found. Our renderer reports
    /// `no-matching-variant`; doing the same gives PoB's lines.
    PobZeroRangeRetry,
    /// PoB keeps a line's surrounding spaces (it splits on `\n` without
    /// trimming); our renderer trims. Trimmed, PoB's lines equal ours.
    PobUntrimmed,
    /// A lookup stat with a range: PoB writes the range of raw keys, our
    /// renderer names no single row (`lookup-range`); every line we write
    /// is one of PoB's.
    PobLookupRange,
    /// The game text changed since PoB's export: for a description we
    /// render, PoB's exported stat descriptions
    /// (`src/Data/StatDescriptions/*.lua`, same stat list) hold a variant
    /// text the extract's description does not have.
    PatchDelta,
    /// Anything else.
    Uncategorized,
}

impl Category {
    /// Stable name for reports.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::PobRewritten => "pob-rewritten",
            Self::PobIgnoresUnknownHandler => "pob-ignores-unknown-handler",
            Self::PobZeroRangeRetry => "pob-zero-range-retry",
            Self::PobUntrimmed => "pob-untrimmed",
            Self::PobLookupRange => "pob-lookup-range",
            Self::PatchDelta => "patch-delta",
            Self::Uncategorized => "uncategorized",
        }
    }
}

/// Writes every lookup value as its number, as PoB's export does.
struct RawNumbers;

impl Lookup for RawNumbers {
    fn name(&self, _: LookupHandler, value: i64) -> Option<String> {
        Some(value.to_string())
    }

    fn entries(&self, _: LookupHandler) -> Vec<(i64, String)> {
        Vec::new()
    }
}

/// A row rendered through [`Scopes`].
struct Rendered<'a> {
    lines: Vec<String>,
    /// Stats not rendered (value 0 left out), with the reason.
    not_rendered: Vec<(String, NotRenderedReason)>,
    /// Descriptions that wrote lines.
    described: Vec<&'a Description>,
    /// Numeric handlers of the variants that wrote lines.
    numeric: BTreeSet<&'static str>,
}

/// The two scopes `mods.lua` loads.
struct Scopes<'a> {
    tincture: Scope<'a>,
    graft: Scope<'a>,
}

impl<'a> Scopes<'a> {
    /// The scope a stat is rendered in.
    fn scope_of(&self, stat: &str) -> &Scope<'a> {
        if self.graft.find(stat).is_some_and(|at| at.depth == 0) {
            &self.graft
        } else {
            &self.tincture
        }
    }

    fn render(&self, stats: &[(String, i64, i64)]) -> Rendered<'a> {
        let mut tincture = BTreeMap::new();
        let mut graft = BTreeMap::new();
        for (stat, min, max) in stats {
            let value = StatValue {
                min: *min.min(max),
                max: *min.max(max),
            };
            if std::ptr::eq(self.scope_of(stat), &self.graft) {
                graft.insert(stat.clone(), value);
            } else {
                tincture.insert(stat.clone(), value);
            }
        }
        let mut out = Rendered {
            lines: Vec::new(),
            not_rendered: Vec::new(),
            described: Vec::new(),
            numeric: BTreeSet::new(),
        };
        for (scope, values) in [(&self.tincture, tincture), (&self.graft, graft)] {
            if values.is_empty() {
                continue;
            }
            let r = scope.render(&values, &RawNumbers, VariantFilter::default());
            for line in r.lines {
                if let Some((_, d)) = scope.description(line.at) {
                    if let Some(v) = d.variants.get(line.variant) {
                        out.numeric
                            .extend(v.handlers.iter().filter_map(|h| match h {
                                Handler::Numeric { handler, .. } => Some(handler.name),
                                _ => None,
                            }));
                    }
                    if !out.described.iter().any(|x| std::ptr::eq(*x, d)) {
                        out.described.push(d);
                    }
                }
                out.lines.push(line.text);
            }
            out.not_rendered.extend(
                r.not_rendered
                    .into_iter()
                    .filter(|n| n.reason != NotRenderedReason::Zero)
                    .map(|n| (n.stat, n.reason)),
            );
        }
        out
    }

    /// The descriptions behind stats not rendered for `reason`, each once,
    /// with the row's values in description order.
    fn failed(
        &self,
        stats: &[(String, i64, i64)],
        rendered: &Rendered<'_>,
        reason: &str,
    ) -> Vec<(&'a Description, DescRef, Vec<StatValue>)> {
        let values: BTreeMap<&str, StatValue> = stats
            .iter()
            .map(|(s, min, max)| {
                (
                    s.as_str(),
                    StatValue {
                        min: *min.min(max),
                        max: *min.max(max),
                    },
                )
            })
            .collect();
        let mut out: Vec<(&'a Description, DescRef, Vec<StatValue>)> = Vec::new();
        for (stat, r) in &rendered.not_rendered {
            if r.name() != reason {
                continue;
            }
            let scope = self.scope_of(stat);
            let Some(at) = scope.find(stat) else {
                continue;
            };
            let Some((_, d)) = scope.description(at) else {
                continue;
            };
            if out.iter().any(|(x, ..)| std::ptr::eq(*x, d)) {
                continue;
            }
            let ids = d.stats_of(at.hand).unwrap_or_default();
            let vals = ids
                .iter()
                .map(|id| {
                    values
                        .get(id.as_str())
                        .copied()
                        .unwrap_or(StatValue::exact(0))
                })
                .collect();
            out.push((d, at, vals));
        }
        out
    }
}

/// The PoE1 `Mods` layout: `StatsKey<n>`, `Stat<n>Min`, `Stat<n>Max`.
struct ModsTable {
    table: Table,
    id: usize,
    slots: Vec<(usize, usize, usize)>,
    buffs: Vec<usize>,
}

/// Runs the check.
///
/// # Errors
/// A message when a table or column is missing or a scope does not build.
pub fn run(extract: &Extract, read: Read<'_>, pob: &Pob) -> Result<(Summary, Report), String> {
    let mut s = Summary {
        inputs: extract.inputs.clone(),
        pob_files: pob.files.clone(),
        pob_unparsed: pob.unparsed,
        pob_ids: pob.texts.len(),
        pob_ids_with_several_texts: pob.texts.values().filter(|t| t.len() > 1).count(),
        pob_stat_description_files: pob.stat_files.clone(),
        ..Summary::default()
    };
    let scope = |name: &str| {
        extract
            .set
            .scope(&format!("metadata/statdescriptions/{name}.txt"))
            .map_err(|e| format!("{name}: {} ({e})", e.code()))
    };
    let scopes = Scopes {
        tincture: scope("tincture_stat_descriptions")?,
        graft: scope("graft_stat_descriptions")?,
    };
    let mods = mods_table(read)?;
    let stat_ids = Table::require(read, "Stats")?;
    let stat_id = stat_ids.column("Id")?;
    let buff_templates = Table::require(read, "BuffTemplates")?;
    let buff_stats = buff_templates
        .columns
        .iter()
        .position(|c| c.references.as_deref() == Some("Stats"))
        .ok_or("BuffTemplates: no column of Stats keys")?;
    let stat_name = |key: i64| -> Option<String> {
        let row = stat_ids.rows.get(usize::try_from(key).ok()?)?;
        Table::text(row, stat_id).map(str::to_owned)
    };
    let mut report = Report::default();
    let mut ours_ids = BTreeSet::new();
    for row in &mods.table.rows {
        s.mods_rows += 1;
        let Some(id) = Table::text(row, mods.id) else {
            continue;
        };
        ours_ids.insert(id.to_owned());
        let Some(texts) = pob.texts.get(id) else {
            s.not_in_pob += 1;
            continue;
        };
        let skip: BTreeSet<i64> = mods
            .buffs
            .iter()
            .filter_map(|c| row.get(*c))
            .flat_map(keys)
            .filter_map(|b| buff_templates.rows.get(usize::try_from(b).ok()?))
            .filter_map(|b| b.get(buff_stats))
            .flat_map(keys)
            .collect();
        let stats: Vec<(String, i64, i64)> = mods
            .slots
            .iter()
            .filter_map(|&(key, min, max)| {
                let k = Table::int(row, key)?;
                if skip.contains(&k) {
                    return None;
                }
                Some((stat_name(k)?, Table::int(row, min)?, Table::int(row, max)?))
            })
            .collect();
        let rendered = scopes.render(&stats);
        let (outcome, evidence) = if texts.contains(&rendered.lines) {
            ("match".to_owned(), String::new())
        } else if texts.iter().any(|t| same_lines(t, &rendered.lines)) {
            ("order-only".to_owned(), String::new())
        } else {
            let (category, evidence) = categorize(id, &stats, &rendered, texts, &scopes, pob);
            (format!("differs/{}", category.name()), evidence)
        };
        *s.outcomes.entry(outcome.clone()).or_default() += 1;
        let handlers = if outcome.starts_with("differs/") {
            &mut s.numeric_handlers_in_differing_rows
        } else {
            &mut s.numeric_handlers_confirmed
        };
        for name in &rendered.numeric {
            *handlers.entry(name).or_default() += 1;
        }
        if outcome != "match" {
            let d = Difference {
                outcome,
                id: id.to_owned(),
                stats,
                ours: rendered.lines,
                pob: texts.first().cloned().unwrap_or_default(),
                not_rendered: rendered
                    .not_rendered
                    .into_iter()
                    .map(|(s, r)| (s, r.name().to_owned()))
                    .collect(),
                evidence,
            };
            let examples = s.examples.entry(d.outcome.clone()).or_default();
            if examples.len() < EXAMPLES {
                examples.push(d.clone());
            }
            report.differences.push(d);
        }
    }
    s.pob_only = pob.texts.keys().filter(|k| !ours_ids.contains(*k)).count();
    Ok((s, report))
}

/// Integer keys of a cell: one key or an array of keys.
fn keys(v: &Value) -> Vec<i64> {
    match v {
        Value::Array(a) => a.iter().filter_map(Value::as_i64).collect(),
        other => other.as_i64().into_iter().collect(),
    }
}

fn mods_table(read: Read<'_>) -> Result<ModsTable, String> {
    let table = Table::require(read, "Mods")?;
    let id = table.column("Id")?;
    let mut slots = Vec::new();
    for n in 1..=6 {
        slots.push((
            table.column(&format!("StatsKey{n}"))?,
            table.column(&format!("Stat{n}Min"))?,
            table.column(&format!("Stat{n}Max"))?,
        ));
    }
    let buffs = table
        .columns
        .iter()
        .enumerate()
        .filter(|(_, c)| c.references.as_deref() == Some("BuffTemplates"))
        .map(|(i, _)| i)
        .collect();
    Ok(ModsTable {
        table,
        id,
        slots,
        buffs,
    })
}

fn same_lines(a: &[String], b: &[String]) -> bool {
    let mut a = a.to_vec();
    let mut b = b.to_vec();
    a.sort();
    b.sort();
    a == b
}

/// A copy of `d` with only variant `index`, its conditions open, and its
/// unknown handlers removed when `drop_unknown`.
fn forced(d: &Description, index: usize, drop_unknown: bool) -> Option<Description> {
    let mut v = d.variants.get(index)?.clone();
    v.conditions.fill(Condition::Range {
        min: None,
        max: None,
    });
    if drop_unknown {
        v.handlers.retain(|h| !matches!(h, Handler::Unknown { .. }));
    }
    let mut copy = d.clone();
    copy.variants = vec![v];
    Some(copy)
}

/// The variant PoB's `matchLimit` picks for `values` (minimum tested),
/// skipping `table_only` only, as `describeStats` does.
fn match_limit(d: &Description, values: &[StatValue]) -> Option<usize> {
    d.variants.iter().position(|v| {
        !v.table_only
            && v.conditions
                .iter()
                .zip(values)
                .all(|(c, value)| c.holds(value.min))
    })
}

/// Our lines with the descriptions that failed for `reason` written the
/// way PoB's export writes them; `None` if one cannot be.
fn with_pob_fallback(
    scopes: &Scopes<'_>,
    stats: &[(String, i64, i64)],
    rendered: &Rendered<'_>,
    reason: &str,
) -> Option<Vec<String>> {
    let failed = scopes.failed(stats, rendered, reason);
    if failed.is_empty() {
        return None;
    }
    let mut lines = rendered.lines.clone();
    for (d, at, values) in failed {
        let index = match reason {
            "no-matching-variant" => {
                let raised: Vec<StatValue> = values
                    .iter()
                    .map(|v| match (v.min, v.max) {
                        (0, max) if max > 0 => StatValue { min: 1, max },
                        (min, 0) if min < 0 => StatValue { min, max: -1 },
                        _ => *v,
                    })
                    .collect();
                match_limit(d, &raised)?
            }
            _ => match_limit(d, &values)?,
        };
        let copy = forced(d, index, reason == "unknown-handler")?;
        let (_, more) = render_description(
            &copy,
            at.hand,
            &values,
            &RawNumbers,
            VariantFilter {
                gem_quality: true,
                table_only: false,
            },
        )
        .ok()?;
        lines.extend(more);
    }
    Some(lines)
}

/// The first category whose rule holds for the row, with the evidence.
fn categorize(
    id: &str,
    stats: &[(String, i64, i64)],
    rendered: &Rendered<'_>,
    texts: &[Vec<String>],
    scopes: &Scopes<'_>,
    pob: &Pob,
) -> (Category, String) {
    let equals = |lines: &[String]| texts.iter().any(|t| same_lines(t, lines));
    for (marker, prefix) in [
        (
            "EldritchImplicitUniquePresence",
            "While a Unique Enemy is in your Presence, ",
        ),
        (
            "EldritchImplicitPinnaclePresence",
            "While a Pinnacle Atlas Boss is in your Presence, ",
        ),
    ] {
        if id.contains(marker) {
            let rewritten: Vec<String> = rendered
                .lines
                .iter()
                .map(|l| format!("{prefix}{l}"))
                .collect();
            if equals(&rewritten) {
                return (
                    Category::PobRewritten,
                    format!("mods.lua prefixes `{prefix}` for ids containing {marker}"),
                );
            }
        }
    }
    if id == "JewelExpansionPassiveNodes" {
        let mut changed = stats.to_vec();
        if let Some(second) = changed.get_mut(1) {
            (second.1, second.2) = (2, 12);
        }
        if equals(&scopes.render(&changed).lines) {
            return (
                Category::PobRewritten,
                "mods.lua sets this mod's second stat to 2-12".to_owned(),
            );
        }
    }
    for (reason, category) in [
        ("unknown-handler", Category::PobIgnoresUnknownHandler),
        ("no-matching-variant", Category::PobZeroRangeRetry),
    ] {
        if with_pob_fallback(scopes, stats, rendered, reason).is_some_and(|l| equals(&l)) {
            let failed: Vec<&str> = rendered
                .not_rendered
                .iter()
                .filter(|(_, r)| r.name() == reason)
                .map(|(s, _)| s.as_str())
                .collect();
            return (
                category,
                format!(
                    "{reason} for {}; PoB's fallback gives its lines",
                    failed.join(", ")
                ),
            );
        }
    }
    let trimmed = texts.iter().any(|t| {
        let trimmed: Vec<String> = t.iter().map(|l| l.trim().to_owned()).collect();
        trimmed != *t && same_lines(&trimmed, &rendered.lines)
    });
    if trimmed {
        return (
            Category::PobUntrimmed,
            "PoB's lines, trimmed, equal ours".to_owned(),
        );
    }
    let lookup_ranges: Vec<&str> = rendered
        .not_rendered
        .iter()
        .filter(|(_, r)| matches!(r, NotRenderedReason::LookupRange(_)))
        .map(|(s, _)| s.as_str())
        .collect();
    if !lookup_ranges.is_empty()
        && texts
            .iter()
            .any(|t| t.len() > rendered.lines.len() && rendered.lines.iter().all(|l| t.contains(l)))
    {
        return (
            Category::PobLookupRange,
            format!("lookup stats with a range: {}", lookup_ranges.join(", ")),
        );
    }
    for d in &rendered.described {
        let Some(theirs) = pob.stat_texts.get(&d.stats.join(" ")) else {
            continue;
        };
        let ours: BTreeSet<&str> = d.variants.iter().map(|v| v.text.as_str()).collect();
        let changed: Vec<&String> = theirs
            .iter()
            .filter(|t| !ours.contains(t.as_str()))
            .collect();
        if !changed.is_empty() {
            return (
                Category::PatchDelta,
                format!(
                    "PoB's exported description of {} has {:?}, not in the extract",
                    d.stats.join(" "),
                    changed
                ),
            );
        }
    }
    (Category::Uncategorized, String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_should_read_text_lines_after_type_and_affix() {
        let line = "Strength1\"] = { type = \"Suffix\", affix = \"of the Brute\", \"+(8-12) to Strength\", \"Second\", statOrder = { 1 }, level = 1 },";
        let (id, lines) = entry(line).unwrap();
        assert_eq!(id, "Strength1");
        assert_eq!(lines, vec!["+(8-12) to Strength", "Second"]);
        let no_type = "X\"] = { affix = \"\", \"Line\", statOrder = { 1 } },";
        assert_eq!(entry(no_type).unwrap().1, vec!["Line"]);
        assert!(entry("Armoured\"] = {").is_none());
        assert!(entry("Y\"] = { affix = \"\", statOrder = { 1 } },").is_none());
    }

    #[test]
    fn add_mod_file_should_count_entries_and_unparsed_lines() {
        let mut pob = Pob::default();
        pob.add_mod_file(
            "ModA.lua",
            "return {\n\t[\"A\"] = { affix = \"\", \"x\", statOrder = { 1 } },\n\t[\"B\"] = {\n\t\t[\"C\"] = {\n\t[\"A\"] = { affix = \"\", \"x\", statOrder = { 2 } },\n}",
        );
        assert_eq!(pob.files.get("ModA.lua"), Some(&2));
        assert_eq!(pob.unparsed, 1);
        assert_eq!(pob.texts.get("A"), Some(&vec![vec!["x".to_owned()]]));
    }

    #[test]
    fn add_stat_file_should_collect_texts_per_stat_list() {
        let text = "return {\n\t[1]={\n\t\t[1]={\n\t\t\t[1]={\n\t\t\t\tlimit={\n\t\t\t\t\t[1]={\n\t\t\t\t\t\t[1]=1,\n\t\t\t\t\t\t[2]=\"#\"\n\t\t\t\t\t}\n\t\t\t\t},\n\t\t\t\ttext=\"{0}% increased X\"\n\t\t\t},\n\t\t\t[2]={\n\t\t\t\ttext=\"{0}% reduced X\"\n\t\t\t}\n\t\t},\n\t\tstats={\n\t\t\t[1]=\"a\",\n\t\t\t[2]=\"b\"\n\t\t}\n\t},\n\t[2]={\n\t\t[1]={\n\t\t\t[1]={\n\t\t\t\ttext=\"Other\"\n\t\t\t}\n\t\t},\n\t\tstats={\n\t\t\t[1]=\"c\"\n\t\t}\n\t}\n}";
        let mut pob = Pob::default();
        pob.add_stat_file("s.lua", text);
        assert_eq!(pob.stat_files.get("s.lua"), Some(&2));
        assert_eq!(
            pob.stat_texts.get("a b"),
            Some(&BTreeSet::from([
                "{0}% increased X".to_owned(),
                "{0}% reduced X".to_owned()
            ]))
        );
        assert_eq!(
            pob.stat_texts.get("c"),
            Some(&BTreeSet::from(["Other".to_owned()]))
        );
    }

    #[test]
    fn match_limit_should_test_minimums_and_skip_table_only_variants() {
        use loom_data::stat_desc::{Template, Variant};
        let v = |min: Option<i64>, table_only: bool| Variant {
            conditions: vec![Condition::Range { min, max: None }],
            gem_quality: false,
            table_only,
            unknown_flags: Vec::new(),
            handlers: Vec::new(),
            line: 1,
            text: "{0}".to_owned(),
            template: Template::parse("{0}", 1),
        };
        let d = Description {
            name: None,
            line: 1,
            stats: vec!["s".to_owned()],
            off_hand_stats: None,
            variants: vec![v(Some(1), true), v(Some(1), false)],
        };
        assert_eq!(match_limit(&d, &[StatValue { min: 1, max: 5 }]), Some(1));
        assert_eq!(match_limit(&d, &[StatValue { min: 0, max: 5 }]), None);
    }
}
