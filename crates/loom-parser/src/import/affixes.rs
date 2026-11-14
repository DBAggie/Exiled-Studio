//! Which mod made which line, for the item tooltip's Alt view (UX sprint,
//! B-2; `dev/active/ux-improvements/b2-item-data-plan.md`).
//!
//! **Display only.** Nothing here feeds line resolution or the
//! calculation: a line no mod is found for keeps its text and is shown
//! without a header (ADR-015: never dropped).
//!
//! **Exact headers (PR 1).** PoB writes an item's affixes as spec lines,
//! `Prefix: {range:0.718}LocalBaseArmourAndEnergyShield4` (`None` for an
//! open affix). A spec is used only when its mod is in the bundle and its
//! stats account for lines of the item: an explicit, non-crafted line all
//! of whose shown stats are the mod's, with values in its ranges. Lines may
//! be anywhere in the item (the game prints lines in stat-description
//! order, not mod order). A line two named mods both carry (the game sums
//! them into one line: a hybrid and a pure "increased Armour") goes under
//! both when its value fits their summed ranges, else under neither. A spec
//! whose mod accounts for no line (unknown ID, a line edited by hand) is
//! dropped. The kind is PoB's key, so no per-game generation numbers are
//! needed.
//!
//! The affix name, level, and tag names come from a data schema 4 bundle
//! ([`bundle::AFFIX_SCHEMA`]); an older bundle gives the header without
//! them.

use loom_data::bundle::{self, Bundle, ModEntry};
use loom_domain::{
    ItemBody, LineKind, LineResolution, ModSection, ModifierLine, Resolved, StatValue,
};

/// Whether an affix is a prefix or a suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AffixKind {
    /// A prefix.
    Prefix,
    /// A suffix.
    Suffix,
}

/// One mod of an item and the lines it made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Affix {
    /// Prefix or suffix.
    pub kind: AffixKind,
    /// The mod's ID.
    pub mod_id: String,
    /// The affix name (`Urchin's`), when the bundle has one.
    pub name: Option<String>,
    /// The mod's level, when the bundle has it.
    pub level: Option<i64>,
    /// The mod's tags as the game prints them (`Life`, `Defences`).
    pub tags: Vec<String>,
    /// The item's line indices it made, in item order.
    pub lines: Vec<usize>,
}

/// The item's affixes PoB named, prefixes first, each in spec order
/// (module docs).
#[must_use]
pub fn named_affixes(body: &ItemBody, bundle: &Bundle) -> Vec<Affix> {
    let specs: Vec<(AffixKind, &ModEntry)> = body
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
                bundle.mods.iter().find(|m| m.id == id).map(|m| (kind, m))
            }
            _ => None,
        })
        .collect();
    let mut lines: Vec<Vec<usize>> = vec![Vec::new(); specs.len()];
    for (index, line) in body.lines.iter().enumerate() {
        let LineKind::Modifier(m) = &line.kind else {
            continue;
        };
        let Some(shown) = shown_stats(m) else {
            continue;
        };
        let claimants: Vec<usize> = specs
            .iter()
            .enumerate()
            .filter(|(_, (_, entry))| {
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
                .filter_map(|(_, entry)| entry.stats.iter().find(|s| s.stat == *stat))
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
    let with_data = bundle.manifest.data_schema >= bundle::AFFIX_SCHEMA;
    let mut out: Vec<Affix> = specs
        .iter()
        .zip(lines)
        .filter(|(_, l)| !l.is_empty())
        .map(|((kind, entry), lines)| Affix {
            kind: *kind,
            mod_id: entry.id.clone(),
            name: (with_data && !entry.name.is_empty()).then(|| entry.name.clone()),
            level: entry.level.filter(|_| with_data),
            tags: if with_data {
                entry
                    .implicit_tags
                    .iter()
                    .filter_map(|t| bundle.tags.iter().find(|n| n.id == *t))
                    .map(|n| n.display.clone())
                    .collect()
            } else {
                Vec::new()
            },
            lines,
        })
        .collect();
    out.sort_by_key(|a| a.kind);
    out
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

/// An explicit, non-crafted, resolved line's shown stats with their value
/// bounds (hidden and lookup values left out); `None` for any other line
/// or one with no shown stat.
fn shown_stats(m: &ModifierLine) -> Option<Vec<(&str, (i64, i64))>> {
    if m.section != ModSection::Explicit || m.prefixes.iter().any(|p| p.key == "crafted") {
        return None;
    }
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
