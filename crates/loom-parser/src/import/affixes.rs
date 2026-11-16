//! Which mod made which line, for the item tooltip's Alt view (UX sprint,
//! B-2; `dev/active/ux-improvements/b2-item-data-plan.md`).
//!
//! **Display only.** Nothing here feeds line resolution or the
//! calculation: a line no mod is found for keeps its text and is shown
//! without a header (ADR-015: never dropped).
//!
//! **Named mods (PR 1).** PoB writes an item's affixes as spec lines,
//! `Prefix: {range:0.718}LocalBaseArmourAndEnergyShield4` (`None` for an
//! open affix). A spec is used only when its mod is in the bundle and its
//! stats account for lines of the item: an explicit, non-crafted line all
//! of whose shown stats are the mod's, with values in its ranges. Lines may
//! be anywhere in the item (the game prints lines in stat-description
//! order, not mod order). A line two named mods both carry (the game sums
//! them into one line: a hybrid and a pure "increased Armour") goes under
//! both when its value fits their summed ranges, else under neither. A spec
//! whose mod accounts for no line (unknown ID, a line edited by hand) is
//! dropped. The kind is PoB's key.
//!
//! **Inferred mods (PR 2).** The lines no named mod took are explained by
//! the mod rows that could be on the item: rows that fit the line on the
//! base ([`super::mods::fits`]) and can appear on it (spawn on the base's
//! tags, an essence for its class, one of the item's influences; crafted
//! lines: any fitting bench row), every stat of the row shown on one of the
//! lines or carried by one of them with no shown value (a hidden stat), and
//! of a mod type no named mod holds (an item has one mod of a type). The
//! lines split into groups no row spans; for each group every set of rows
//! of distinct mod types whose summed ranges hold each line's shown values
//! is an *explanation*. A row gets a header only when it is in every
//! explanation of its group: one reading or none. A group with no
//! explanation, or more than the search allows, gets no headers. Unique
//! items get none (their lines are the unique's own). The kind of an
//! explicit row comes from the game's rules ([`ModRules::affix_generations`]);
//! rules without them infer no explicit mod. Not checked: the number of
//! prefixes and suffixes an item may have (a per-rarity rule the data does
//! not carry); leaving it out only adds explanations, so it can take a
//! header away, never add a wrong one.
//!
//! **Implicits.** Unflagged implicit lines are explained the same way from
//! the base's own implicit rows and the implicit rows that spawn on it
//! (on a unique, the base's rows only), and headed `Implicit Modifier`.
//!
//! **Tier.** The game's tier of a prefix or suffix on a base: 1 plus the
//! rows of the same mod type, domain, and generation type with a higher
//! level that spawn on the base's tags (no essence or influence rows).
//! Checked against the game: Urchin's (`LocalBaseArmourAndLife2`, level
//! 46) on a Pig-Faced Bascinet is Tier 2, under Nautilus's (62);
//! Crocodile's (78) does not spawn on helmets. A row that does not spawn on
//! the base by its tags (an essence, an influence, a crafted row) has no
//! tier.
//!
//! The affix name, level, and tags come from a data schema 4 bundle
//! ([`bundle::AFFIX_SCHEMA`]); an older bundle gives the header without
//! them, and no tier. Tag names: the game's text rules
//! ([`loom_data::rules::TextRules::mod_tag_names`]) when it has them, else
//! the tag table's display names.

use std::collections::{BTreeMap, BTreeSet};

use loom_data::bundle::{self, Bundle, ModEntry};
use loom_data::rules::ModRules;
use loom_domain::{
    ItemBody, LineKind, LineResolution, ModSection, ModifierLine, Resolved, StatValue,
};

use super::base::BaseRow;
use super::bundle::BundleData;
use super::mods::{Context, LineContext, ModRow};

/// Where an affix is on the item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AffixKind {
    /// An implicit modifier.
    Implicit,
    /// A prefix.
    Prefix,
    /// A suffix.
    Suffix,
}

/// One mod of an item and the lines it made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Affix {
    /// Implicit, prefix, or suffix.
    pub kind: AffixKind,
    /// Whether the mod is crafted (the game's `Master Crafted` header).
    pub crafted: bool,
    /// Whether the mod was inferred from the lines (module docs) rather
    /// than named by the item.
    pub inferred: bool,
    /// The mod's ID.
    pub mod_id: String,
    /// The affix name (`Urchin's`), when the bundle has one.
    pub name: Option<String>,
    /// The mod's level, when the bundle has it.
    pub level: Option<i64>,
    /// Its tier on the item's base (1 the best), when known (module docs).
    pub tier: Option<u32>,
    /// The mod's tags as the game prints them (`Life`, `Defences`).
    pub tags: Vec<String>,
    /// The item's line indices it made, in item order.
    pub lines: Vec<usize>,
}

/// The item's affixes PoB named, prefixes first, each in spec order
/// (module docs), with no tier.
#[must_use]
pub fn named_affixes(body: &ItemBody, bundle: &Bundle) -> Vec<Affix> {
    let mut out: Vec<Affix> = named(body, bundle)
        .into_iter()
        .map(|(kind, n, lines)| affix(bundle, kind, false, false, n, lines))
        .collect();
    out.sort_by_key(|a| a.kind);
    out
}

/// Every affix of the item the tooltip's Alt view heads (module docs):
/// the named ones, then the inferred ones, implicits first, then prefixes
/// and suffixes, crafted last within each; with tiers. `bases` are the
/// item's base rows; with none, only the named affixes, with no tier.
#[must_use]
pub fn item_affixes(
    body: &ItemBody,
    data: &BundleData<'_>,
    bases: Option<&[BaseRow]>,
) -> Vec<Affix> {
    let bundle = data.bundle();
    let named = named(body, bundle);
    let Some(bases) = bases.filter(|b| !b.is_empty()) else {
        return named_affixes(body, bundle);
    };
    let rules = &bundle.item_rules.mods;
    let ctx = Context {
        rules,
        mods: data.mods(),
        bases,
        unique: body.rarity.is_none_or(loom_domain::Rarity::is_unique),
        influences: body
            .influences()
            .filter_map(|id| rules.influence_value(id))
            .collect(),
    };
    let claimed: BTreeSet<usize> = named
        .iter()
        .flat_map(|(_, _, l)| l.iter().copied())
        .collect();
    // An item has one mod of a mod type: the named mods' types are taken.
    let taken: BTreeSet<i64> = named
        .iter()
        .filter_map(|(_, n, _)| ctx.mods.row(*n).and_then(|r| r.mod_type))
        .collect();
    let mut found: Vec<(AffixKind, bool, bool, usize, Vec<usize>)> = named
        .into_iter()
        .map(|(kind, n, lines)| (kind, false, false, n, lines))
        .collect();
    for pool in Pool::ALL {
        let lines: Vec<PoolLine<'_>> = body
            .lines
            .iter()
            .enumerate()
            .filter(|(i, _)| !claimed.contains(i))
            .filter_map(|(i, l)| match &l.kind {
                LineKind::Modifier(m) if pool.takes(m, rules, ctx.unique) => {
                    shown_stats(m).map(|stats| PoolLine { index: i, stats })
                }
                _ => None,
            })
            .collect();
        if lines.is_empty() {
            continue;
        }
        // Stats the pool's lines carry with no shown value: a row's stat
        // among them is on the item though no line shows it.
        let hidden: BTreeSet<&str> = body
            .lines
            .iter()
            .filter_map(|l| match &l.kind {
                LineKind::Modifier(m) if pool.takes(m, rules, ctx.unique) => Some(m),
                _ => None,
            })
            .flat_map(hidden_stats)
            .collect();
        let line_ctx = pool.line_context();
        let found_here = infer(
            &ctx,
            &Inputs {
                line: &line_ctx,
                pool,
                lines: &lines,
                hidden: &hidden,
                taken: &taken,
            },
        );
        for (row, lines) in found_here {
            let Some(kind) = pool.kind(rules, ctx.mods.row(row)) else {
                continue;
            };
            found.push((kind, pool == Pool::Crafted, true, row, lines));
        }
    }
    let with_data = bundle.manifest.data_schema >= bundle::AFFIX_SCHEMA;
    let mut out: Vec<Affix> = found
        .into_iter()
        .map(|(kind, crafted, inferred, n, lines)| {
            let mut a = affix(bundle, kind, crafted, inferred, n, lines);
            if with_data && !crafted && kind != AffixKind::Implicit {
                a.tier = tier(&ctx, bundle, n);
            }
            a
        })
        .collect();
    out.sort_by_key(|a| (a.kind, a.crafted));
    out
}

/// One affix's view of bundle row `n`.
fn affix(
    bundle: &Bundle,
    kind: AffixKind,
    crafted: bool,
    inferred: bool,
    n: usize,
    lines: Vec<usize>,
) -> Affix {
    let with_data = bundle.manifest.data_schema >= bundle::AFFIX_SCHEMA;
    let entry = bundle.mods.get(n);
    Affix {
        kind,
        crafted,
        inferred,
        mod_id: entry.map(|e| e.id.clone()).unwrap_or_default(),
        name: entry
            .filter(|e| with_data && !e.name.is_empty())
            .map(|e| e.name.clone()),
        level: entry.and_then(|e| e.level).filter(|_| with_data),
        tier: None,
        tags: match entry {
            Some(e) if with_data => tag_names(bundle, &e.implicit_tags),
            _ => Vec::new(),
        },
        lines,
    }
}

/// The names the Alt view shows for a mod's tags, in the mod's order: the
/// game's rules' names when it has them (tags not named there are not
/// shown), else the tag table's display names.
fn tag_names(bundle: &Bundle, tags: &[String]) -> Vec<String> {
    let names = &bundle.item_rules.text.mod_tag_names;
    tags.iter()
        .filter_map(|t| {
            if names.is_empty() {
                bundle
                    .tags
                    .iter()
                    .find(|n| n.id == *t)
                    .map(|n| n.display.clone())
            } else {
                names.iter().find(|(id, _)| id == t).map(|(_, n)| n.clone())
            }
        })
        .filter(|n| !n.is_empty())
        .collect()
}

/// The named specs with their bundle row and lines, in spec order (module
/// docs); specs that account for no line left out.
fn named(body: &ItemBody, bundle: &Bundle) -> Vec<(AffixKind, usize, Vec<usize>)> {
    let specs: Vec<(AffixKind, usize, &ModEntry)> = body
        .lines
        .iter()
        .filter_map(|l| match &l.kind {
            LineKind::Spec { key, value, .. } => {
                let kind = match key.as_str() {
                    "Prefix" => AffixKind::Prefix,
                    "Suffix" => AffixKind::Suffix,
                    _ => return None,
                };
                let id = mod_id(value)?;
                bundle
                    .mods
                    .iter()
                    .position(|m| m.id == id)
                    .and_then(|n| bundle.mods.get(n).map(|m| (kind, n, m)))
            }
            _ => None,
        })
        .collect();
    let mut lines: Vec<Vec<usize>> = vec![Vec::new(); specs.len()];
    for (index, line) in body.lines.iter().enumerate() {
        let LineKind::Modifier(m) = &line.kind else {
            continue;
        };
        if !m.selected
            || m.section != ModSection::Explicit
            || m.prefixes.iter().any(|p| p.key == CRAFTED)
        {
            continue;
        }
        let Some(shown) = shown_stats(m) else {
            continue;
        };
        let claimants: Vec<usize> = specs
            .iter()
            .enumerate()
            .filter(|(_, (_, _, entry))| {
                shown
                    .iter()
                    .all(|(stat, _)| entry.stats.iter().any(|s| s.stat == *stat))
            })
            .map(|(i, _)| i)
            .collect();
        if claimants.is_empty() {
            continue;
        }
        // Each shown stat's value overlaps the claimants' summed range.
        let fits = shown.iter().all(|(stat, (lo, hi))| {
            let (min, max) = claimants
                .iter()
                .filter_map(|i| specs.get(*i))
                .filter_map(|(_, _, entry)| entry.stats.iter().find(|s| s.stat == *stat))
                .fold((0, 0), |(a, b), s| (a + s.min, b + s.max));
            *lo <= max && *hi >= min
        });
        if fits {
            for i in claimants {
                if let Some(l) = lines.get_mut(i) {
                    l.push(index);
                }
            }
        }
    }
    specs
        .iter()
        .zip(lines)
        .filter(|(_, l)| !l.is_empty())
        .map(|((kind, n, _), lines)| (*kind, *n, lines))
        .collect()
}

/// The mod ID of a `Prefix:`/`Suffix:` value: after any `{...}` prefixes;
/// `None` for an open affix (`None`) or an empty value.
fn mod_id(value: &str) -> Option<&str> {
    let mut rest = value.trim();
    while let Some(after) = rest.strip_prefix('{') {
        rest = after.split_once('}')?.1.trim_start();
    }
    (!rest.is_empty() && rest != "None").then_some(rest)
}

/// A resolved line's shown stats with their value bounds (hidden and
/// lookup values left out); `None` for an unresolved line or one with no
/// shown stat.
fn shown_stats(m: &ModifierLine) -> Option<Vec<(&str, (i64, i64))>> {
    let LineResolution::Resolved {
        result: Resolved::Stats(stats),
        ..
    } = &m.resolution
    else {
        return None;
    };
    let shown: Vec<_> = stats
        .iter()
        .filter_map(|s| match s.value {
            StatValue::Exact { min, max } | StatValue::Ranged { min, max } => {
                Some((s.stat.as_str(), (min, max)))
            }
            StatValue::Hidden { .. } | StatValue::Lookup(_) => None,
        })
        .collect();
    (!shown.is_empty()).then_some(shown)
}

/// A resolved line's stats with no shown value (hidden or lookup values).
fn hidden_stats(m: &ModifierLine) -> Vec<&str> {
    let LineResolution::Resolved {
        result: Resolved::Stats(stats),
        ..
    } = &m.resolution
    else {
        return Vec::new();
    };
    stats
        .iter()
        .filter(|s| matches!(s.value, StatValue::Hidden { .. } | StatValue::Lookup(_)))
        .map(|s| s.stat.as_str())
        .collect()
}

/// The groups of lines inferred apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pool {
    /// Explicit lines with no flag.
    Explicit,
    /// Crafted explicit lines.
    Crafted,
    /// Implicit lines with no flag.
    Implicit,
}

/// The flag of crafted lines (PoB's `{crafted}`).
const CRAFTED: &str = "crafted";

impl Pool {
    const ALL: [Self; 3] = [Self::Implicit, Self::Explicit, Self::Crafted];

    /// Whether the pool takes line `m` (selected lines only; explicit lines
    /// of a unique item never; implicit lines with a flag of the rules'
    /// `base_implicit` lines, the eldritch ones, never).
    fn takes(self, m: &ModifierLine, rules: &ModRules, unique: bool) -> bool {
        if !m.selected {
            return false;
        }
        let has = |f: &str| m.prefixes.iter().any(|p| p.key == f);
        let crafted = has(CRAFTED);
        match self {
            Self::Explicit => m.section == ModSection::Explicit && !unique && !crafted,
            Self::Crafted => m.section == ModSection::Explicit && crafted,
            Self::Implicit => {
                m.section == ModSection::Implicit
                    && !rules.base_implicit.flags.iter().any(|f| has(f))
            }
        }
    }

    fn line_context(self) -> LineContext {
        match self {
            Self::Explicit => LineContext {
                section: ModSection::Explicit,
                flags: Vec::new(),
            },
            Self::Crafted => LineContext {
                section: ModSection::Explicit,
                flags: vec![CRAFTED.to_owned()],
            },
            Self::Implicit => LineContext {
                section: ModSection::Implicit,
                flags: Vec::new(),
            },
        }
    }

    /// The kind of a row the pool explains; `None` for an explicit row of
    /// no affix generation the rules name.
    fn kind(self, rules: &ModRules, row: Option<&ModRow>) -> Option<AffixKind> {
        if self == Self::Implicit {
            return Some(AffixKind::Implicit);
        }
        let g = row?.generation;
        let a = &rules.affix_generations;
        if a.prefix.contains(&g) {
            Some(AffixKind::Prefix)
        } else if a.suffix.contains(&g) {
            Some(AffixKind::Suffix)
        } else {
            None
        }
    }
}

/// One line to explain.
struct PoolLine<'a> {
    index: usize,
    stats: Vec<(&'a str, (i64, i64))>,
}

/// The most rows in one explanation.
const MAX_ROWS: usize = 8;
/// The most search steps for one group; past it the group gets no header.
const MAX_STEPS: usize = 20_000;

/// The rows in every explanation of their group, each with its lines
/// (module docs).
/// What [`infer`] explains one pool's lines with.
struct Inputs<'i> {
    /// The pool's line context.
    line: &'i LineContext,
    /// The pool.
    pool: Pool,
    /// Its lines.
    lines: &'i [PoolLine<'i>],
    /// Stats its lines carry with no shown value.
    hidden: &'i BTreeSet<&'i str>,
    /// Mod types the named mods already hold.
    taken: &'i BTreeSet<i64>,
}

fn infer(ctx: &Context<'_>, inputs: &Inputs<'_>) -> Vec<(usize, Vec<usize>)> {
    let Inputs {
        line,
        pool,
        lines,
        hidden,
        taken,
    } = *inputs;
    // Each stat to its line; a stat on two lines is left out with them.
    let mut at: BTreeMap<&str, usize> = BTreeMap::new();
    let mut twice: BTreeSet<&str> = BTreeSet::new();
    for (p, l) in lines.iter().enumerate() {
        for (s, _) in &l.stats {
            if at.insert(s, p).is_some_and(|q| q != p) {
                twice.insert(s);
            }
        }
    }
    let mut dropped: BTreeSet<usize> = twice.iter().filter_map(|s| at.get(s).copied()).collect();
    let bounds = |stat: &str| -> Option<(i64, i64)> {
        let p = at.get(stat)?;
        lines
            .get(*p)?
            .stats
            .iter()
            .find(|(s, _)| *s == stat)
            .map(|(_, b)| *b)
    };
    let implicits: BTreeSet<usize> = ctx
        .bases
        .iter()
        .flat_map(|b| b.implicits.iter().copied())
        .collect();
    let possible = |n: usize, row: &ModRow| match pool {
        Pool::Crafted => true,
        // A unique's implicits are its own: only the base's rows.
        Pool::Implicit => implicits.contains(&n) || (!ctx.unique && ctx.row_spawns(line, row)),
        Pool::Explicit => {
            ctx.bases.iter().all(|b| b.tags.is_none())
                || ctx.row_spawns(line, row)
                || row.essence_unknown()
                || row.influence.is_some_and(|i| {
                    ctx.influences.contains(&i) && row.spawn.iter().any(|(_, w)| *w > 0)
                })
        }
    };
    // A line a fitting, possible row could join to a line left out is left
    // out too: without its partner's value the row is not a reading the
    // search can weigh, and the line's other readings would look like the
    // only one (Ghoul Pelt's "Mammoth's": increased armour, on two lines,
    // with stun recovery; B-2 item evidence).
    loop {
        let out_stats: BTreeSet<&str> = lines
            .iter()
            .enumerate()
            .filter(|(p, _)| dropped.contains(p))
            .flat_map(|(_, l)| l.stats.iter().map(|(s, _)| *s))
            .collect();
        let joined: BTreeSet<usize> = out_stats
            .iter()
            .flat_map(|s| ctx.mods.row_indexes_with_stat(s).iter().copied())
            .filter_map(|n| Some((n, ctx.mods.row(n)?)))
            .filter(|(n, row)| ctx.fits(row, line) && possible(*n, row))
            .flat_map(|(_, row)| row.stats.iter().filter_map(|(s, _, _)| at.get(s.as_str())))
            .copied()
            .filter(|p| !dropped.contains(p))
            .collect();
        if joined.is_empty() {
            break;
        }
        dropped.extend(joined);
    }
    // Candidate rows: every stat shown on a kept line, values not out of
    // reach, fitting and possible.
    let mut candidates: BTreeSet<usize> = BTreeSet::new();
    for (stat, p) in &at {
        if dropped.contains(p) {
            continue;
        }
        for &n in ctx.mods.row_indexes_with_stat(stat) {
            let Some(row) = ctx.mods.row(n) else { continue };
            // Each row stat shown on a kept line (its value in reach), or
            // carried with no shown value.
            let shown = row.stats.iter().all(|(s, lo, hi)| {
                hidden.contains(s.as_str())
                    || (at.get(s.as_str()).is_some_and(|q| !dropped.contains(q))
                        && bounds(s).is_some_and(|(vlo, vhi)| reachable((*lo, *hi), (vlo, vhi))))
            });
            let free = row.mod_type.is_none_or(|t| !taken.contains(&t));
            if shown && free && ctx.fits(row, line) && possible(n, row) {
                candidates.insert(n);
            }
        }
    }
    // Groups: lines joined by a row that spans them.
    let mut group: Vec<usize> = (0..lines.len()).collect();
    fn root(group: &mut [usize], mut i: usize) -> usize {
        while let Some(&g) = group.get(i) {
            if g == i {
                break;
            }
            i = g;
        }
        i
    }
    for &n in &candidates {
        let Some(row) = ctx.mods.row(n) else { continue };
        let ps: Vec<usize> = row
            .stats
            .iter()
            .filter_map(|(s, _, _)| at.get(s.as_str()).copied())
            .collect();
        if let Some((first, rest)) = ps.split_first() {
            for p in rest {
                let (a, b) = (root(&mut group, *first), root(&mut group, *p));
                if let Some(slot) = group.get_mut(a) {
                    *slot = b;
                }
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for p in 0..lines.len() {
        if !dropped.contains(&p) {
            let r = root(&mut group, p);
            groups.entry(r).or_default().push(p);
        }
    }
    let mut out = Vec::new();
    for members in groups.values() {
        let stats: Vec<(&str, (i64, i64))> = members
            .iter()
            .filter_map(|p| lines.get(*p))
            .flat_map(|l| l.stats.iter().copied())
            .collect();
        let rows: Vec<usize> = candidates
            .iter()
            .copied()
            .filter(|n| {
                ctx.mods.row(*n).is_some_and(|r| {
                    r.stats
                        .iter()
                        .any(|(s, _, _)| at.get(s.as_str()).is_some_and(|p| members.contains(p)))
                })
            })
            .collect();
        let Some(common) = Search::new(ctx, &stats, &rows).run() else {
            continue;
        };
        for n in common {
            let Some(row) = ctx.mods.row(n) else { continue };
            let mut idx: Vec<usize> = row
                .stats
                .iter()
                .filter_map(|(s, _, _)| at.get(s.as_str()))
                .filter_map(|p| lines.get(*p).map(|l| l.index))
                .collect();
            idx.sort_unstable();
            idx.dedup();
            out.push((n, idx));
        }
    }
    out
}

/// Whether a row's `[lo, hi]` can be part of a sum that holds a value in
/// `[vlo, vhi]`: a row of one sign cannot pass the value on its own.
const fn reachable((lo, hi): (i64, i64), (vlo, vhi): (i64, i64)) -> bool {
    !((lo >= 0 && lo > vhi) || (hi <= 0 && hi < vlo))
}

/// The search for one group's explanations (module docs).
struct Search<'s, 'c> {
    ctx: &'s Context<'c>,
    stats: &'s [(&'s str, (i64, i64))],
    rows: &'s [usize],
    seen: BTreeSet<Vec<usize>>,
    steps: usize,
    /// The rows in every explanation so far; `None` before the first.
    common: Option<BTreeSet<usize>>,
    over: bool,
}

impl<'s, 'c> Search<'s, 'c> {
    fn new(ctx: &'s Context<'c>, stats: &'s [(&'s str, (i64, i64))], rows: &'s [usize]) -> Self {
        Self {
            ctx,
            stats,
            rows,
            seen: BTreeSet::new(),
            steps: 0,
            common: None,
            over: false,
        }
    }

    /// The rows in every explanation; `None` when there is none, or the
    /// search ran out of steps.
    fn run(mut self) -> Option<BTreeSet<usize>> {
        self.step(&mut Vec::new());
        if self.over {
            return None;
        }
        self.common.filter(|c| !c.is_empty())
    }

    fn range(&self, n: usize, stat: &str) -> Option<(i64, i64)> {
        self.ctx
            .mods
            .row(n)?
            .stats
            .iter()
            .find(|(s, _, _)| s == stat)
            .map(|(_, lo, hi)| (*lo, *hi))
    }

    fn step(&mut self, chosen: &mut Vec<usize>) {
        if self.over || self.common.as_ref().is_some_and(BTreeSet::is_empty) {
            return;
        }
        self.steps += 1;
        if self.steps > MAX_STEPS {
            self.over = true;
            return;
        }
        let mut key = chosen.clone();
        key.sort_unstable();
        if !self.seen.insert(key.clone()) {
            return;
        }
        // The first stat whose summed range misses its value.
        let unmet = self.stats.iter().find_map(|(stat, (vlo, vhi))| {
            let (mut lo, mut hi, mut carried) = (0, 0, false);
            for n in chosen.iter() {
                if let Some((a, b)) = self.range(*n, stat) {
                    lo += a;
                    hi += b;
                    carried = true;
                }
            }
            (!carried || hi < *vlo || lo > *vhi).then_some((*stat, lo, hi, *vlo, *vhi, carried))
        });
        let Some((stat, lo, hi, vlo, vhi, carried)) = unmet else {
            let set: BTreeSet<usize> = key.into_iter().collect();
            self.common = Some(match self.common.take() {
                None => set,
                Some(c) => c.intersection(&set).copied().collect(),
            });
            return;
        };
        if chosen.len() >= MAX_ROWS {
            return;
        }
        let types: BTreeSet<i64> = chosen
            .iter()
            .filter_map(|n| self.ctx.mods.row(*n).and_then(|r| r.mod_type))
            .collect();
        let next: Vec<usize> = self
            .rows
            .iter()
            .copied()
            .filter(|n| !chosen.contains(n))
            .filter(|n| {
                self.ctx
                    .mods
                    .row(*n)
                    .is_some_and(|r| r.mod_type.is_none_or(|t| !types.contains(&t)))
            })
            .filter(|n| {
                self.range(*n, stat)
                    .is_some_and(|(a, b)| !carried || (hi < vlo && b > 0) || (lo > vhi && a < 0))
            })
            .collect();
        for n in next {
            chosen.push(n);
            self.step(chosen);
            chosen.pop();
        }
    }
}

/// The tier of row `n` on the item's base (module docs); `None` when the
/// row does not spawn on every base row by its tags, or the base rows
/// disagree.
fn tier(ctx: &Context<'_>, bundle: &Bundle, n: usize) -> Option<u32> {
    let row = ctx.mods.row(n)?;
    let mod_type = row.mod_type?;
    if row.essence_only || row.influence.is_some() {
        return None;
    }
    let level = |k: usize| bundle.mods.get(k).and_then(|m| m.level);
    let own = level(n)?;
    let tiers: BTreeSet<Option<u32>> = ctx
        .bases
        .iter()
        .map(|b| {
            let tags = b.tags.as_ref()?;
            if row.weight(tags) <= 0 {
                return None;
            }
            let above = ctx
                .mods
                .row_indexes_of_type(mod_type)
                .iter()
                .filter(|&&k| {
                    ctx.mods.row(k).is_some_and(|q| {
                        q.domain == row.domain
                            && q.generation == row.generation
                            && !q.essence_only
                            && q.influence.is_none()
                            && q.weight(tags) > 0
                    }) && level(k).is_some_and(|l| l > own)
                })
                .count();
            u32::try_from(above).ok().map(|a| a + 1)
        })
        .collect();
    match tiers.into_iter().collect::<Vec<_>>()[..] {
        [Some(t)] => Some(t),
        _ => None,
    }
}
