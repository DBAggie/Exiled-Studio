//! Keystones that change the main skill's hit (Phase 3 plan, step 8;
//! P3-8m): Resolute Technique, Elemental Overload, and Bitter Frost; and
//! Glancing Blows' block (P6-11g, [`KEYSTONES`]); Ancestral Bond and
//! Runebinder (P6-11hv: their totem and brand limits, and Ancestral Bond's
//! "You can't deal Damage with Skills yourself" under
//! [`super::conditions::SKILL_USED_BY_YOU`]).
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
//!   Critical Strikes" (`AilmentsAreNeverFromCrit`, 2227): an ailment's
//!   chance on a crit is then its chance on a hit, and its DoT multiplier
//!   has no critical bonus (Phase 6, [`super::ailment`]).
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
use crate::quantity::{Bound, DamageType, Op, Quantity};

use super::stacks::{StackKind, StackValue};
use super::{Poe1Quantity, Pool, TakenOf, TakenWhen};

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

/// Ancestral Bond's `DealNo` of one type, for a skill used by you
/// (P6-11hv).
const fn deal_no(t: DamageType) -> KeystoneEffect {
    (
        Quantity::Poe1(Poe1Quantity::DealNo(t)),
        Op::Flag,
        1.0,
        Flags::NONE,
        Flags::NONE,
        Some(super::conditions::SKILL_USED_BY_YOU),
    )
}

/// A change to the maximum number of summoned totems (P6-11hv; PoB's
/// `ActiveTotemLimit` BASE, read by [`super::stacks::totems`]).
const fn totems(n: f64) -> KeystoneEffect {
    (
        Quantity::Poe1(Poe1Quantity::Stack(StackKind::Totems, StackValue::Maximum)),
        Op::Base,
        n,
        Flags::NONE,
        Flags::NONE,
        None,
    )
}

/// The keystones, sorted by stat ID (a test checks the order), each read as
/// PoB reads its lines (module docs). A keystone stat's value is 1, so each
/// Divine Shield's pseudo recoup, PoB's value (`ModCache.lua` 4760).
pub const DIVINE_SHIELD_PSEUDO_RECOUP: f64 = 12.0;

/// Mind over Matter's damage taken from mana before life, percent.
pub const MIND_OVER_MATTER: f64 = 40.0;

/// Glancing Blows' damage taken from blocked hits, percent (`ModParser`'s
/// "you take (%d+)%% of damage from blocked hits", `BlockEffect` BASE).
pub const GLANCING_BLOWS_BLOCK_EFFECT: f64 = 65.0;

/// value is the effect's.
pub const KEYSTONES: &[(&str, &[KeystoneEffect])] = &[
    // Blood Magic (P6-11an): the tree's "Removes all mana" (`Mana` MORE
    // -100), "10% more maximum Life" (`Life` MORE 10), "Skills Cost Life
    // instead of Mana" (`CostLifeInsteadOfMana` FLAG), and "Skills Reserve
    // Life instead of Mana" (`BloodMagicReserved` FLAG).
    // Acrobatics (P6-11el): the tree's "Modifiers to Chance to Suppress
    // Spell Damage instead apply to Chance to Dodge Spell Hits at 50% of
    // their value" (`ConvertSpellSuppressionToSpellDodge` FLAG,
    // `SpellSuppressionChance` OVERRIDE 0) and "Maximum Chance to Dodge
    // Spell Hits is 75%" (`SpellDodgeChanceMax` OVERRIDE 75): the flag, read
    // by `phases` and `enemy_hit`.
    (
        "keystone_acrobatics",
        &[flag(Poe1Quantity::SuppressionToSpellDodge)],
    ),
    // Ancestral Bond (P6-11hv): the tree's "You can't deal Damage with
    // Skills yourself" (`DealNoDamage` FLAG unless the skill is a totem, a
    // mine, or a trap, or the mirage's; `ModParser.lua` 5252: every type's
    // `DealNo`, under `conditions::SKILL_USED_BY_YOU`) and "+1 to maximum
    // number of Summoned Totems" (`ActiveTotemLimit` BASE 1).
    (
        "keystone_ancestral_bond",
        &[
            deal_no(DamageType::Physical),
            deal_no(DamageType::Fire),
            deal_no(DamageType::Cold),
            deal_no(DamageType::Lightning),
            deal_no(DamageType::Chaos),
            totems(1.0),
        ],
    ),
    (
        "keystone_blood_magic",
        &[
            (
                Quantity::Mana,
                Op::More,
                -100.0,
                Flags::NONE,
                Flags::NONE,
                None,
            ),
            (
                Quantity::Life,
                Op::More,
                10.0,
                Flags::NONE,
                Flags::NONE,
                None,
            ),
            flag(Poe1Quantity::CostLifeInsteadOfMana),
            flag(Poe1Quantity::ReserveLifeInsteadOfMana),
        ],
    ),
    // Call to Arms (P6-11hw): the tree's "Your Warcries do not grant Buffs or
    // Charges to You" (`CannotGainWarcryBuffs` FLAG, `ModCache.lua` 13113)
    // and "100% more Warcry Duration" (`Duration` MORE 100 with
    // `KeywordFlag.Warcry`, 2929), both read by `poe1::gems`' warcries.
    (
        "keystone_call_to_arms",
        &[
            flag(Poe1Quantity::CannotGainWarcryBuffs),
            (
                Quantity::Poe1(Poe1Quantity::SkillTiming(
                    super::skill_modifiers::SkillTiming::WarcryDuration,
                )),
                Op::More,
                100.0,
                Flags::NONE,
                Flags::NONE,
                None,
            ),
        ],
    ),
    // Chaos Inoculation (P6-11ej): the tree's "Maximum Life becomes 1, Immune
    // to Chaos Damage" (`ChaosInoculation` FLAG, `ChaosDamageTaken` MORE
    // -100; `ModParser.lua` 2242). Its `FullLife` is `states`'.
    (
        "keystone_chaos_inoculation",
        &[
            flag(Poe1Quantity::ChaosInoculation),
            (
                Quantity::Poe1(Poe1Quantity::SelfDamageTaken(
                    TakenWhen::Always,
                    TakenOf::Type(DamageType::Chaos),
                )),
                Op::More,
                -100.0,
                Flags::NONE,
                Flags::NONE,
                None,
            ),
        ],
    ),
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
    // Divine Shield (P6-11am): the tree's "Cannot Recover Energy Shield to
    // above Armour" (`ArmourESRecoveryCap` FLAG) and "3% of Physical Damage
    // prevented from Hits Recently is Regenerated as Energy Shield per
    // second" (`PhysicalDamageMitigatedEnergyShieldPseudoRecoup` BASE 12,
    // `ModCache.lua` 4760).
    (
        "keystone_divine_shield",
        &[
            flag(Poe1Quantity::ArmourEsRecoveryCap),
            (
                Quantity::Poe1(Poe1Quantity::PseudoRecoup),
                Op::Base,
                DIVINE_SHIELD_PSEUDO_RECOUP,
                Flags::NONE,
                Flags::NONE,
                None,
            ),
        ],
    ),
    // Eldritch Battery (P6-11ap): the tree's "Spend Energy Shield before
    // Mana for Skill Mana Costs" (PoB parses it to nothing; the cost moves
    // with the flag), "Energy Shield protects Mana instead of Life"
    // (`EnergyShieldProtectsMana` FLAG), and "50% less Energy Shield
    // Recharge Rate" (`EnergyShieldRecharge` MORE -50).
    (
        "keystone_eldritch_battery",
        &[
            flag(Poe1Quantity::EsProtectsMana),
            (
                Quantity::Poe1(Poe1Quantity::EnergyShieldRecharge),
                Op::More,
                -50.0,
                Flags::NONE,
                Flags::NONE,
                None,
            ),
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
            // PoB's keyword flags are `Hit` or `Ailment`: the same more for
            // ailments (Phase 6, P6-2b). A hit's context never has the
            // ailment flag, an ailment's never the hit's.
            (
                Quantity::Damage,
                Op::More,
                ELEMENTAL_OVERLOAD_MORE,
                Flags::AILMENT,
                Flags::ELEMENTAL,
                Some(CRIT_IN_PAST_8_SEC),
            ),
            flag(Poe1Quantity::NoCritMultiplier),
            flag(Poe1Quantity::AilmentsNeverFromCrit),
        ],
    ),
    // Glancing Blows (P6-11g): "Chance to Block Attack Damage is doubled"
    // is `BlockChance` MORE 100, which PoB's spell block also reads
    // (`CalcDefence.lua` 709 to 713 take the `BlockChance` increases and
    // mores); its "Chance to Block Spell Damage is doubled"
    // (`SpellBlockChance` MORE) only scales projectile spell block, and "You
    // take 65% of Damage from Blocked Hits" is `BlockEffect` BASE 65, which
    // the effective health pool reads (P6-10b part 14).
    // Eternal Youth (P6-11ao): the tree's "50% less Life Regeneration Rate"
    // (`LifeRegen` MORE -50), "50% less maximum Total Life Recovery per
    // Second from Leech" (`MaxLifeLeechRate` MORE -50), and "Energy Shield
    // Recharge instead applies to Life" (`EnergyShieldRechargeAppliesToLife`).
    (
        "keystone_eternal_youth",
        &[
            (
                Quantity::Poe1(Poe1Quantity::Regen(Pool::Life)),
                Op::More,
                -50.0,
                Flags::NONE,
                Flags::NONE,
                None,
            ),
            (
                Quantity::Poe1(Poe1Quantity::LeechRateMax(Pool::Life)),
                Op::More,
                -50.0,
                Flags::NONE,
                Flags::NONE,
                None,
            ),
            flag(Poe1Quantity::EsRechargeAppliesToLife),
        ],
    ),
    (
        "keystone_glancing_blows",
        &[
            (
                Quantity::BlockChance,
                Op::More,
                100.0,
                Flags::NONE,
                Flags::NONE,
                None,
            ),
            (
                Quantity::Poe1(Poe1Quantity::BlockEffect),
                Op::Base,
                GLANCING_BLOWS_BLOCK_EFFECT,
                Flags::NONE,
                Flags::NONE,
                None,
            ),
        ],
    ),
    // Lightning Purist (P6-11ei; the game's name for Roiling Tempest's
    // stat): the tree's "25% more Maximum Lightning Damage"
    // (`MaxLightningDamage` MORE 25), "50% less Minimum Lightning Damage"
    // (`MinLightningDamage` MORE -50), and "Cannot deal non-Lightning
    // Damage" (`DealNoPhysical`, `DealNoCold`, `DealNoFire`, `DealNoChaos`;
    // `ModCache.lua` 5945, 8049).
    // Iron Reflexes (P6-11ek): the tree's "Converts all Evasion Rating to
    // Armour. Dexterity provides no bonus to Evasion Rating" (`IronReflexes`
    // and `NoDexBonusToEvasion` FLAG; `ModParser.lua` 2230).
    (
        "keystone_iron_reflexes",
        &[
            flag(Poe1Quantity::IronReflexes),
            flag(Poe1Quantity::NoDexBonusToEvasion),
        ],
    ),
    (
        "keystone_lightning_purist",
        &[
            (
                Quantity::Poe1(Poe1Quantity::BoundDamage(
                    Some(DamageType::Lightning),
                    Bound::Max,
                )),
                Op::More,
                25.0,
                Flags::NONE,
                Flags::NONE,
                None,
            ),
            (
                Quantity::Poe1(Poe1Quantity::BoundDamage(
                    Some(DamageType::Lightning),
                    Bound::Min,
                )),
                Op::More,
                -50.0,
                Flags::NONE,
                Flags::NONE,
                None,
            ),
            flag(Poe1Quantity::DealNo(DamageType::Physical)),
            flag(Poe1Quantity::DealNo(DamageType::Cold)),
            flag(Poe1Quantity::DealNo(DamageType::Fire)),
            flag(Poe1Quantity::DealNo(DamageType::Chaos)),
        ],
    ),
    // Mind over Matter (P6-10b part 15): the tree's text "40% of Damage is
    // taken from Mana before Life" (`DamageTakenFromManaBeforeLife` BASE).
    (
        "keystone_mana_shield",
        &[(
            Quantity::Poe1(Poe1Quantity::ManaBeforeLife(TakenOf::All)),
            Op::Base,
            MIND_OVER_MATTER,
            Flags::NONE,
            Flags::NONE,
            None,
        )],
    ),
    // Pain Attunement (P6-11ei): the tree's "30% more Spell Damage when on
    // Low Life" (`Damage` MORE 30 with `ModFlag.Spell`, `Condition`
    // `LowLife`, the config's check, `conditions::LOW_LIFE`).
    (
        "keystone_pain_attunement",
        &[(
            Quantity::Damage,
            Op::More,
            30.0,
            Flags::SPELL,
            Flags::NONE,
            Some(super::conditions::LOW_LIFE),
        )],
    ),
    // Runebinder (P6-11hv): the tree's "-1 to maximum number of Summoned
    // Totems" (`ActiveTotemLimit` BASE -1) and "You can have an additional
    // Brand Attached to an Enemy" (`BrandsAttachedLimit` BASE 1,
    // `ModParser.lua` 2334).
    (
        "keystone_runebinder",
        &[
            totems(-1.0),
            (
                Quantity::Poe1(Poe1Quantity::Stack(
                    StackKind::BrandsAttached,
                    StackValue::AttachMaximum,
                )),
                Op::Base,
                1.0,
                Flags::NONE,
                Flags::NONE,
                None,
            ),
        ],
    ),
    // Supreme Ostentation (P6-11hx; Elegant Hubris' keystone for Caspiro,
    // the game's `keystone_supreme_prodigy`): "Ignore Attribute Requirements"
    // (`IgnoreAttributeRequirements` FLAG, read only by the requirements,
    // no slice field) and "Gain no inherent bonuses from Attributes"
    // (`NoAttributeBonuses` FLAG, `ModCache.lua` 9192): the flag.
    (
        "keystone_supreme_prodigy",
        &[flag(Poe1Quantity::NoAttributeBonuses)],
    ),
    // Wicked Ward (open items S7, part 9): the game's node has this stat
    // alone; PoB reads the tree's text (3_16 to 3_29 alike): "Energy
    // Shield Recharge is not interrupted by Damage if Recharge began
    // Recently" parses to nothing (`ModCache.lua` 8636), and "40% less
    // Energy Shield Recharge Rate" is `EnergyShieldRecharge` MORE -40
    // (5482). PoB's `Condition:HaveWickedWard` is read by no module.
    (
        "keystone_wicked_ward",
        &[(
            Quantity::Poe1(Poe1Quantity::EnergyShieldRecharge),
            Op::More,
            -40.0,
            Flags::NONE,
            Flags::NONE,
            None,
        )],
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
