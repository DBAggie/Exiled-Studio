//! PoB's rules for an item line's values that the line's text does not
//! settle (P3-8b): the roll of a ranged line, and the scalings the engine
//! does not model. The host hands the result to the engine
//! (`loom_calc::CalcOptions::item_lines`); the build document is not
//! changed.
//!
//! Cited from PoB `v2.67.2`:
//! - **Roll** (`Modules/ItemTools.lua` `itemLib.applyRange`, `formatValue`):
//!   each range `(a-b)` of a line with `{range:r}` becomes `a + r x (b -
//!   a)`, rounded away from zero at the stat's precision. The resolver
//!   keeps a range's ends only for a stat whose raw value is its shown
//!   number (or its negative), so the precision is 1 and the roll is
//!   `roundSymmetric(from + r x (to - from))` on the raw ends.
//!   A line without `{range:}` would take PoB's user setting
//!   `defaultItemAffixQuality` (0.5 by default): not the build's, so no
//!   roll. A roll outside `[0, 1]`, or a `value(min-max)` line (PoB's
//!   `rangeFromValue`, which derives the roll from the value), gives none.
//! - **`<ModRange>`** (`Classes/ItemsTab.lua` `Load`): an entry overrides
//!   its line's roll. PoB writes both from the same `modLine.range`, so a
//!   saved build agrees; the adapter maps entry `k` to the `k`-th
//!   modifier line (class-requirement lines aside) and gives no roll on
//!   an item where an entry differs from its line's `{range:}` by more
//!   than 5e-7 (the prefix is written to 6 decimals). With more entries
//!   than lines PoB ignores them all, and so does the adapter.
//! - **Catalyst** (`Classes/Item.lua` `getCatalystScalar`, `ParseRaw`):
//!   with a `Catalyst` and its quality (`CatalystQuality`, default 20),
//!   every line whose `{tags}` (or `prefix`/`suffix` flag) meets the
//!   catalyst's tags is scaled, ranged or not, unless `{unscalable}` or
//!   ` - Unscalable Value`. The game scales only stats it marks scalable,
//!   which the bundle does not carry yet: such a line is
//!   [`LineValues::Scaled`] (`catalyst`). An untagged ranged line on a
//!   catalysed item is too (the game scales it; PoB does not), and a
//!   `Quality (... Modifiers)` catalyst line, whose catalyst the adapter
//!   does not name, scales every tagged or ranged line.
//! - **Corrupted range** (`{corruptedRange}`, `formatValue`'s
//!   `baseValueScalar`): the line is scaled (`corrupted-range`).
//! - **Modifier magnitudes** ("... Modifier magnitudes", "... effect of
//!   Prefixes/Suffixes"; `Item.lua` `modMagnitudeMods`): they scale other
//!   lines of the item, so every modifier line of such an item is scaled
//!   (`modifier-magnitude`).

use std::collections::BTreeMap;

use loom_calc::{LineValues, Scaling};
use loom_domain::{
    Entry, Item, LineKind, LineResolution, ModSection, ModifierLine, OpaqueNode, Resolved,
    TypedResolution,
};

/// PoB's `catalystList`, in order.
const CATALYSTS: [&str; 12] = [
    "Abrasive",
    "Accelerating",
    "Dextral",
    "Fertile",
    "Imbued",
    "Intrinsic",
    "Noxious",
    "Prismatic",
    "Sinistral",
    "Tempering",
    "Turbulent",
    "Unstable",
];

/// PoB's `catalystTags`, by the index of [`CATALYSTS`].
const CATALYST_TAGS: [&[&str]; 12] = [
    &["attack"],
    &["speed"],
    &["suffix"],
    &["life", "mana", "resource"],
    &["caster"],
    &["jewellery_attribute", "attribute"],
    &["physical_damage", "chaos_damage"],
    &["jewellery_resistance", "resistance"],
    &["prefix"],
    &[
        "jewellery_defense",
        "defences",
        "armour",
        "evasion",
        "energyshield",
    ],
    &["jewellery_elemental", "elemental_damage"],
    &["critical"],
];

/// How far a `<ModRange>` may differ from its line's `{range:}` (PoB
/// writes the prefix rounded to 6 decimals).
const MOD_RANGE_TOLERANCE: f64 = 5e-7;

/// The exact ends of a line's ranges, one per stat of its resolution
/// (`(from, to)` in the line's order; `None`: not a range with exact ends),
/// by line index: the resolver's `LineRangeEnds` as plain pairs.
pub type LineEnds = BTreeMap<usize, Vec<Option<(i64, i64)>>>;

/// The catalyst state of an item.
enum Catalyst {
    None,
    /// Named, with its tags.
    Tags(&'static [&'static str]),
    /// From a `Quality (... Modifiers)` line: tags not known here.
    Unnamed,
}

/// What PoB's rules settle about the item's lines ([`LineValues`] by line
/// index; a line not listed keeps its resolution's values), given the
/// exact ends of its ranges.
#[must_use]
pub fn line_values(item: &Item, ends: &LineEnds) -> BTreeMap<usize, LineValues> {
    let body = &item.body;
    let mut out = BTreeMap::new();
    let modifiers: Vec<(usize, &ModifierLine)> = body.modifiers().collect();
    if modifiers
        .iter()
        .any(|(_, m)| m.selected && is_magnitude(&m.text))
    {
        for (n, _) in &modifiers {
            out.insert(*n, LineValues::Scaled(Scaling::ModifierMagnitude));
        }
        return out;
    }
    let catalyst = catalyst(item);
    let rolls_agree = mod_ranges_agree(item, &modifiers);
    // A multi-line reading is decided on its first line, with the lines
    // that continue it: one scaled line scales the reading, and one roll
    // must hold for every ranged line (PoB rolls each line on its own).
    let mut covered: BTreeMap<usize, Vec<&ModifierLine>> = BTreeMap::new();
    for (n, m) in &modifiers {
        let first = match m.resolution {
            LineResolution::Continues { line } => line,
            _ => *n,
        };
        covered.entry(first).or_default().push(m);
    }
    for (n, lines) in &covered {
        if lines.iter().any(|m| m.has("corruptedRange")) {
            out.insert(*n, LineValues::Scaled(Scaling::CorruptedRange));
            continue;
        }
        if lines
            .iter()
            .any(|m| catalysed(&catalyst, m, has_range(&m.text)))
        {
            out.insert(*n, LineValues::Scaled(Scaling::Catalyst));
            continue;
        }
        let ranged: Vec<&&ModifierLine> = lines.iter().filter(|m| has_range(&m.text)).collect();
        if !rolls_agree || ranged.is_empty() || lines.iter().any(|m| has_value_then_range(&m.text))
        {
            continue;
        }
        let rolls: Vec<Option<f64>> = ranged.iter().map(|m| roll(m)).collect();
        let Some(Some(r)) = rolls.first().copied() else {
            continue;
        };
        if rolls.iter().any(|x| *x != Some(r)) {
            continue;
        }
        let Some(line_ends) = ends.get(n) else {
            continue;
        };
        let values: Vec<Option<i64>> = line_ends
            .iter()
            .map(|e| e.map(|(from, to)| interpolate(from, to, r)))
            .collect();
        if values.iter().any(Option::is_some) {
            out.insert(*n, LineValues::Rolled(values));
        }
    }
    out
}

/// PoB's roll at precision 1: `from + r x (to - from)`, rounded away
/// from zero (`roundSymmetric`). PoB's fallback path (a line without
/// `modScalability` data) rounds a negative tie up instead; a known limit.
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "stat values are far below 2^53; the rounded roll lies between two i64 ends"
)]
fn interpolate(from: i64, to: i64, r: f64) -> i64 {
    let (a, b) = (from as f64, to as f64);
    let v = a + r * (b - a);
    let rounded = if v >= 0.0 {
        (v + 0.5).floor()
    } else {
        (v - 0.5).ceil()
    };
    rounded as i64
}

/// The line's `{range:r}` when it is a number in `[0, 1]`.
fn roll(m: &ModifierLine) -> Option<f64> {
    let r: f64 = m
        .prefixes
        .iter()
        .find(|p| p.key == "range")?
        .value
        .as_deref()?
        .trim()
        .parse()
        .ok()?;
    (0.0..=1.0).contains(&r).then_some(r)
}

/// Whether the text holds a range `(a-b)` (PoB's pattern
/// `%((%-?%d+%.?%d*)%-(%-?%d+%.?%d*)%)`).
fn has_range(text: &str) -> bool {
    text.split('(').skip(1).any(|rest| {
        number(rest)
            .and_then(|after| after.strip_prefix('-'))
            .and_then(number)
            .is_some_and(|after| after.starts_with(')'))
    })
}

/// The rest of `s` after a number `-?\d+(\.\d*)?` at its start; `None`
/// when it does not start with one.
fn number(s: &str) -> Option<&str> {
    let unsigned = s.strip_prefix('-').unwrap_or(s);
    let after = unsigned.trim_start_matches(|c: char| c.is_ascii_digit());
    if after.len() == unsigned.len() {
        return None;
    }
    Some(after.strip_prefix('.').map_or(after, |frac| {
        frac.trim_start_matches(|c: char| c.is_ascii_digit())
    }))
}

/// A `value(min-max)` line (PoB's `%d+%.?%d*%(%-?%d`): a number right
/// before a parenthesis that opens on a number.
fn has_value_then_range(text: &str) -> bool {
    let parts: Vec<&str> = text.split('(').collect();
    parts.windows(2).any(|w| match w {
        [before, after] => {
            before.ends_with(|c: char| c.is_ascii_digit() || c == '.')
                && after
                    .strip_prefix('-')
                    .unwrap_or(after)
                    .starts_with(|c: char| c.is_ascii_digit())
        }
        _ => false,
    })
}

/// PoB's `modMagnitudePattern`s: "... modifier magnitudes" and "...
/// effect of prefixes/suffixes".
fn is_magnitude(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("modifier magnitudes")
        || t.contains("effect of prefixes")
        || t.contains("effect of suffixes")
}

/// The item's catalyst: a `Catalyst` spec PoB names with a quality above
/// 0 (`CatalystQuality`, default 20), or a `catalyst-quality` line.
fn catalyst(item: &Item) -> Catalyst {
    let mut named = None;
    let mut quality: Option<f64> = None;
    for line in &item.body.lines {
        match &line.kind {
            LineKind::Spec { key, value, .. } if key == "Catalyst" => {
                named = CATALYSTS
                    .iter()
                    .position(|c| *c == value.trim())
                    .and_then(|i| CATALYST_TAGS.get(i).copied());
            }
            LineKind::Spec { key, value, .. } if key == "CatalystQuality" => {
                quality = value.trim().parse().ok();
            }
            LineKind::Modifier(m)
                if m.selected
                    && matches!(
                        &m.resolution,
                        LineResolution::Resolved {
                            result: Resolved::Typed(TypedResolution::CatalystQuality { .. }),
                            ..
                        }
                    ) =>
            {
                // Conservative: the quality is not read (even +0%).
                return Catalyst::Unnamed;
            }
            _ => {}
        }
    }
    match named {
        Some(tags) if quality.unwrap_or(20.0) != 0.0 => Catalyst::Tags(tags),
        _ => Catalyst::None,
    }
}

/// Whether the catalyst scales the line.
fn catalysed(catalyst: &Catalyst, m: &ModifierLine, ranged: bool) -> bool {
    if matches!(catalyst, Catalyst::None)
        || m.has("unscalable")
        || m.text.ends_with(" - Unscalable Value")
    {
        return false;
    }
    let mut tags: Vec<&str> = m
        .prefixes
        .iter()
        .filter(|p| p.key == "tags")
        .filter_map(|p| p.value.as_deref())
        .flat_map(|v| v.split(','))
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect();
    let untagged = tags.is_empty();
    for flag in ["prefix", "suffix"] {
        if m.has(flag) {
            tags.push(flag);
        }
    }
    match catalyst {
        Catalyst::None => false,
        Catalyst::Unnamed => !untagged || ranged,
        Catalyst::Tags(scaled) => (untagged && ranged) || tags.iter().any(|t| scaled.contains(t)),
    }
}

/// Whether the item's `<ModRange>` entries agree with its lines'
/// `{range:}` prefixes (module docs).
fn mod_ranges_agree(item: &Item, modifiers: &[(usize, &ModifierLine)]) -> bool {
    let listed: Vec<&ModifierLine> = modifiers
        .iter()
        .map(|(_, m)| *m)
        .filter(|m| m.section != ModSection::Other("class-requirement".to_owned()))
        .collect();
    let entries: Vec<(usize, f64)> = item
        .layout
        .entries()
        .iter()
        .filter_map(|e| match e {
            Entry::Opaque(OpaqueNode::Element {
                name, attributes, ..
            }) if name == "ModRange" => {
                let get = |k: &str| {
                    attributes
                        .iter()
                        .find(|a| a.name == k)
                        .map(|a| a.value.trim())
                };
                let id = get("id").and_then(|v| v.parse().ok()).unwrap_or(0);
                let range = get("range").and_then(|v| v.parse().ok()).unwrap_or(1.0);
                Some((id, range))
            }
            _ => None,
        })
        .collect();
    if entries.len() > listed.len() {
        return true; // PoB ignores them all.
    }
    entries.iter().all(|(id, range)| {
        let Some(line) = id.checked_sub(1).and_then(|k| listed.get(k)) else {
            return true; // No such line: PoB applies it to none.
        };
        // A line with a range but no `{range:}`: PoB rolls it from the
        // entry, the adapter does not roll it (it stays unsettled), so
        // the entry cannot change a value the adapter gives.
        if !has_range(&line.text) || !line.has("range") {
            return true;
        }
        roll(line).is_some_and(|r| (r - range).abs() <= MOD_RANGE_TOLERANCE)
    })
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    clippy::panic,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]
mod tests {
    use loom_domain::{Attribute, ItemId, ItemPart, Ordered};

    use super::super::{read, to_domain};
    use super::*;

    /// An item from PoB text, with `<ModRange>` entries `(id, range)`.
    fn item(text: &str, mod_ranges: &[(&str, &str)]) -> Item {
        let mut layout = Ordered::new();
        layout.push(ItemPart::Body);
        for (id, range) in mod_ranges {
            layout.push_opaque(OpaqueNode::Element {
                name: "ModRange".to_owned(),
                attributes: vec![Attribute::new("id", *id), Attribute::new("range", *range)],
                children: Vec::new(),
            });
        }
        Item {
            id: ItemId(1),
            body: to_domain(&read(text)),
            layout,
            text_spacing: Vec::new(),
            attributes: Vec::new(),
        }
    }

    /// `ends` for each line index given, one stat each.
    fn ends(lines: &[(usize, i64, i64)]) -> LineEnds {
        lines
            .iter()
            .map(|(n, from, to)| (*n, vec![Some((*from, *to))]))
            .collect()
    }

    const RARE: &str = "Rarity: RARE\nA\nIron Ring\nPrefix: {range:0.4}IncreasedLife1\n";

    fn rare(lines: &str) -> String {
        format!("{RARE}{lines}")
    }

    #[test]
    fn a_rolled_line_should_take_from_plus_r_times_the_span_rounded_away_from_zero() {
        let text = rare(
            "Implicits: 0\n{range:0.468}+(10-20) to maximum Life\n{range:0.5}(10-20)% reduced X\n{range:0.049}-(7-6) to Y\n{range:0.5}+(16-24) to Z",
        );
        let it = item(&text, &[]);
        let lines: Vec<usize> = it.body.modifiers().map(|(n, _)| n).collect();
        let e = ends(&[
            (lines[0], 10, 20),
            (lines[1], -10, -20),
            (lines[2], -7, -6),
            (lines[3], 16, 24),
        ]);
        let got = line_values(&it, &e);
        let rolled = |n: usize| match got.get(&lines[n]) {
            Some(LineValues::Rolled(v)) => v.clone(),
            other => panic!("{other:?}"),
        };
        // 14.68 -> 15; -15 (a tie away from zero is -15 either way);
        // -6.951 -> -7; 20.
        assert_eq!(rolled(0), [Some(15)]);
        assert_eq!(rolled(1), [Some(-15)]);
        assert_eq!(rolled(2), [Some(-7)]);
        assert_eq!(rolled(3), [Some(20)]);
        // The affix spec's {range:} is not a mod line.
        assert_eq!(got.len(), 4);
    }

    #[test]
    fn a_tie_should_round_away_from_zero() {
        assert_eq!(interpolate(1, 2, 0.5), 2);
        assert_eq!(interpolate(-1, -2, 0.5), -2);
        assert_eq!(interpolate(10, 20, 0.0), 10);
        assert_eq!(interpolate(10, 20, 1.0), 20);
    }

    #[test]
    fn no_roll_without_a_prefix_in_range_or_with_a_value_before_the_range() {
        let text = rare(
            "Implicits: 0\n+(10-20) to maximum Life\n{range:1.2}+(10-20) to maximum Life\n{range:0.5}+15(10-20) to maximum Life",
        );
        let it = item(&text, &[]);
        let e: LineEnds = it
            .body
            .modifiers()
            .map(|(n, _)| (n, vec![Some((10, 20))]))
            .collect();
        assert!(line_values(&it, &e).is_empty());
    }

    #[test]
    fn catalyst_lines_should_be_scaled_ranged_or_not() {
        let text = format!(
            "Rarity: RARE\nA\nIron Ring\nCatalyst: Intrinsic\nCatalystQuality: 20\nImplicits: 0\n{}\n{}\n{}\n{}\n{}",
            "{tags:attribute}{range:1}+(16-24) to Strength",
            "{tags:attribute}+20 to Dexterity",
            "{tags:life}{range:1}+(10-20) to maximum Life",
            "+(10-20) to Intelligence",
            "{tags:attribute}{unscalable}+5 to Strength",
        );
        let it = item(&text, &[]);
        let lines: Vec<usize> = it.body.modifiers().map(|(n, _)| n).collect();
        let e = ends(&[(lines[0], 16, 24), (lines[2], 10, 20)]);
        let got = line_values(&it, &e);
        assert_eq!(
            got.get(&lines[0]),
            Some(&LineValues::Scaled(Scaling::Catalyst))
        );
        assert_eq!(
            got.get(&lines[1]),
            Some(&LineValues::Scaled(Scaling::Catalyst))
        );
        assert_eq!(
            got.get(&lines[2]),
            Some(&LineValues::Rolled(vec![Some(20)]))
        );
        // Untagged and ranged: the game scales it.
        assert_eq!(
            got.get(&lines[3]),
            Some(&LineValues::Scaled(Scaling::Catalyst))
        );
        assert_eq!(got.get(&lines[4]), None);
        // Quality 0: no scaling.
        let zero = text.replace("CatalystQuality: 20", "CatalystQuality: 0");
        let got = line_values(&item(&zero, &[]), &e);
        assert_eq!(
            got.get(&lines[0]),
            Some(&LineValues::Rolled(vec![Some(24)]))
        );
    }

    #[test]
    fn corrupted_range_and_magnitude_lines_should_be_scaled() {
        let text =
            rare("Implicits: 0\n{corruptedRange:1.22}35% of Your Maximum Life\n+10 to Strength");
        let it = item(&text, &[]);
        let lines: Vec<usize> = it.body.modifiers().map(|(n, _)| n).collect();
        let got = line_values(&it, &LineEnds::new());
        assert_eq!(
            got.get(&lines[0]),
            Some(&LineValues::Scaled(Scaling::CorruptedRange))
        );
        assert_eq!(got.get(&lines[1]), None);
        let text =
            rare("Implicits: 0\n100% increased Explicit Modifier magnitudes\n+10 to Strength");
        let got = line_values(&item(&text, &[]), &LineEnds::new());
        assert_eq!(got.len(), 2);
        assert!(
            got.values()
                .all(|v| *v == LineValues::Scaled(Scaling::ModifierMagnitude))
        );
    }

    #[test]
    fn mod_ranges_should_agree_with_the_prefixes_or_stop_every_roll() {
        let text = rare("Implicits: 0\n+10 to Strength\n{range:0.5}+(10-20) to maximum Life");
        let lines: Vec<usize> = item(&text, &[]).body.modifiers().map(|(n, _)| n).collect();
        let e = ends(&[(lines[1], 10, 20)]);
        // Agreeing (to 6 decimals), and an entry on a line without a range.
        let agree = item(&text, &[("1", "0"), ("2", "0.5000001")]);
        assert_eq!(
            line_values(&agree, &e).get(&lines[1]),
            Some(&LineValues::Rolled(vec![Some(15)]))
        );
        let conflict = item(&text, &[("1", "1"), ("2", "0.7")]);
        assert!(line_values(&conflict, &e).is_empty());
        // More entries than lines: PoB ignores them.
        let extra = item(&text, &[("1", "1"), ("2", "0.7"), ("3", "1")]);
        assert_eq!(line_values(&extra, &e).len(), 1);
    }

    #[test]
    fn has_range_should_follow_pobs_pattern() {
        assert!(has_range("+(10-20) to X"));
        assert!(has_range("Adds 1 to (20.5-30) X"));
        assert!(has_range("(-10-10)% increased X"));
        assert!(!has_range("+10 to X (Hidden)"));
        assert!(!has_range("(10) X"));
        assert!(has_value_then_range("+15(10-20) to X"));
        assert!(!has_value_then_range("+(10-20) to X"));
        // A number before a parenthesis that does not open on a number.
        assert!(!has_value_then_range("Level 10(Hidden) and +(1-2) to X"));
    }

    /// `text`'s modifier lines, the second continuing the first.
    fn two_line_reading(text: &str) -> (Item, Vec<usize>) {
        let mut it = item(text, &[]);
        let lines: Vec<usize> = it.body.modifiers().map(|(n, _)| n).collect();
        if let LineKind::Modifier(m) = &mut it.body.lines[lines[1]].kind {
            m.resolution = LineResolution::Continues { line: lines[0] };
        }
        (it, lines)
    }

    #[test]
    fn a_multi_line_reading_should_be_decided_with_its_continuation_lines() {
        // The same roll on both lines: rolled on the first.
        let (it, lines) = two_line_reading(&rare(
            "Implicits: 0\n{range:0.5}+(10-20) to X\n{range:0.5}+(30-40) to Y",
        ));
        let e: LineEnds = BTreeMap::from([(lines[0], vec![Some((10, 20)), Some((30, 40))])]);
        assert_eq!(
            line_values(&it, &e).get(&lines[0]),
            Some(&LineValues::Rolled(vec![Some(15), Some(35)]))
        );
        // Different rolls: none.
        let (it, _) = two_line_reading(&rare(
            "Implicits: 0\n{range:0.5}+(10-20) to X\n{range:1}+(30-40) to Y",
        ));
        assert!(line_values(&it, &e).is_empty());
        // A corrupted range on the continuation scales the reading.
        let (it, lines) = two_line_reading(&rare(
            "Implicits: 0\n{range:0.5}+(10-20) to X\n{corruptedRange:1.2}+30 to Y",
        ));
        assert_eq!(
            line_values(&it, &e).get(&lines[0]),
            Some(&LineValues::Scaled(Scaling::CorruptedRange))
        );
    }

    #[test]
    fn an_unnamed_catalyst_should_scale_tagged_or_ranged_lines() {
        let mut it = item(
            &rare(
                "Implicits: 0\nQuality (Life Modifiers): +20%\n{tags:life}+10 to maximum Life\n+5 to Strength",
            ),
            &[],
        );
        let lines: Vec<usize> = it.body.modifiers().map(|(n, _)| n).collect();
        if let LineKind::Modifier(m) = &mut it.body.lines[lines[0]].kind {
            m.resolution = LineResolution::Resolved {
                result: Resolved::Typed(TypedResolution::CatalystQuality {
                    quality_type: "Life".to_owned(),
                    stats: Vec::new(),
                    quality: "20".to_owned(),
                }),
                adapted: None,
            };
        }
        let got = line_values(&it, &LineEnds::new());
        assert_eq!(
            got.get(&lines[1]),
            Some(&LineValues::Scaled(Scaling::Catalyst))
        );
        assert_eq!(got.get(&lines[2]), None, "untagged and not ranged");
    }

    #[test]
    fn an_unselected_magnitude_line_should_scale_nothing() {
        let mut it = item(
            &rare("Implicits: 0\n100% increased Explicit Modifier magnitudes\n+10 to Strength"),
            &[],
        );
        let first = it
            .body
            .modifiers()
            .map(|(n, _)| n)
            .next()
            .unwrap_or_default();
        if let LineKind::Modifier(m) = &mut it.body.lines[first].kind {
            m.selected = false;
        }
        assert!(line_values(&it, &LineEnds::new()).is_empty());
    }

    #[test]
    fn a_mod_range_should_agree_within_the_tolerance_only() {
        let text = rare("Implicits: 0\n{range:0.5}+(10-20) to maximum Life");
        let line = item(&text, &[])
            .body
            .modifiers()
            .map(|(n, _)| n)
            .next()
            .unwrap_or_default();
        let e = ends(&[(line, 10, 20)]);
        assert_eq!(
            line_values(&item(&text, &[("1", "0.5000005")]), &e).len(),
            1
        );
        assert!(line_values(&item(&text, &[("1", "0.500001")]), &e).is_empty());
    }
}
