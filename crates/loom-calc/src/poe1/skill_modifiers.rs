//! Modifiers of the other gems' reservation and aura effect (Phase 3 plan,
//! step 8; P3-8l): "#% increased Mana Reservation Efficiency of Skills" and
//! kin, "#% increased Reservation of Skills", and "#% increased effect of
//! Non-Curse Auras from your Skills". Until P3-8l each was a reason
//! ([`super::gems::DEPENDS_ON`]), so [`super::gems`] reserved without
//! modifiers and added every buff unscaled.
//!
//! What PoB v2.67.2 does (line numbers of the pinned checkout):
//! - **Reading.** `Modules/ModParser.lua` reads "mana (life) reservation
//!   efficiency of skills" as `ManaReservationEfficiency`
//!   (`LifeReservationEfficiency`), "reservation efficiency of skills" as
//!   `ReservationEfficiency`, and "reservation of skills" as `Reserved`,
//!   each INC with no tag (222 to 232). It reads "effect of non-curse auras
//!   from your skills" as `AuraEffect` INC tagged with the skill types
//!   `Aura` and not `AppliesCurse` (457).
//! - **Reservation** (`Modules/CalcPerform.lua` 1965 to 1989), per
//!   reserving skill and pool: `inc` is the INC sum of `<Pool>Reserved` and
//!   `Reserved`, `more` their MORE product, `efficiency` the INC sum of
//!   `<Pool>ReservationEfficiency` and `ReservationEfficiency` at least
//!   -100, `efficiencyMore` their MORE product. With `more > 0`,
//!   `inc > -100`, and a base above 0, the flat amount is
//!   `round(floor(flat x mult) x (100 + inc) / 100 x more / (1 + efficiency
//!   / 100) / efficiencyMore, 0)` and the percent the same from
//!   `percent x mult`, rounded to 2 decimals; each at least 0. Otherwise
//!   the skill reserves nothing of that kind.
//! - **Aura effect** (2392 to 2424): an `Aura` buff's mods are scaled by
//!   `(1 + inc / 100) x more`, where `inc` sums `AuraEffect`, `BuffEffect`,
//!   `BuffEffectOnSelf`, `AuraEffectOnSelf`, `AuraBuffEffect`, and
//!   `SkillAuraEffectOnSelf` INC of the aura skill's list (its tags
//!   matched against the skill's types). A `Buff` buff (2266 to 2281) reads
//!   no `AuraEffect`. Each skill's buff is scaled **before** buffs of one
//!   name merge (`mergeBuff` keeps each mod's largest scaled value).
//! - **Scaling a mod** (`Classes/ModStore.lua` `ScaleAddMod`, 45 to 80): at
//!   a scale of 1 the mod is added unchanged. Otherwise a mod whose name
//!   and type are in `data.highPrecisionMods` (`Modules/Data.lua` 430 on;
//!   of the engine's buff rows only `CritChance` BASE, precision 2) or
//!   whose value is not whole (precision 1) becomes
//!   `floor(value x scale x 10^p) / 10^p`; any other becomes
//!   `modf(round(value x scale, 2))`, its integer part toward zero.
//!
//! **The engine.** [`STATS`] maps the seven stats to
//! [`Poe1Quantity::ReservationEfficiency`], [`Poe1Quantity::Reserved`],
//! and [`Poe1Quantity::NonCurseAuraEffect`] (`Increased`, unconditional:
//! a test). [`Totals::read`] sums them once every source of them is in
//! (passives, items, config; [`super::collect`] calls [`super::gems`]
//! after those); [`super::gems`] gives each reserving skill
//! [`Totals::reservation`] and scales each aura's rows by
//! [`Totals::buff_scale`] with [`scale`] before the merge. Only these
//! stats are read: every other stat PoB reads as one of these mods (a
//! skill's own, "Mana Reservation of Skills", buff effect, the MORE ones)
//! stays a reason ([`super::gems::DEPENDS_ON`]), so with one of them the
//! fields stay `Partial` and the sums here are PoB's.
//!
//! **Left as reasons.** "#% increased Mana (Life) Reservation of Skills"
//! (`base_mana_reservation_+%`, `base_life_reservation_+%`): PoB tags them
//! with the `Aura` skill type ("mana reservation of skills", 221), so it
//! would not apply them to a herald or Arctic Armour, while the game's
//! text names every skill. Applying either reading without evidence of
//! the game's would be a guess. "#% increased Effect of Buffs on you"
//! (`buff_effect_on_self_+%`): PoB also reads it for Onslaught, Arcane
//! Surge, Fortification, and others (`CalcPerform.lua` 676 to 826), which
//! [`super::buffs`] and [`super::charges`] do not scale yet.
//!
//! Provenance: a scaled buff row or a reservation carries its gem's source;
//! the modifiers that scaled it are no graph node (a documented gap, as
//! the charges' count and the shock's minimum sources).

use std::collections::BTreeSet;

use super::{Poe1Quantity, Pool};
use crate::ingest::Ingest;
use crate::quantity::{Bound, DamageType, Op, Quantity};
use crate::store::Conditions;

/// One stat's quantity (`Increased`) and scale.
type ModifierRow = (&'static str, Poe1Quantity, f64);

/// The stats (module docs), sorted by stat ID. Each description is the
/// game's (`stat_descriptions.txt`), read by PoB's pattern:
/// - "#% increased Life (Mana) Reservation Efficiency of Skills";
/// - "#% increased Reservation of Skills";
/// - "#% increased Reservation Efficiency of Skills";
/// - "#% reduced Mana Reservation Efficiency of Skills" and "#% reduced
///   Reservation Efficiency of Skills" for the `-2%_per_1` stats, whose
///   descriptions double the value (`double`, `negate_and_double`): PoB
///   reads the line's number, so the stat acts at -2 per point;
/// - "#% increased effect of Non-Curse Auras from your Skills".
pub const STATS: [ModifierRow; 7] = [
    (
        "base_life_reservation_efficiency_+%",
        Poe1Quantity::ReservationEfficiency(Some(Pool::Life)),
        1.0,
    ),
    (
        "base_mana_reservation_efficiency_+%",
        Poe1Quantity::ReservationEfficiency(Some(Pool::Mana)),
        1.0,
    ),
    ("base_reservation_+%", Poe1Quantity::Reserved(None), 1.0),
    (
        "base_reservation_efficiency_+%",
        Poe1Quantity::ReservationEfficiency(None),
        1.0,
    ),
    (
        "mana_reservation_efficiency_-2%_per_1",
        Poe1Quantity::ReservationEfficiency(Some(Pool::Mana)),
        -2.0,
    ),
    (
        "non_curse_aura_effect_+%",
        Poe1Quantity::NonCurseAuraEffect,
        1.0,
    ),
    (
        "reservation_efficiency_-2%_per_1",
        Poe1Quantity::ReservationEfficiency(None),
        -2.0,
    ),
];

/// The effects of a stat of [`STATS`]: `(quantity, op, scale)`.
#[must_use]
pub fn effects(stat: &str) -> Option<Vec<(Quantity, Op, f64)>> {
    let i = STATS.binary_search_by(|(id, ..)| (*id).cmp(stat)).ok()?;
    let (_, q, scale) = STATS[i];
    Some(vec![(Quantity::Poe1(q), Op::Increased, scale)])
}

/// The sums of [`STATS`]' quantities.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Totals {
    /// `ReservationEfficiency` INC (no pool).
    pub efficiency: f64,
    /// `LifeReservationEfficiency` INC.
    pub life_efficiency: f64,
    /// `ManaReservationEfficiency` INC.
    pub mana_efficiency: f64,
    /// `Reserved` INC (no pool).
    pub reserved: f64,
    /// `AuraEffect` INC of non-curse aura skills.
    pub aura_effect: f64,
}

/// A skill's reservation terms for one pool (PoB's `values`, module docs).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReservationTerms {
    /// `values.inc`.
    pub inc: f64,
    /// `values.efficiency`, at least -100.
    pub efficiency: f64,
}

impl ReservationTerms {
    /// No modifier.
    pub const NONE: Self = Self {
        inc: 0.0,
        efficiency: 0.0,
    };
}

impl Totals {
    /// The sums of the unconditional `Increased` effects ingested so far.
    /// Every row is unconditional (a test), so a condition can only come
    /// from a later row, which must bring its handling here.
    #[must_use]
    pub fn read(ingest: &Ingest<'_>) -> Self {
        let mut t = Self::default();
        for e in ingest.effects() {
            if e.op != Op::Increased || e.conditions != Conditions::none() {
                continue;
            }
            let Quantity::Poe1(q) = e.quantity else {
                continue;
            };
            let sum = match q {
                Poe1Quantity::ReservationEfficiency(None) => &mut t.efficiency,
                Poe1Quantity::ReservationEfficiency(Some(Pool::Life)) => &mut t.life_efficiency,
                Poe1Quantity::ReservationEfficiency(Some(Pool::Mana)) => &mut t.mana_efficiency,
                Poe1Quantity::Reserved(None) => &mut t.reserved,
                Poe1Quantity::NonCurseAuraEffect => &mut t.aura_effect,
                _ => continue,
            };
            *sum += e.value;
        }
        t
    }

    /// The reservation terms of `pool` (module docs): the pool's and the
    /// generic efficiency summed, at least -100.
    #[must_use]
    pub fn reservation(&self, pool: Pool) -> ReservationTerms {
        let own = match pool {
            Pool::Life => self.life_efficiency,
            Pool::Mana => self.mana_efficiency,
        };
        ReservationTerms {
            inc: self.reserved,
            efficiency: crate::math::max(own + self.efficiency, -100.0),
        }
    }

    /// The scale of a buff of `effect` from a skill of `types` (module
    /// docs): `1 + inc / 100` for an aura of a skill typed `Aura` and not
    /// `AppliesCurse`, else 1.
    #[must_use]
    pub fn buff_scale(&self, aura: bool, types: &BTreeSet<String>) -> f64 {
        if aura && types.contains("Aura") && !types.contains("AppliesCurse") {
            1.0 + self.aura_effect / 100.0
        } else {
            1.0
        }
    }
}

/// PoB's precision for scaling a mod of this quantity and op
/// (`data.highPrecisionMods`), among the quantities buff rows reach.
fn high_precision(quantity: Quantity, op: Op) -> Option<u8> {
    match (quantity, op) {
        (Quantity::CritChance, Op::Base) => Some(2),
        _ => None,
    }
}

/// A mod's value scaled as PoB's `ScaleAddMod` (module docs).
#[must_use]
pub fn scale(quantity: Quantity, op: Op, value: f64, scale: f64) -> f64 {
    if scale == 1.0 {
        return value;
    }
    let precision = high_precision(quantity, op).or_else(|| {
        // `data.defaultHighPrecision` for a value that is not whole.
        (crate::math::floor(value) != value).then_some(1)
    });
    if let Some(p) = precision {
        let power = 10f64.powi(i32::from(p));
        return crate::math::floor(value * scale * power) / power;
    }
    // `m_modf(round(value x scale, 2))`: the integer part, toward zero.
    let rounded = crate::math::floor(value * scale * 100.0 + 0.5) / 100.0;
    crate::math::trunc(rounded)
}

/// The main skill's own aura stats, by granted effect ID: PoB's stat map
/// gives them a `GlobalEffect` aura tag, so they are a buff on the
/// character, scaled as any aura (module docs), though the engine reads
/// them through the hit ([`super::mapping::SkillMapping`]). Smite's
/// (`Data/Skills/act_str.lua`, `skills["Smite"]`: `LightningMin` and
/// `LightningMax` BASE, `effectType = "Aura"`), each with the quantity it
/// maps to (for [`scale`]'s precision).
pub const MAIN_SKILL_AURA_STATS: &[(&str, &str, Quantity)] = &[
    (
        "Smite",
        "maximum_added_lightning_damage_from_skill",
        Quantity::AddedDamage(DamageType::Lightning, Bound::Max),
    ),
    (
        "Smite",
        "minimum_added_lightning_damage_from_skill",
        Quantity::AddedDamage(DamageType::Lightning, Bound::Min),
    ),
];

/// A main skill instance's stat value as its buff applies it: scaled by
/// `buff_scale` when it is one of [`MAIN_SKILL_AURA_STATS`], else as is.
/// The scaled value is whole ([`scale`] takes the integer part of a whole
/// value's product), so it stays a stat value.
#[must_use]
pub fn main_skill_stat(effect: &str, stat: &str, value: i64, buff_scale: f64) -> i64 {
    let Some(&(_, _, quantity)) = MAIN_SKILL_AURA_STATS
        .iter()
        .find(|(e, s, _)| *e == effect && *s == stat)
    else {
        return value;
    };
    #[expect(
        clippy::cast_precision_loss,
        reason = "stat values are small integers, exact as f64"
    )]
    let scaled = scale(quantity, Op::Base, value as f64, buff_scale);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a whole value's scale is whole and small"
    )]
    let scaled = scaled as i64;
    scaled
}

/// One skill's reservation of one pool, as PoB computes it (module docs):
/// the flat amount and the percent. `None` when the efficiency is -100,
/// where PoB divides by 0.
#[must_use]
pub fn reserved(
    flat: i64,
    percent_hundredths: i64,
    mult: f64,
    terms: ReservationTerms,
) -> Option<(f64, f64)> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "reservations are small integers, exact as f64"
    )]
    let (flat, percent) = (flat as f64, percent_hundredths as f64 / 100.0);
    let ReservationTerms { inc, efficiency } = terms;
    if efficiency <= -100.0 {
        return None;
    }
    // `values.more` and `efficiencyMore` are 1: every MORE stat is a
    // reason.
    let (more, efficiency_more) = (1.0, 1.0);
    let applies = more > 0.0 && inc > -100.0;
    let flat_value = crate::math::floor(flat * mult);
    let reserved_flat = if !applies || flat_value == 0.0 {
        0.0
    } else {
        let x = flat_value * (100.0 + inc) / 100.0 * more
            / (1.0 + efficiency / 100.0)
            / efficiency_more;
        crate::math::max(crate::math::round_half_up(x), 0.0)
    };
    let percent_value = percent * mult;
    let reserved_percent = if !applies || percent_value == 0.0 {
        0.0
    } else {
        let x = percent_value * (100.0 + inc) / 100.0 * more
            / (1.0 + efficiency / 100.0)
            / efficiency_more;
        // PoB's `round(x, 2)`: `floor(x x 10^2 + 0.5) / 10^2`.
        crate::math::max(crate::math::floor(x * 100.0 + 0.5) / 100.0, 0.0)
    };
    Some((reserved_flat, reserved_percent))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{StatMap, StatMapping};
    use crate::poe1::ArmourSlot;
    use crate::poe1::mapping::{PieceMapping, Poe1Mapping, SkillMapping, WeaponMapping};
    use loom_domain::StatId;

    #[test]
    fn stats_should_be_sorted_and_not_guarded() {
        assert!(STATS.windows(2).all(|w| w[0].0 < w[1].0));
        for (stat, ..) in STATS {
            assert!(
                !super::super::gems::DEPENDS_ON.contains(&stat),
                "{stat}: mapped here, so not a reason"
            );
        }
    }

    #[test]
    fn the_character_mapping_should_read_every_row_unconditionally() {
        for (stat, q, scale) in STATS {
            let StatMap::Effects(effects) = Poe1Mapping.map(&StatId::new(stat)) else {
                panic!("{stat} is not mapped");
            };
            assert_eq!(effects.len(), 1, "{stat}");
            let t = &effects[0];
            assert_eq!(t.quantity, Quantity::Poe1(q), "{stat}");
            assert_eq!(t.op, Op::Increased, "{stat}");
            assert!((t.scale - scale).abs() < f64::EPSILON, "{stat}");
            assert!((t.divisor - 1.0).abs() < f64::EPSILON, "{stat}");
            assert_eq!(t.conditions, Conditions::none(), "{stat}");
        }
    }

    /// A gem's own modifiers act on its skill alone (PoB's `skillModList`):
    /// through the main skill's mapping they stay reasons, and other
    /// groups' gems never reach a mapping.
    #[test]
    fn a_gems_modifiers_should_be_reasons() {
        for (stat, ..) in STATS {
            assert_eq!(
                SkillMapping.map(&StatId::new(stat)),
                StatMap::Unmapped,
                "{stat}"
            );
        }
    }

    /// Each stat left (module docs) stays a reason through every mapping.
    #[test]
    fn the_stats_left_should_stay_unmapped() {
        let piece = PieceMapping {
            slot: ArmourSlot::Helmet,
        };
        let mappings: [&dyn StatMapping; 4] = [&Poe1Mapping, &SkillMapping, &piece, &WeaponMapping];
        for stat in [
            "aura_effect_+%",
            "base_life_reservation_+%",
            "base_mana_reservation_+%",
            "buff_effect_on_self_+%",
        ] {
            assert!(super::super::gems::DEPENDS_ON.contains(&stat), "{stat}");
            for m in mappings {
                assert_eq!(m.map(&StatId::new(stat)), StatMap::Unmapped, "{stat}");
            }
        }
    }

    #[test]
    fn reservation_should_round_as_pob() {
        let none = ReservationTerms::NONE;
        // Without modifiers, as P3-12h.
        assert_eq!(reserved(32, 0, 1.0, none), Some((32.0, 0.0)));
        assert_eq!(reserved(15, 0, 1.3, none), Some((19.0, 0.0)));
        assert_eq!(reserved(0, 2500, 1.2, none), Some((0.0, 30.0)));
        assert_eq!(reserved(0, 2250, 1.1, none), Some((0.0, 24.75)));
        assert_eq!(reserved(0, 0, 1.5, none), Some((0.0, 0.0)));
        // Efficiency divides: 50% / 1.12 = 44.642..., rounded to 44.64.
        let eff = |efficiency| ReservationTerms {
            inc: 0.0,
            efficiency,
        };
        assert_eq!(reserved(0, 5000, 1.0, eff(12.0)), Some((0.0, 44.64)));
        // 25 / 1.3 = 19.23..., rounded half up to 19.
        assert_eq!(reserved(25, 0, 1.0, eff(30.0)), Some((19.0, 0.0)));
        // A negative efficiency raises it: 35% / 0.9 = 38.888... to 38.89.
        assert_eq!(reserved(0, 3500, 1.0, eff(-10.0)), Some((0.0, 38.89)));
        // At -100 PoB divides by 0.
        assert_eq!(reserved(0, 3500, 1.0, eff(-100.0)), None);
        // `inc` multiplies, and at -100 or below the skill reserves nothing.
        let inc = |inc| ReservationTerms {
            inc,
            efficiency: 0.0,
        };
        assert_eq!(reserved(0, 5000, 1.0, inc(-20.0)), Some((0.0, 40.0)));
        assert_eq!(reserved(40, 5000, 1.0, inc(-100.0)), Some((0.0, 0.0)));
        // Both: 50 x 0.9 / 1.1 = 40.909... to 40.91.
        let both = ReservationTerms {
            inc: -10.0,
            efficiency: 10.0,
        };
        assert_eq!(reserved(0, 5000, 1.0, both), Some((0.0, 40.91)));
    }

    #[test]
    fn the_efficiency_should_sum_the_pool_and_generic_and_stop_at_minus_100() {
        let t = Totals {
            efficiency: 5.0,
            life_efficiency: 3.0,
            mana_efficiency: 10.0,
            reserved: -4.0,
            aura_effect: 0.0,
        };
        assert_eq!(
            t.reservation(Pool::Mana),
            ReservationTerms {
                inc: -4.0,
                efficiency: 15.0
            }
        );
        assert_eq!(t.reservation(Pool::Life).efficiency, 8.0);
        let low = Totals {
            mana_efficiency: -150.0,
            ..Totals::default()
        };
        assert_eq!(low.reservation(Pool::Mana).efficiency, -100.0);
    }

    #[test]
    fn aura_effect_should_scale_non_curse_auras_only() {
        let t = Totals {
            aura_effect: 25.0,
            ..Totals::default()
        };
        let types = |names: &[&str]| {
            names
                .iter()
                .map(|s| (*s).to_owned())
                .collect::<BTreeSet<_>>()
        };
        assert_eq!(t.buff_scale(true, &types(&["Aura", "Buff"])), 1.25);
        assert_eq!(t.buff_scale(true, &types(&["Aura", "AppliesCurse"])), 1.0);
        // A buff (a herald) reads no aura effect, nor an aura without the
        // type.
        assert_eq!(t.buff_scale(false, &types(&["Aura"])), 1.0);
        assert_eq!(t.buff_scale(true, &types(&["Buff"])), 1.0);
    }

    #[test]
    fn a_mod_should_scale_as_pob() {
        let acc = Quantity::Accuracy;
        // Scale 1: unchanged.
        assert_eq!(scale(acc, Op::Base, 117.0, 1.0), 117.0);
        // Whole values: rounded to 2 decimals, then toward zero.
        assert_eq!(scale(acc, Op::Base, 117.0, 1.25), 146.0);
        assert_eq!(scale(acc, Op::Base, 15.0, 1.1), 16.0);
        assert_eq!(scale(acc, Op::Base, 30.0, 1.33), 39.0);
        assert_eq!(scale(Quantity::Damage, Op::More, -9.0, 1.5), -13.0);
        // 1 x 2.996 rounds to 3.0 at 2 decimals before the integer part.
        assert_eq!(scale(acc, Op::Base, 1.0, 2.996), 3.0);
        assert_eq!(scale(acc, Op::Base, 1.0, 2.99), 2.0);
        // Not whole: floored at 1 decimal.
        assert_eq!(scale(acc, Op::Base, 2.5, 1.3), 3.2);
        // High precision: floored at 2 decimals.
        assert_eq!(scale(Quantity::CritChance, Op::Base, 1.0, 1.255), 1.25);
        assert_eq!(
            scale(Quantity::CritChance, Op::Increased, 50.0, 1.255),
            62.0
        );
    }

    #[test]
    fn the_main_skills_aura_stats_should_scale_alone() {
        let min = "minimum_added_lightning_damage_from_skill";
        // Smite's aura: 1 to 19 at 1.1 gives 1 (1.1) and 20 (20.9).
        assert_eq!(main_skill_stat("Smite", min, 1, 1.1), 1);
        assert_eq!(
            main_skill_stat(
                "Smite",
                "maximum_added_lightning_damage_from_skill",
                19,
                1.1
            ),
            20
        );
        assert_eq!(main_skill_stat("Smite", min, 19, 1.0), 19);
        // Another skill's stat, or another stat of Smite: as is.
        assert_eq!(main_skill_stat("IceNova", min, 19, 1.1), 19);
        assert_eq!(main_skill_stat("Smite", "attack_speed_+%", 10, 1.1), 10);
        for (_, stat, quantity) in MAIN_SKILL_AURA_STATS {
            let StatMap::Effects(e) = SkillMapping.map(&StatId::new(*stat)) else {
                panic!("{stat}");
            };
            assert_eq!(e.len(), 1, "{stat}");
            assert_eq!(e[0].quantity, *quantity, "{stat}");
            assert_eq!(e[0].op, Op::Base, "{stat}");
        }
    }

    #[test]
    fn totals_should_read_unconditional_increases_only() {
        use crate::source::Source;
        use crate::store::Effect;
        let mut ingest = Ingest::new(&Poe1Mapping);
        let effect = |q, op, value, conditions| Effect {
            quantity: Quantity::Poe1(q),
            op,
            value,
            conditions,
            source: Source::Rule("test"),
        };
        ingest.effect(effect(
            Poe1Quantity::ReservationEfficiency(Some(Pool::Mana)),
            Op::Increased,
            12.0,
            Conditions::none(),
        ));
        ingest.effect(effect(
            Poe1Quantity::ReservationEfficiency(Some(Pool::Mana)),
            Op::Increased,
            8.0,
            Conditions::none(),
        ));
        ingest.effect(effect(
            Poe1Quantity::NonCurseAuraEffect,
            Op::Increased,
            10.0,
            Conditions::none(),
        ));
        ingest.effect(effect(
            Poe1Quantity::NonCurseAuraEffect,
            Op::More,
            50.0,
            Conditions::none(),
        ));
        let t = Totals::read(&ingest);
        assert_eq!(t.mana_efficiency, 20.0);
        assert_eq!(t.aura_effect, 10.0);
        assert_eq!(t.efficiency, 0.0);
    }

    #[test]
    fn names_should_be_stable() {
        for (q, name) in [
            (
                Poe1Quantity::ReservationEfficiency(None),
                "poe1.reservation-efficiency",
            ),
            (
                Poe1Quantity::ReservationEfficiency(Some(Pool::Mana)),
                "poe1.reservation-efficiency.mana",
            ),
            (Poe1Quantity::Reserved(None), "poe1.reserved"),
            (
                Poe1Quantity::Reserved(Some(Pool::Life)),
                "poe1.reserved.life",
            ),
            (
                Poe1Quantity::NonCurseAuraEffect,
                "poe1.non-curse-aura-effect",
            ),
        ] {
            assert_eq!(Quantity::Poe1(q).to_string(), name);
        }
    }
}
