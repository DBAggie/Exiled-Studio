//! The Path of Exile stat-to-effect mapping (ADR-015; Phase 3 plan,
//! decision 2 and step 8): data rows, one per stat, each with a test.
//!
//! P3-6 maps the global stats of attributes, life, mana, and energy
//! shield. Each row names the stat's meaning as the game's stat
//! description writes it, and how PoB v2.67.2 reads the same line
//! (`Modules/ModParser.lua`), so a difference in reading shows in review.
//! Every other stat is `Unmapped`, which makes the computed fields
//! `Partial` (never a silent zero).

use loom_domain::StatId;

use crate::condition::Conditions;
use crate::ingest::{EffectTemplate, StatMap, StatMapping};
use crate::quantity::{Op, Quantity};

/// One mapped effect: what, how, and the scale per unit of the stat.
type Row = (&'static str, &'static [(Quantity, Op, f64)]);

const STR: Quantity = Quantity::Strength;
const DEX: Quantity = Quantity::Dexterity;
const INT: Quantity = Quantity::Intelligence;

/// The mapped stats, sorted by stat ID (a test checks the order).
///
/// - `base_maximum_life`: "+# to maximum Life" (PoB `Life` BASE).
/// - `maximum_life_+%`: "#% increased maximum Life" (`Life` INC).
/// - `base_maximum_mana`, `maximum_mana_+%`: the same for mana.
/// - `base_maximum_energy_shield`: "+# to maximum Energy Shield", the
///   global stat (an armour piece's own is `local_energy_shield`, P3-7)
///   (`EnergyShield` BASE).
/// - `maximum_energy_shield_+%`: "#% increased maximum Energy Shield"
///   (`EnergyShield` INC).
/// - `global_defences_+%`: "#% increased Global Defences" (PoB
///   `Defences` INC, which its energy shield, armour, and evasion sums
///   read).
/// - `additional_*`: "+# to Strength" and the pairs and "all Attributes"
///   (PoB `Str`, `Dex`, `Int` BASE; the pairs and all attributes read as
///   each attribute).
/// - `*_+%`: "#% increased Strength", ... (`Str` INC, ...).
pub const ROWS: &[Row] = &[
    (
        "additional_all_attributes",
        &[
            (STR, Op::Base, 1.0),
            (DEX, Op::Base, 1.0),
            (INT, Op::Base, 1.0),
        ],
    ),
    ("additional_dexterity", &[(DEX, Op::Base, 1.0)]),
    (
        "additional_dexterity_and_intelligence",
        &[(DEX, Op::Base, 1.0), (INT, Op::Base, 1.0)],
    ),
    ("additional_intelligence", &[(INT, Op::Base, 1.0)]),
    ("additional_strength", &[(STR, Op::Base, 1.0)]),
    (
        "additional_strength_and_dexterity",
        &[(STR, Op::Base, 1.0), (DEX, Op::Base, 1.0)],
    ),
    (
        "additional_strength_and_intelligence",
        &[(STR, Op::Base, 1.0), (INT, Op::Base, 1.0)],
    ),
    (
        "all_attributes_+%",
        &[
            (STR, Op::Increased, 1.0),
            (DEX, Op::Increased, 1.0),
            (INT, Op::Increased, 1.0),
        ],
    ),
    (
        "base_maximum_energy_shield",
        &[(Quantity::EnergyShield, Op::Base, 1.0)],
    ),
    ("base_maximum_life", &[(Quantity::Life, Op::Base, 1.0)]),
    ("base_maximum_mana", &[(Quantity::Mana, Op::Base, 1.0)]),
    ("dexterity_+%", &[(DEX, Op::Increased, 1.0)]),
    (
        "global_defences_+%",
        &[
            (Quantity::EnergyShield, Op::Increased, 1.0),
            (Quantity::Armour, Op::Increased, 1.0),
            (Quantity::Evasion, Op::Increased, 1.0),
        ],
    ),
    ("intelligence_+%", &[(INT, Op::Increased, 1.0)]),
    (
        "maximum_energy_shield_+%",
        &[(Quantity::EnergyShield, Op::Increased, 1.0)],
    ),
    ("maximum_life_+%", &[(Quantity::Life, Op::Increased, 1.0)]),
    ("maximum_mana_+%", &[(Quantity::Mana, Op::Increased, 1.0)]),
    ("strength_+%", &[(STR, Op::Increased, 1.0)]),
];

/// The Path of Exile stat-to-effect mapping: [`ROWS`]; every other stat
/// is `Unmapped`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Poe1Mapping;

impl StatMapping for Poe1Mapping {
    fn map(&self, stat: &StatId) -> StatMap {
        ROWS.binary_search_by(|(id, _)| (*id).cmp(stat.as_str()))
            .map_or(StatMap::Unmapped, |i| {
                StatMap::Effects(
                    ROWS[i]
                        .1
                        .iter()
                        .map(|&(quantity, op, scale)| EffectTemplate {
                            quantity,
                            op,
                            scale,
                            conditions: Conditions::none(),
                        })
                        .collect(),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effects(stat: &str) -> Vec<(Quantity, Op, f64)> {
        match Poe1Mapping.map(&StatId::new(stat)) {
            StatMap::Effects(e) => e.into_iter().map(|t| (t.quantity, t.op, t.scale)).collect(),
            other => panic!("{stat}: {other:?}"),
        }
    }

    #[test]
    fn rows_should_be_sorted_and_unique() {
        assert!(ROWS.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn an_unknown_stat_should_be_unmapped() {
        assert_eq!(
            Poe1Mapping.map(&StatId::new("local_energy_shield")),
            StatMap::Unmapped
        );
        assert_eq!(Poe1Mapping.map(&StatId::new("")), StatMap::Unmapped);
    }

    #[test]
    fn life_mana_and_energy_shield_should_map_to_base_and_increased() {
        assert_eq!(
            effects("base_maximum_life"),
            [(Quantity::Life, Op::Base, 1.0)]
        );
        assert_eq!(
            effects("maximum_life_+%"),
            [(Quantity::Life, Op::Increased, 1.0)]
        );
        assert_eq!(
            effects("base_maximum_mana"),
            [(Quantity::Mana, Op::Base, 1.0)]
        );
        assert_eq!(
            effects("maximum_mana_+%"),
            [(Quantity::Mana, Op::Increased, 1.0)]
        );
        assert_eq!(
            effects("base_maximum_energy_shield"),
            [(Quantity::EnergyShield, Op::Base, 1.0)]
        );
        assert_eq!(
            effects("maximum_energy_shield_+%"),
            [(Quantity::EnergyShield, Op::Increased, 1.0)]
        );
    }

    #[test]
    fn global_defences_should_increase_each_defence() {
        assert_eq!(
            effects("global_defences_+%"),
            [
                (Quantity::EnergyShield, Op::Increased, 1.0),
                (Quantity::Armour, Op::Increased, 1.0),
                (Quantity::Evasion, Op::Increased, 1.0)
            ]
        );
    }

    #[test]
    fn attribute_stats_should_reach_each_attribute_they_name() {
        assert_eq!(effects("additional_strength"), [(STR, Op::Base, 1.0)]);
        assert_eq!(effects("additional_dexterity"), [(DEX, Op::Base, 1.0)]);
        assert_eq!(effects("additional_intelligence"), [(INT, Op::Base, 1.0)]);
        assert_eq!(
            effects("additional_strength_and_dexterity"),
            [(STR, Op::Base, 1.0), (DEX, Op::Base, 1.0)]
        );
        assert_eq!(
            effects("additional_strength_and_intelligence"),
            [(STR, Op::Base, 1.0), (INT, Op::Base, 1.0)]
        );
        assert_eq!(
            effects("additional_dexterity_and_intelligence"),
            [(DEX, Op::Base, 1.0), (INT, Op::Base, 1.0)]
        );
        assert_eq!(
            effects("additional_all_attributes"),
            [
                (STR, Op::Base, 1.0),
                (DEX, Op::Base, 1.0),
                (INT, Op::Base, 1.0)
            ]
        );
        assert_eq!(effects("strength_+%"), [(STR, Op::Increased, 1.0)]);
        assert_eq!(effects("dexterity_+%"), [(DEX, Op::Increased, 1.0)]);
        assert_eq!(effects("intelligence_+%"), [(INT, Op::Increased, 1.0)]);
        assert_eq!(
            effects("all_attributes_+%"),
            [
                (STR, Op::Increased, 1.0),
                (DEX, Op::Increased, 1.0),
                (INT, Op::Increased, 1.0)
            ]
        );
    }
}
