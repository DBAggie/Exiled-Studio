//! Devotion (Phase 6 P6-11az): "+# to Devotion" (`base_devotion`, PoB's
//! `Devotion` BASE; Militant Faith's conquests) and the lines that read it.
//!
//! PoB v2.67.2 sums `Devotion` (`Modules/CalcPerform.lua` 490 to 491) and
//! reads it in two ways (`ModParser.lua`):
//! - "per 10 Devotion": a `PerStat` tag, `stat = "Devotion", div = 10`,
//!   whose multiplier `ModStore` floors (`floor(Devotion / 10 + 0.0001)`,
//!   `Classes/ModStore.lua`);
//! - "if you have at least 150 Devotion": a `StatThreshold` tag,
//!   `threshold = 150`.
//!
//! Each reader here ([`READERS`]) means one of the game's other stats,
//! times that count (or 0 or 1 for a threshold): [`devotion`] sums the
//! devotion after every other source and ingests that stat with the
//! reader's value so scaled, through the mapping, from the reader's
//! source. A reader whose stat the mapping does not have stays
//! `Unmapped`: a reason. "Regenerate # Mana per Second per 10 Devotion"
//! stays one too: the game prints its per-minute value to one decimal, so
//! the text does not settle it, and a reader mapped with an unsettled
//! value would make the fields indeterminate rather than partial.

use super::Poe1Quantity;
use crate::ingest::Ingest;
use crate::quantity::{Op, Quantity};
use crate::store::Conditions;
use loom_domain::{StatId, StatLine, StatValue};

/// How a reader counts devotion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    /// Per 10 Devotion, floored.
    Per10,
    /// At least 150 Devotion: 1, else 0.
    AtLeast150,
}

/// The readers the engine models, sorted by stat ID: `(reader, the stat
/// it means, the factor between them, how it counts)`. PoB's text for
/// each: "#% increased Area Damage per 10 Devotion", "#% increased
/// Elemental Damage per 10 Devotion", "+#% to all Elemental Resistances
/// per 10 Devotion", "#% increased Mana Cost Efficiency per 10 Devotion",
/// "#% reduced Mana Cost of Skills per 10 Devotion" (the reader is the
/// `+%` stat, its value negative; the stat it means is the `-%` one),
/// and at 150 Devotion
/// "+#% to all maximum Elemental Resistances", "#% additional Physical
/// Damage Reduction", "#% of Physical Damage Converted to Fire (Lightning)
/// Damage".
pub const READERS: &[(&str, &str, f64, Reading)] = &[
    (
        "additional_maximum_all_resistances_%_at_devotion_threshold",
        "additional_maximum_all_resistances_%",
        1.0,
        Reading::AtLeast150,
    ),
    (
        "area_damage_+%_per_10_devotion",
        "area_damage_+%",
        1.0,
        Reading::Per10,
    ),
    (
        "base_mana_cost_efficiency_+%_per_10_devotion",
        "base_mana_cost_efficiency_+%",
        1.0,
        Reading::Per10,
    ),
    (
        "elemental_damage_+%_per_10_devotion",
        "elemental_damage_+%",
        1.0,
        Reading::Per10,
    ),
    (
        "elemental_resistance_%_per_10_devotion",
        "base_resist_all_elements_%",
        1.0,
        Reading::Per10,
    ),
    (
        "mana_cost_+%_per_10_devotion",
        "base_mana_cost_-%",
        -1.0,
        Reading::Per10,
    ),
    // P6-11fb.
    (
        "minimum_endurance_charges_at_devotion_threshold",
        "base_minimum_endurance_charges",
        1.0,
        Reading::AtLeast150,
    ),
    // P6-11fb.
    (
        "minimum_frenzy_charges_at_devotion_threshold",
        "base_minimum_frenzy_charges",
        1.0,
        Reading::AtLeast150,
    ),
    // P6-11fb.
    (
        "minimum_power_charges_at_devotion_threshold",
        "base_minimum_power_charges",
        1.0,
        Reading::AtLeast150,
    ),
    // P6-11fb.
    (
        "physical_damage_%_to_convert_to_cold_at_devotion_threshold",
        "base_physical_damage_%_to_convert_to_cold",
        1.0,
        Reading::AtLeast150,
    ),
    (
        "physical_damage_%_to_convert_to_fire_at_devotion_threshold",
        "base_physical_damage_%_to_convert_to_fire",
        1.0,
        Reading::AtLeast150,
    ),
    (
        "physical_damage_%_to_convert_to_lightning_at_devotion_threshold",
        "base_physical_damage_%_to_convert_to_lightning",
        1.0,
        Reading::AtLeast150,
    ),
    (
        "physical_damage_reduction_%_at_devotion_threshold",
        "base_additional_physical_damage_reduction_%",
        1.0,
        Reading::AtLeast150,
    ),
];

/// The threshold of the `AtLeast150` readers.
pub const THRESHOLD: f64 = 150.0;

/// The reader `stat` is, by its index in [`READERS`], if any.
#[must_use]
pub fn reader(stat: &str) -> Option<u8> {
    READERS
        .binary_search_by(|(id, ..)| (*id).cmp(stat))
        .ok()
        .and_then(|i| u8::try_from(i).ok())
}

/// Adds the readers' effects (module docs): after every other source of
/// devotion and of readers.
pub fn devotion(ingest: &mut Ingest<'_>) {
    let total: f64 = ingest
        .effects()
        .iter()
        .filter(|e| {
            e.quantity == Quantity::Poe1(Poe1Quantity::Devotion)
                && e.op == Op::Base
                && e.conditions == Conditions::none()
        })
        .map(|e| e.value)
        .sum();
    let readers: Vec<_> = ingest
        .effects()
        .iter()
        .filter_map(|e| match e.quantity {
            Quantity::Poe1(Poe1Quantity::DevotionReader(i)) if e.op == Op::Base => {
                Some((i, e.value, e.source.clone()))
            }
            _ => None,
        })
        .collect();
    let per10 = (total / 10.0 + 0.0001).floor();
    for (i, value, source) in readers {
        let Some((_, stat, factor, reading)) = READERS.get(usize::from(i)) else {
            continue;
        };
        let count = match reading {
            Reading::Per10 => per10,
            Reading::AtLeast150 => f64::from(u8::from(total >= THRESHOLD)),
        };
        let scaled = value * count * factor;
        if scaled == 0.0 {
            continue;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a reader's value times a small count, whole and far below i64's range"
        )]
        let v = scaled.round() as i64;
        ingest.stat(
            &source,
            &StatLine {
                stat: StatId::new(*stat),
                value: StatValue::Exact { min: v, max: v },
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readers_should_be_sorted() {
        assert!(READERS.windows(2).all(|w| w[0].0 < w[1].0));
        assert_eq!(reader("area_damage_+%_per_10_devotion"), Some(1));
        assert_eq!(reader("base_devotion"), None);
    }
}
