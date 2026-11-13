//! Keystones that change the main skill's hit (Phase 3 plan, step 8;
//! P3-8m): Resolute Technique, Elemental Overload, and Bitter Frost.
//!
//! What PoB v2.67.2 does. A keystone node's lines are the passive tree's
//! text (`passive_skill_stat_descriptions.txt`), each read by
//! `Modules/ModParser.lua`:
//! - **Resolute Technique** (`resolute_technique`): "Your hits can't be
//!   Evaded" is the `CannotBeEvaded` flag (2216), and `calcs.offence`
//!   then sets an attack's `AccuracyHitChance` to 100
//!   (`Modules/CalcOffence.lua` 2162); "Never deal Critical Strikes" is the
//!   `NeverCrit` flag and `Condition:NeverCrit` (2219), and the crit chance
//!   and crit multiplier are then 0 and the crit effect 1 (2955 to 2960).
//! - **Elemental Overload** (`keystone_elemental_overload`): "Skills that
//!   have dealt a Critical Strike in the past 8 seconds deal 40% more
//!   Elemental Damage with Hits and Ailments" is `ElementalDamage` MORE 40
//!   with the `Hit` and `Ailment` keyword flags (one of them, 992) and
//!   `Condition:CritInPast8Sec` (1307), a skill condition `calcs.offence`
//!   sets when the skill's final crit chance (after its chance to hit) is
//!   not 0 (3184), before the hit's damage; "Your Critical Strikes do not
//!   deal extra Damage" is the `NoCritMultiplier` flag (3517): the crit
//!   multiplier is then 1 (3115); "Ailments never count as being from
//!   Critical Strikes" (`AilmentsAreNeverFromCrit`, 2227) reaches no slice
//!   field.
//! - **Bitter Frost** (`keystone_cold_purist`): "Enemies Chilled by your
//!   Hits have Cold Damage taken increased by Chill Effect" is the
//!   `ChillEffectIncColdDamageTaken` flag (3823), "Enemies in your Chilling
//!   Areas have Cold Damage taken increased by Chill Effect"
//!   `ChillingAreaIncColdDamageTaken` while the enemy is in a chilling
//!   area (3824), which `calcs.perform` reads only without the first flag
//!   (`Modules/CalcPerform.lua` 3438 to 3445), so it adds nothing here;
//!   "Cannot deal non-Cold Damage" is `DealNoPhysical`, `DealNoFire`,
//!   `DealNoLightning`, and `DealNoChaos` (5256, `dealNoNonDamageType`):
//!   the hit deals none of those types (`canDeal`, `CalcOffence.lua` 1874,
//!   3319), though they still convert.
//!
//! What the engine does: each keystone stat maps to its effects
//! ([`KEYSTONES`]): flags on [`Poe1Quantity::CannotBeEvaded`],
//! [`Poe1Quantity::NeverCrit`], [`Poe1Quantity::NoCritMultiplier`],
//! [`Poe1Quantity::DealNo`], and [`Poe1Quantity::ChillDamageTaken`], which
//! [`super::hit`] and [`super::damage_taken`] read, and Elemental
//! Overload's more damage under [`CRIT_IN_PAST_8_SEC`], which the hit
//! settles from its crit chance. `Condition:NeverCrit` only conditions
//! modifiers ("... if you never deal critical strikes"); no mapped row
//! reads it (a test), so such a stat stays a reason.
//!
//! `CritInPast8Sec` is settled in the hit's damage passes only: another
//! query of a stat conditioned on it ("if you've dealt a critical strike
//! in the past 8 seconds": accuracy, damage over time) would meet it
//! unsettled, a reason, until its own context settles it. PoB sets the
//! condition on the skill's list and never clears it, so with dual
//! wielding (not modelled) the main hand's crit would hold it for the off
//! hand's pass too: a per-pass reading would then differ from PoB.

use crate::condition::{ConditionKey, Conditions, Flags};
use crate::ingest::EffectTemplate;
use crate::quantity::{DamageType, Op, Quantity};

use super::Poe1Quantity;

/// The skill condition Elemental Overload's more damage needs (PoB's
/// `CritInPast8Sec`): settled by the hit, true when its crit chance is not
/// 0.
pub const CRIT_IN_PAST_8_SEC: &str = "CritInPast8Sec";

/// Elemental Overload's more elemental damage, percent (the keystone's
/// text).
pub const ELEMENTAL_OVERLOAD_MORE: f64 = 40.0;

/// One effect of a keystone: quantity, op, value, the flags it needs (all
/// of the first, one of the second when not empty), and the named
/// condition it needs to be true.
pub type KeystoneEffect = (Quantity, Op, f64, Flags, Flags, Option<&'static str>);

const fn flag(q: Poe1Quantity) -> KeystoneEffect {
    (
        Quantity::Poe1(q),
        Op::Flag,
        1.0,
        Flags::NONE,
        Flags::NONE,
        None,
    )
}

/// The keystones, sorted by stat ID (a test checks the order), each read as
/// PoB reads its lines (module docs). A keystone stat's value is 1, so each
/// value is the effect's.
pub const KEYSTONES: &[(&str, &[KeystoneEffect])] = &[
    (
        "keystone_cold_purist",
        &[
            flag(Poe1Quantity::ChillDamageTaken(Some(DamageType::Cold))),
            flag(Poe1Quantity::DealNo(DamageType::Physical)),
            flag(Poe1Quantity::DealNo(DamageType::Fire)),
            flag(Poe1Quantity::DealNo(DamageType::Lightning)),
            flag(Poe1Quantity::DealNo(DamageType::Chaos)),
        ],
    ),
    (
        "keystone_elemental_overload",
        &[
            (
                Quantity::Damage,
                Op::More,
                ELEMENTAL_OVERLOAD_MORE,
                Flags::HIT,
                Flags::ELEMENTAL,
                Some(CRIT_IN_PAST_8_SEC),
            ),
            flag(Poe1Quantity::NoCritMultiplier),
        ],
    ),
    (
        "resolute_technique",
        &[
            flag(Poe1Quantity::CannotBeEvaded),
            flag(Poe1Quantity::NeverCrit),
        ],
    ),
];

/// The effects of a keystone stat of [`KEYSTONES`].
#[must_use]
pub fn effects(stat: &str) -> Option<Vec<EffectTemplate>> {
    let i = KEYSTONES.binary_search_by(|(id, _)| (*id).cmp(stat)).ok()?;
    Some(
        KEYSTONES[i]
            .1
            .iter()
            .map(|&(quantity, op, value, all, any, state)| {
                let mut conditions = Conditions::none().with_all(all).with_any(any);
                if let Some(state) = state {
                    conditions = conditions.with_state(ConditionKey::new(state), true);
                }
                EffectTemplate {
                    quantity,
                    op,
                    scale: value,
                    divisor: 1.0,
                    conditions,
                }
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{StatMap, StatMapping};
    use crate::poe1::mapping::{Poe1Mapping, STATE_ROWS};
    use loom_domain::StatId;

    #[test]
    fn the_keystones_should_be_sorted() {
        assert!(KEYSTONES.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn the_mapping_should_give_each_keystone_its_effects() {
        for (stat, rows) in KEYSTONES {
            let StatMap::Effects(t) = Poe1Mapping.map(&StatId::new(*stat)) else {
                panic!("{stat}");
            };
            assert_eq!(t.len(), rows.len(), "{stat}");
        }
        let StatMap::Effects(t) = Poe1Mapping.map(&StatId::new("keystone_elemental_overload"))
        else {
            panic!("elemental overload");
        };
        assert_eq!(t[0].quantity, Quantity::Damage);
        assert_eq!(t[0].op, Op::More);
        assert_eq!(t[0].scale, ELEMENTAL_OVERLOAD_MORE);
        assert_eq!(
            t[0].conditions.states,
            vec![(ConditionKey::new(CRIT_IN_PAST_8_SEC), true)]
        );
        assert_eq!(t[0].conditions.any, Flags::ELEMENTAL);
    }

    /// No mapped row conditions on `NeverCrit` (PoB's
    /// `Condition:NeverCrit`, which Resolute Technique also sets): such a
    /// stat would need the condition settled.
    #[test]
    fn no_row_should_read_never_crit() {
        for (stat, rows) in STATE_ROWS {
            for (.., state) in *rows {
                assert_ne!(*state, "NeverCrit", "{stat}");
            }
        }
        for (stat, rows) in KEYSTONES {
            for (.., state) in *rows {
                assert_ne!(*state, Some("NeverCrit"), "{stat}");
            }
        }
    }
}
