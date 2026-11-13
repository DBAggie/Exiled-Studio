//! The mod-row filter for ambiguous lines and the `mod-backed` measure
//! (Phase 2 plan, step 2c; lifted from Spike B's `mod_filter.rs`, PRs B4a,
//! B4y, B5a, step 4d).
//!
//! Every game-specific number (mod domains, generation types, influences)
//! and name (line flags, carrier tags, sections) is per-game data in
//! [`ModRules`] (plan decision 5); the rules a game's data bundle gives
//! decide which rows fit which lines. The Spike B module docs
//! (`tools/parity-cli/src/mod_filter.rs`) record the PoE1 rules and the
//! evidence for each step.
//!
//! **Fit.** A row fits a line on an item when it fits one of the base's
//! rows: the first [`FitRule`] whose section, line flag, and rarity match
//! the line decides, and the row fits when one of the rule's
//! [`FitClause`]s holds. A line no rule matches fits no row.
//!
//! **Backing.** A row *backs* a candidate when at least one candidate stat
//! is on the row and every candidate stat that is not on the row admits 0.
//! Row stats with `(0, 0)` are not on the row. *Equal* when every row stat
//! is a candidate stat; *hybrid* otherwise. At the *values* level, every
//! candidate stat on the row also overlaps the row's `[min, max]`.
//!
//! **Filter** of an ambiguous line, each step applied only when it keeps
//! at least one candidate and removes at least one: `mods`, then (none
//! backed) `unset-fit` and `numbered-stat`; else `base-implicit`,
//! `local-stat`, `spawn-tags`, `values`, `spawn-values` ([`filter`]).

use std::collections::{BTreeMap, BTreeSet};

use loom_data::stat_desc::Condition;
use loom_domain::{ModSection, ModifierLine};

use super::base::BaseRow;
use crate::resolve::{Candidate, RawValue, ResolvedStat};

pub use loom_data::rules::{
    DomainMatch, FitClause, FitRule, LocalCarriers, ModRules, ModRulesError, Unflagged,
};

/// Whether `line` is one of the lines `u` names: its section, and none of
/// its flags.
fn unflagged(u: &Unflagged, line: &LineContext) -> bool {
    line.section == u.section && !u.flags.iter().any(|f| line.has(f))
}

/// One mod row, as the filter needs it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModRow {
    /// Row ID.
    pub id: String,
    /// Domain.
    pub domain: i64,
    /// Generation type.
    pub generation: i64,
    /// `(stat, min, max)` with `min <= max`; stats with `(0, 0)` left out.
    pub stats: Vec<(String, i64, i64)>,
    /// Stats with `(0, 0)`: absent, or a value set outside the row; only
    /// used to name why a line is not backed.
    pub unset: Vec<String>,
    /// `(tag, weight)` in row order.
    pub spawn: Vec<(String, i64)>,
    /// Influence value; `None` for no influence.
    pub influence: Option<i64>,
    /// Crafting class restriction (item class rows).
    pub craft_classes: Vec<i64>,
    /// Whether only an essence puts the row on an item.
    pub essence_only: bool,
    /// Item class rows an essence puts this row on; `None` when unknown.
    pub essence_classes: Option<Vec<i64>>,
    /// Mod type row.
    pub mod_type: Option<i64>,
    /// For an essence-only row with unknown classes: the tags on which a
    /// row of the same mod type (domain, generation type) that is not
    /// essence-only has a positive spawn weight; `None` when the mod type
    /// has no such row.
    pub type_spawn: Option<Vec<String>>,
}

impl ModRow {
    /// The spawn weight on an item with `tags`: the first row tag the item
    /// has gives it.
    #[must_use]
    pub fn weight(&self, tags: &[String]) -> i64 {
        self.spawn
            .iter()
            .find(|(t, _)| tags.contains(t))
            .map_or(0, |(_, w)| *w)
    }

    /// Whether an essence can put this row on `base`: `true` when the
    /// essence classes or the base's class are unknown.
    fn essence_on(&self, base: &BaseRow) -> bool {
        self.essence_classes
            .as_ref()
            .is_none_or(|c| base.class.is_none_or(|k| c.contains(&k)))
    }

    /// Whether an essence with known classes puts this row on `base`.
    fn essence_known_on(&self, base: &BaseRow) -> bool {
        self.essence_classes
            .as_ref()
            .is_some_and(|c| base.class.is_some_and(|k| c.contains(&k)))
    }

    /// Whether this is a legacy essence row (essence-only, classes unknown)
    /// whose mod type spawns on one of `tags`.
    #[must_use]
    pub fn type_spawns_on(&self, tags: &[String]) -> bool {
        self.essence_only
            && self.essence_classes.is_none()
            && self
                .type_spawn
                .as_ref()
                .is_some_and(|t| t.iter().any(|x| tags.contains(x)))
    }

    /// Whether this is a legacy essence row whose classes stay unknown.
    #[must_use]
    pub fn essence_unknown(&self) -> bool {
        self.essence_only && self.essence_classes.is_none() && self.type_spawn.is_none()
    }

    fn range(&self, stat: &str) -> Option<(i64, i64)> {
        self.stats
            .iter()
            .find(|(s, _, _)| s == stat)
            .map(|(_, lo, hi)| (*lo, *hi))
    }

    /// `Some(true)` equal, `Some(false)` hybrid, `None` when the row does
    /// not back `stats` (module docs).
    #[must_use]
    pub fn backs(&self, stats: &[ResolvedStat]) -> Option<bool> {
        let mut on = 0;
        for s in stats {
            if self.range(&s.stat).is_some() {
                on += 1;
            } else if !admits(&s.value, 0, 0) {
                return None;
            }
        }
        (on > 0).then(|| {
            self.stats
                .iter()
                .all(|(r, _, _)| stats.iter().any(|s| s.stat == *r))
        })
    }

    /// Whether every candidate stat on the row overlaps the row's range.
    #[must_use]
    pub fn values_overlap(&self, stats: &[ResolvedStat]) -> bool {
        stats.iter().all(|s| {
            self.range(&s.stat)
                .is_none_or(|(lo, hi)| admits(&s.value, lo, hi))
        })
    }
}

/// Whether a value read from a line allows some raw value in `[lo, hi]`.
fn admits(value: &RawValue, lo: i64, hi: i64) -> bool {
    match value {
        RawValue::Exact(s) | RawValue::Ranged(s, _) => s.min <= hi && lo <= s.max,
        RawValue::Hidden { condition, .. } => match *condition {
            Condition::Range { min, max } => {
                min.map_or(lo, |m| m.max(lo)) <= max.map_or(hi, |m| m.min(hi))
            }
            Condition::Not(v) => !(lo == hi && lo == v),
        },
        RawValue::Lookup(keys) => keys.iter().any(|k| (lo..=hi).contains(k)),
    }
}

/// Which item property a local stat applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Locality {
    /// The item's weapon properties.
    Weapon,
    /// The item's own other properties (defences).
    Item,
}

impl Locality {
    /// The base tag of items that carry the property.
    #[must_use]
    pub fn carrier_tag(self, carriers: &LocalCarriers) -> &str {
        match self {
            Self::Weapon => &carriers.weapon,
            Self::Item => &carriers.item,
        }
    }
}

/// The local stats of one candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalKind {
    /// No local stat.
    Global,
    /// Local stats of one locality.
    Local(Locality),
    /// Local stats of both localities.
    Mixed,
}

/// Mod rows, indexed by stat.
#[derive(Debug, Default)]
pub struct Mods {
    rows: Vec<ModRow>,
    by_stat: BTreeMap<String, Vec<usize>>,
    by_unset: BTreeMap<String, Vec<usize>>,
    /// Local stats, by ID.
    pub locality: BTreeMap<String, Locality>,
    /// Rows whose spawn tag and weight lists differ in length (read with
    /// no spawn weights; the loader counts them).
    pub bad_spawn_rows: usize,
}

/// Sets `type_spawn` on the legacy essence rows (essence-only, classes
/// unknown): the tags on which the rows of the same mod type, domain, and
/// generation type that are not essence-only spawn. A mod type with such
/// rows and no positive weight gives an empty list: the row spawns nowhere.
fn set_type_spawn(rows: &mut [ModRow]) {
    let mut by_type: BTreeMap<(i64, i64, i64), BTreeSet<String>> = BTreeMap::new();
    for r in rows.iter().filter(|r| !r.essence_only) {
        if let Some(t) = r.mod_type {
            by_type
                .entry((t, r.domain, r.generation))
                .or_default()
                .extend(
                    r.spawn
                        .iter()
                        .filter(|(_, w)| *w > 0)
                        .map(|(tag, _)| tag.clone()),
                );
        }
    }
    for r in rows
        .iter_mut()
        .filter(|r| r.essence_only && r.essence_classes.is_none())
    {
        r.type_spawn = r
            .mod_type
            .and_then(|t| by_type.get(&(t, r.domain, r.generation)))
            .map(|tags| tags.iter().cloned().collect());
    }
}

impl Mods {
    /// Indexes rows (row `n` keeps index `n`, as base implicits name it)
    /// and sets `type_spawn`. Rows keep their `essence_classes`.
    #[must_use]
    pub fn from_rows(mut rows: Vec<ModRow>) -> Self {
        set_type_spawn(&mut rows);
        fn add(index: &mut BTreeMap<String, Vec<usize>>, stat: &str, n: usize) {
            let list = index.entry(stat.to_owned()).or_default();
            if list.last() != Some(&n) {
                list.push(n);
            }
        }
        let (mut by_stat, mut by_unset) = (BTreeMap::new(), BTreeMap::new());
        for (n, row) in rows.iter().enumerate() {
            for (stat, _, _) in &row.stats {
                add(&mut by_stat, stat, n);
            }
            for stat in &row.unset {
                add(&mut by_unset, stat, n);
            }
        }
        Self {
            rows,
            by_stat,
            by_unset,
            locality: BTreeMap::new(),
            bad_spawn_rows: 0,
        }
    }

    /// Whether a candidate's stats are global, local of one locality, or
    /// local of two.
    fn local_kind(&self, stats: &[ResolvedStat]) -> LocalKind {
        let mut kinds = stats.iter().filter_map(|s| self.locality.get(&s.stat));
        let Some(first) = kinds.next().copied() else {
            return LocalKind::Global;
        };
        if kinds.any(|k| *k != first) {
            return LocalKind::Mixed;
        }
        LocalKind::Local(first)
    }

    /// Essence-only rows whose item classes are known, and the rest.
    #[must_use]
    pub fn essence_rows(&self) -> (usize, usize) {
        let only = self.rows.iter().filter(|r| r.essence_only);
        let known = only.clone().filter(|r| r.essence_classes.is_some()).count();
        (known, only.count() - known)
    }

    /// Number of rows.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Rows carrying `stat` at a value other than `(0, 0)`, in row order.
    pub fn rows_with_stat(&self, stat: &str) -> impl Iterator<Item = &ModRow> {
        self.by_stat
            .get(stat)
            .into_iter()
            .flatten()
            .filter_map(|n| self.rows.get(*n))
    }

    /// Rows with at least one of `stats`, in row order.
    /// Whether `stats` are global (no stat is local to an item) and a row
    /// of one of `domains` backs them (module docs, "Backing"): the
    /// custom-line rule ([`crate::import`], P3-12j). A custom line acts on
    /// the character, so a local reading is never its reading.
    #[must_use]
    pub fn backed_in(&self, stats: &[ResolvedStat], domains: &BTreeSet<i64>) -> bool {
        stats.iter().all(|s| !self.locality.contains_key(&s.stat))
            && self
                .rows_for(stats)
                .into_iter()
                .filter_map(|n| self.row(n))
                .any(|r| domains.contains(&r.domain) && r.backs(stats).is_some())
    }

    fn rows_for(&self, stats: &[ResolvedStat]) -> BTreeSet<usize> {
        stats
            .iter()
            .filter_map(|s| self.by_stat.get(&s.stat))
            .flatten()
            .copied()
            .collect()
    }

    /// Row `n`.
    #[must_use]
    pub fn row(&self, n: usize) -> Option<&ModRow> {
        self.rows.get(n)
    }

    /// Rows that carry one of `stats` at `(0, 0)`.
    fn unset_rows(&self, stats: &[ResolvedStat]) -> impl Iterator<Item = &ModRow> {
        stats
            .iter()
            .filter_map(|s| self.by_unset.get(&s.stat))
            .flatten()
            .filter_map(|n| self.rows.get(*n))
    }
}

/// What the filter knows about an item.
pub struct Context<'a> {
    /// The game's filter rules.
    pub rules: &'a ModRules,
    /// Mod rows.
    pub mods: &'a Mods,
    /// Every base row with the base's name.
    pub bases: &'a [BaseRow],
    /// Unique (or no rarity, which the import adapter reads as unique).
    pub unique: bool,
    /// Influence values of the item's influences.
    pub influences: Vec<i64>,
}

/// What the filter knows about a line (the first of a multi-line run).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineContext {
    /// Section.
    pub section: ModSection,
    /// Line flags and prefix keys.
    pub flags: Vec<String>,
}

impl LineContext {
    /// The context of a modifier line.
    #[must_use]
    pub fn of(m: &ModifierLine) -> Self {
        Self {
            section: m.section.clone(),
            flags: m.prefixes.iter().map(|p| p.key.clone()).collect(),
        }
    }

    /// Whether the line carries `flag`.
    #[must_use]
    pub fn has(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f == flag)
    }
}

/// Whether `row` fits a line of `line` on an item with base row `base`
/// (module docs).
#[must_use]
pub fn fits(
    rules: &ModRules,
    row: &ModRow,
    base: &BaseRow,
    unique: bool,
    line: &LineContext,
) -> bool {
    let Some(rule) = rules.fits.iter().find(|r| {
        r.section == line.section
            && r.flag.as_deref().is_none_or(|f| line.has(f))
            && r.unique.is_none_or(|u| u == unique)
    }) else {
        return false;
    };
    rule.clauses.iter().any(|c| {
        c.generations.contains(&row.generation)
            && c.domains.admits(row.domain, base.domain)
            && (!c.craft_class
                || row.craft_classes.is_empty()
                || base.class.is_some_and(|k| row.craft_classes.contains(&k)))
            && (!c.essence_class || !(row.essence_only && !row.essence_on(base)))
    })
}

/// A row that backs a candidate and fits one of the base rows.
#[derive(Debug, Clone, Copy)]
struct Fit {
    row: usize,
    equal: bool,
}

impl Context<'_> {
    fn fits(&self, row: &ModRow, line: &LineContext) -> bool {
        self.bases
            .iter()
            .any(|b| fits(self.rules, row, b, self.unique, line))
    }

    fn fitting(&self, line: &LineContext, stats: &[ResolvedStat]) -> Vec<Fit> {
        let mut out = Vec::new();
        for n in self.mods.rows_for(stats) {
            let Some(row) = self.mods.row(n) else {
                continue;
            };
            let Some(equal) = row.backs(stats) else {
                continue;
            };
            if self.fits(row, line) {
                out.push(Fit { row: n, equal });
            }
        }
        out
    }

    fn row(&self, fit: Fit) -> Option<&ModRow> {
        self.mods.row(fit.row)
    }

    /// Whether some fitting row spawns on one of the base rows: a positive
    /// weight on its tags, or an essence with known classes puts the row on
    /// the base's class. Rows of the excluded generations do not count on
    /// the lines [`ModRules::spawn_exclusion`] names.
    fn spawns(&self, line: &LineContext, fits_: &[Fit]) -> bool {
        fits_
            .iter()
            .any(|f| self.row(*f).is_some_and(|row| self.row_spawns(line, row)))
    }

    /// Whether one fitting row spawns on one of the base rows
    /// ([`Self::spawns`]). A legacy essence row counts where its mod type
    /// spawns.
    fn row_spawns(&self, line: &LineContext, row: &ModRow) -> bool {
        let excluded = unflagged(&self.rules.spawn_exclusion, line)
            && self
                .rules
                .spawn_excluded_generations
                .contains(&row.generation);
        !excluded
            && self.bases.iter().any(|b| {
                fits(self.rules, row, b, self.unique, line)
                    && (b
                        .tags
                        .as_ref()
                        .is_some_and(|t| row.weight(t) > 0 || row.type_spawns_on(t))
                        || row.essence_known_on(b))
            })
    }

    /// Whether some fitting row is essence-only with unknown classes.
    fn essence_backed(&self, fits_: &[Fit]) -> bool {
        fits_
            .iter()
            .any(|f| self.row(*f).is_some_and(ModRow::essence_unknown))
    }

    /// Whether some fitting row is an influence row of one of the item's
    /// influences that spawns somewhere.
    fn influence_backed(&self, fits_: &[Fit]) -> bool {
        fits_.iter().any(|f| {
            self.row(*f).is_some_and(|row| {
                row.influence.is_some_and(|i| self.influences.contains(&i))
                    && row.spawn.iter().any(|(_, w)| *w > 0)
            })
        })
    }

    /// Whether a fitting row carries one of `stats` at `(0, 0)`.
    fn unset_fit(&self, line: &LineContext, stats: &[ResolvedStat]) -> bool {
        self.mods.unset_rows(stats).any(|row| self.fits(row, line))
    }

    fn base_implicit(&self, stats: &[ResolvedStat]) -> bool {
        self.bases
            .iter()
            .flat_map(|b| &b.implicits)
            .filter_map(|n| self.mods.row(*n))
            .any(|row| row.backs(stats).is_some())
    }
}

/// How the filter decided an ambiguous line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Decision {
    /// Indexes of the candidates kept, in input order.
    pub kept: Vec<usize>,
    /// Steps that removed candidates, in order.
    pub rules: Vec<&'static str>,
    /// Why a step was skipped, or what a step removed that is worth a look.
    pub notes: BTreeSet<&'static str>,
    /// Whether any candidate was backed by a fitting row.
    pub backed_any: bool,
}

impl Decision {
    /// `resolved/<rules>` when one candidate is left, else
    /// `ambiguous/no-backed-candidate` or `ambiguous/several-backed`.
    #[must_use]
    pub fn name(&self) -> String {
        if self.kept.len() == 1 {
            format!("resolved/{}", self.rules.join("+"))
        } else if self.backed_any {
            "ambiguous/several-backed".to_owned()
        } else {
            "ambiguous/no-backed-candidate".to_owned()
        }
    }
}

fn narrow(
    kept: &mut Vec<usize>,
    rules: &mut Vec<&'static str>,
    rule: &'static str,
    keep: impl Fn(usize) -> bool,
) {
    let next: Vec<usize> = kept.iter().copied().filter(|i| keep(*i)).collect();
    if !next.is_empty() && next.len() < kept.len() {
        *kept = next;
        rules.push(rule);
    }
}

/// Filters the candidates of an ambiguous line (module docs).
#[must_use]
pub fn filter(ctx: &Context<'_>, line: &LineContext, candidates: &[Candidate]) -> Decision {
    let fit: Vec<Vec<Fit>> = candidates
        .iter()
        .map(|c| ctx.fitting(line, &c.stats))
        .collect();
    let mut d = Decision {
        kept: (0..candidates.len()).collect(),
        backed_any: fit.iter().any(|f| !f.is_empty()),
        ..Decision::default()
    };
    if !d.backed_any {
        let unset: Vec<bool> = candidates
            .iter()
            .map(|c| ctx.unset_fit(line, &c.stats))
            .collect();
        if unset.contains(&true) {
            d.notes.insert("row-value-unset");
            narrow(&mut d.kept, &mut d.rules, "unset-fit", |i| {
                unset.get(i).copied().unwrap_or(false)
            });
            numbered_stat(&mut d, candidates);
        }
        return d;
    }
    let has_fit = |i: usize| fit.get(i).is_some_and(|f| !f.is_empty());
    narrow(&mut d.kept, &mut d.rules, "mods", has_fit);

    if d.kept.len() > 1 && unflagged(&ctx.rules.base_implicit, line) {
        narrow(&mut d.kept, &mut d.rules, "base-implicit", |i| {
            candidates
                .get(i)
                .is_some_and(|c| ctx.base_implicit(&c.stats))
        });
    }

    if d.kept.len() > 1 && line.section == ModSection::Explicit && !ctx.unique {
        local_stat(ctx, line, candidates, &fit, &mut d);
    }

    // Whether `spawn-tags` ran (no guard skipped it); `spawn-values` needs it.
    let mut spawn_known = false;
    if d.kept.len() > 1 {
        let spawns = |i: usize| fit.get(i).is_some_and(|f| ctx.spawns(line, f));
        if ctx.rules.no_spawn_flags.iter().any(|f| line.has(f)) {
            d.notes.insert("crafted-no-spawn-tags");
        } else if ctx.bases.iter().all(|b| b.tags.is_none()) {
            d.notes.insert("base-tags-unknown");
        } else if !ctx.influences.is_empty()
            && d.kept
                .iter()
                .any(|&i| !spawns(i) && fit.get(i).is_some_and(|f| ctx.influence_backed(f)))
        {
            d.notes.insert("influence-tags-unknown");
        } else if d
            .kept
            .iter()
            .any(|&i| !spawns(i) && fit.get(i).is_some_and(|f| ctx.essence_backed(f)))
        {
            d.notes.insert("essence-tags-unknown");
        } else {
            spawn_known = true;
            narrow(&mut d.kept, &mut d.rules, "spawn-tags", spawns);
        }
    }

    if d.kept.len() > 1 {
        narrow(&mut d.kept, &mut d.rules, "values", |i| {
            let (Some(c), Some(f)) = (candidates.get(i), fit.get(i)) else {
                return false;
            };
            f.iter()
                .any(|x| ctx.row(*x).is_some_and(|r| r.values_overlap(&c.stats)))
        });
    }

    if d.kept.len() > 1 && spawn_known {
        narrow(&mut d.kept, &mut d.rules, "spawn-values", |i| {
            let (Some(c), Some(f)) = (candidates.get(i), fit.get(i)) else {
                return false;
            };
            f.iter().any(|x| {
                ctx.row(*x)
                    .is_some_and(|r| r.values_overlap(&c.stats) && ctx.row_spawns(line, r))
            })
        });
    }
    d
}

/// `local-stat`: on an explicit line of a non-unique item whose kept
/// candidates are local and global readings of one line, the base decides:
/// a base with the locality's carrier tag keeps the local readings, any
/// other base the global ones. Skipped, with a note, when the base's tags
/// are unknown or disagree, when a candidate mixes localities, or when a
/// global base would drop a local reading that a fitting row spawns on the
/// base.
fn local_stat(
    ctx: &Context<'_>,
    line: &LineContext,
    candidates: &[Candidate],
    fit: &[Vec<Fit>],
    d: &mut Decision,
) {
    let kinds: Vec<LocalKind> = d
        .kept
        .iter()
        .map(|&i| {
            candidates
                .get(i)
                .map_or(LocalKind::Global, |c| ctx.mods.local_kind(&c.stats))
        })
        .collect();
    if kinds.contains(&LocalKind::Mixed) {
        d.notes.insert("local-stat-mixed");
        return;
    }
    let locals: BTreeSet<Locality> = kinds
        .iter()
        .filter_map(|k| match k {
            LocalKind::Local(l) => Some(*l),
            _ => None,
        })
        .collect();
    let [kind] = locals.iter().copied().collect::<Vec<_>>()[..] else {
        return;
    };
    if !kinds.contains(&LocalKind::Global) {
        return;
    }
    // Every base row must have known tags and agree on the carrier tag.
    let tag = kind.carrier_tag(&ctx.rules.local_carriers);
    let carrier: BTreeSet<Option<bool>> = ctx
        .bases
        .iter()
        .map(|b| b.tags.as_ref().map(|t| t.iter().any(|x| x == tag)))
        .collect();
    let carries = match carrier.iter().copied().collect::<Vec<_>>()[..] {
        [Some(c)] => c,
        [None] | [] => {
            d.notes.insert("local-stat-tags-unknown");
            return;
        }
        _ => {
            d.notes.insert("local-stat-bases-differ");
            return;
        }
    };
    let local_at = |i: usize| {
        candidates
            .get(i)
            .is_some_and(|c| matches!(ctx.mods.local_kind(&c.stats), LocalKind::Local(_)))
    };
    if !carries
        && d.kept
            .iter()
            .any(|&i| local_at(i) && fit.get(i).is_some_and(|f| ctx.spawns(line, f)))
    {
        d.notes.insert("local-stat-spawns");
        return;
    }
    narrow(&mut d.kept, &mut d.rules, "local-stat", |i| {
        local_at(i) == carries
    });
}

/// A stat ID without a numbered suffix `_2` .. `_9` (`x_3` to `(x, 3)`);
/// other IDs unchanged, numbered 1.
#[must_use]
pub fn unnumbered(stat: &str) -> (&str, u8) {
    if let Some((base, n)) = stat.rsplit_once('_')
        && let [d @ b'2'..=b'9'] = n.as_bytes()
    {
        return (base, d - b'0');
    }
    (stat, 1)
}

/// `numbered-stat`, run only after `unset-fit`: when every kept candidate
/// has the same stats up to a numbered suffix, they are one reading; the
/// candidate with the lowest numbers is kept, the others are dropped, and
/// the note `numbered-stat-equivalents` is set.
pub fn numbered_stat(d: &mut Decision, candidates: &[Candidate]) {
    if d.kept.len() < 2 {
        return;
    }
    let shape = |i: usize| -> Option<(Vec<&str>, Vec<u8>)> {
        let c = candidates.get(i)?;
        Some(c.stats.iter().map(|s| unnumbered(&s.stat)).unzip())
    };
    let shapes: Option<Vec<(Vec<&str>, Vec<u8>)>> = d.kept.iter().map(|i| shape(*i)).collect();
    let Some(shapes) = shapes else {
        return;
    };
    let Some((first, first_numbers)) = shapes.first() else {
        return;
    };
    let numbers_differ = shapes.iter().any(|(_, n)| n != first_numbers);
    if !numbers_differ || !shapes.iter().all(|(s, _)| s == first) {
        return;
    }
    let best = shapes
        .iter()
        .zip(&d.kept)
        .min_by(|(a, _), (b, _)| a.1.cmp(&b.1))
        .map(|(_, i)| *i);
    if let Some(best) = best {
        d.kept = vec![best];
        d.rules.push("numbered-stat");
        d.notes.insert("numbered-stat-equivalents");
    }
}

/// `mod-backed` of a resolved line: `Ok((true, id))` backed by an equal
/// row, `Ok((false, id))` only by hybrid rows (`id`: the first such row's
/// ID), or why it is not backed: `row-value-unset` (a fitting row carries
/// a stat of the line at `(0, 0)`; checked first), `no-row-with-stats`,
/// `no-fitting-row`, `values-outside`.
///
/// # Errors
/// The reason, as above.
pub fn backing<'m>(
    ctx: &Context<'m>,
    line: &LineContext,
    stats: &[ResolvedStat],
) -> Result<(bool, &'m str), &'static str> {
    let backed = backing_row(ctx, line, stats);
    if backed.is_err() && ctx.unset_fit(line, stats) {
        return Err("row-value-unset");
    }
    backed
}

fn backing_row<'m>(
    ctx: &Context<'m>,
    line: &LineContext,
    stats: &[ResolvedStat],
) -> Result<(bool, &'m str), &'static str> {
    let any_row = ctx
        .mods
        .rows_for(stats)
        .into_iter()
        .filter_map(|n| ctx.mods.row(n))
        .any(|r| r.backs(stats).is_some());
    if !any_row {
        return Err("no-row-with-stats");
    }
    let fits_ = ctx.fitting(line, stats);
    if fits_.is_empty() {
        return Err("no-fitting-row");
    }
    let mods: &'m Mods = ctx.mods;
    fits_
        .iter()
        .filter_map(|f| mods.row(f.row).map(|r| (f.equal, r)))
        .filter(|(_, r)| r.values_overlap(stats))
        // Equal rows first, then row order.
        .min_by_key(|(equal, _)| !equal)
        .map(|(equal, r)| (equal, r.id.as_str()))
        .ok_or("values-outside")
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]
mod tests;
