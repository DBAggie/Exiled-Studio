//! Minion modifiers that also affect you (Phase 3 plan, step 8; P3-8j):
//! "Increases and Reductions to Minion Damage also affect you" (Spiritual
//! Aid), "Increases and Reductions to Minion Attack Speed also affect you"
//! (Spiritual Command), and their kin.
//!
//! What PoB v2.67.2 does:
//! - `Modules/ModParser.lua` reads "Minions deal #% increased Damage",
//!   "Minions have #% increased Attack Speed", "... Cast Speed", and "...
//!   Attack and Cast Speed" as `MinionModifier` LIST modifiers holding
//!   `Damage` INC, `Speed` INC with `ModFlag.Attack`, with `ModFlag.Cast`,
//!   and with no flag (its "minion" word, 1068, and the speed words);
//! - it reads the redirects as flags (3455 to 3462):
//!   `MinionDamageAppliesToPlayer` (with "at #% of their value", also
//!   `ImprovedMinionDamageAppliesToPlayer` MAX #),
//!   `MinionAttackSpeedAppliesToPlayer`, `MinionCastSpeedAppliesToPlayer`;
//! - `Modules/CalcOffence.lua` 613 to 643, for the skill being computed:
//!   with a flag set, every `MinionModifier` of the kind becomes the
//!   player's own modifier at the multiplier (`Improved...` MAX, else
//!   100%), keeping its source and its tags: a `Damage` INC as `Damage`
//!   INC with its flags; a `Speed` INC with no flag or the Attack (Cast)
//!   flag as `Speed` INC with `ModFlag.Attack` (`ModFlag.Cast`). It does
//!   not floor them (only the life redirect, `CalcPerform.lua` 1566, which
//!   reaches no stat here, floors).
//!
//! **The engine.** [`STATS`] maps the minion stats to
//! [`Poe1Quantity::Minion`] (`Increased`) and the redirects to
//! [`Poe1Quantity::MinionToYou`] (`Flag`, with the percent of their value:
//! a redirect line has no value, so its stat value is not read).
//! [`minions`] runs once every source is in: for each kind with a
//! redirect, the largest percent counts (PoB's flag with its MAX), and
//! each minion effect of that kind becomes the character's (`Damage` or
//! `Speed` `Increased`, the speed for attacks or casts) at that percent,
//! with the minion effect's source and conditions. A minion effect without
//! a redirect changes no slice field: no minion is computed.
//!
//! A gem's own minion stats (Minion Damage, Minion Speed, Predator,
//! Infernal Legion) are minion effects too: only the main skill's gems are
//! read through the gem mapping ([`super::mapping::SkillMapping`]), so
//! they are the `MinionModifier` entries of PoB's `skillModList`, which
//! `CalcOffence.lua` converts with the rest (Absolution or Dominating Blow
//! supported by Minion Damage, with Spiritual Aid). Other groups' gems do
//! not reach this mapping, as their minion stats do not reach the skill in
//! PoB.
//!
//! The redirects PoB reads that the engine does not are [`DEPENDS_ON`]:
//! they stay reasons. Other minion stats PoB reads as `Damage` or `Speed`
//! INC (minion melee damage, damage on full or low life, ...) are
//! unmapped: reasons. Every redirect row is unconditional (a test), so
//! [`minions`] reads no condition on a redirect.
//!
//! **The minion's own life and defences** (P6-7b3b): "Minions have #%
//! increased maximum Life" and kin are `MinionModifier` entries PoB adds
//! to the minion (`CalcPerform.lua` `addMinionModifiers`, 1213 to 1219,
//! called at 1918 with the main skill's `skillModList`). Their rows here
//! carry PoB's mod type as the op, each checked against PoB's parse of the
//! game's text, or, for a gem's stat, PoB's stat map
//! (`Data/SkillStatMap.lua` 1922 to 2060, `Data/Skills/sup_int.lua` 4675
//! and 4929). [`MinionValue::on_minion`] names the minion's quantity, and
//! the minion step ([`super::minion_actor`]) adds them to the minion's
//! store. The golems' lines (`ModParser.lua` 1137: `SkillType.Golem` on
//! the main skill) are [`Poe1Quantity::GolemMinion`].

use super::Poe1Quantity;
use crate::condition::Flags;
use crate::ingest::Ingest;
use crate::quantity::{Bound, DamageType, Op, Quantity};
use crate::store::Effect;

/// A kind of minion modifier: one the character can take on (damage and
/// speeds, P3-8j), or one of the minion's own life and defences (P6-7b3b).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MinionValue {
    /// Increased damage.
    Damage,
    /// Increased attack speed.
    AttackSpeed,
    /// Increased cast speed.
    CastSpeed,
    /// The minion's maximum life (PoB's `Life`).
    Life,
    /// The minion's maximum energy shield (PoB's `EnergyShield`).
    EnergyShield,
    /// The minion's armour (PoB's `Armour`).
    Armour,
    /// The minion's evasion rating (PoB's `Evasion`).
    Evasion,
    /// The minion's resistance of a type (PoB's `<Type>Resist`).
    Resistance(DamageType),
    /// The minion's maximum resistance of a type (PoB's `<Type>ResistMax`).
    MaximumResistance(DamageType),
    /// The minion's chance to block attack damage (PoB's `BlockChance`).
    BlockChance,
    /// The minion's chance to block spell damage (PoB's
    /// `SpellBlockChance`).
    SpellBlockChance,
    /// The minion's life regeneration, percent of life per second (PoB's
    /// `LifeRegenPercent`): no minion field reads it.
    LifeRegen,
    /// The minion's movement speed (PoB's `MovementSpeed`): no minion field
    /// reads it, as no character field reads the character's.
    MovementSpeed,
    /// The minion's cooldown recovery (PoB's `CooldownRecovery`): no
    /// minion field reads it.
    CooldownRecovery,
    /// The minion's added damage of a type, one bound (PoB's `<Type>Min`
    /// and `<Type>Max` BASE; P6-11as).
    AddedDamage(DamageType, Bound),
    /// The minion's chance to deal double damage (PoB's
    /// `DoubleDamageChance`; P6-11as).
    DoubleDamageChance,
}

impl MinionValue {
    /// The kinds a redirect can give the character.
    pub const REDIRECTED: [Self; 3] = [Self::Damage, Self::AttackSpeed, Self::CastSpeed];

    /// The kind's name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Damage => "damage",
            Self::AttackSpeed => "attack-speed",
            Self::CastSpeed => "cast-speed",
            Self::Life => "life",
            Self::EnergyShield => "energy-shield",
            Self::Armour => "armour",
            Self::Evasion => "evasion",
            Self::Resistance(t) => match t {
                DamageType::Physical => "physical-resistance",
                DamageType::Fire => "fire-resistance",
                DamageType::Cold => "cold-resistance",
                DamageType::Lightning => "lightning-resistance",
                DamageType::Chaos => "chaos-resistance",
            },
            Self::MaximumResistance(t) => match t {
                DamageType::Physical => "maximum-physical-resistance",
                DamageType::Fire => "maximum-fire-resistance",
                DamageType::Cold => "maximum-cold-resistance",
                DamageType::Lightning => "maximum-lightning-resistance",
                DamageType::Chaos => "maximum-chaos-resistance",
            },
            Self::BlockChance => "block-chance",
            Self::SpellBlockChance => "spell-block-chance",
            Self::LifeRegen => "life-regen",
            Self::MovementSpeed => "movement-speed",
            Self::CooldownRecovery => "cooldown-recovery",
            Self::AddedDamage(t, b) => match (t, b) {
                (DamageType::Physical, Bound::Min) => "added-physical-min",
                (DamageType::Physical, Bound::Max) => "added-physical-max",
                (DamageType::Fire, Bound::Min) => "added-fire-min",
                (DamageType::Fire, Bound::Max) => "added-fire-max",
                (DamageType::Cold, Bound::Min) => "added-cold-min",
                (DamageType::Cold, Bound::Max) => "added-cold-max",
                (DamageType::Lightning, Bound::Min) => "added-lightning-min",
                (DamageType::Lightning, Bound::Max) => "added-lightning-max",
                (DamageType::Chaos, Bound::Min) => "added-chaos-min",
                (DamageType::Chaos, Bound::Max) => "added-chaos-max",
            },
            Self::DoubleDamageChance => "double-damage-chance",
        }
    }

    /// The character's quantity and the flags a redirected kind's effects
    /// take on; `None` for a kind no redirect gives.
    const fn target(self) -> Option<(Quantity, Flags)> {
        match self {
            Self::Damage => Some((Quantity::Damage, Flags::NONE)),
            Self::AttackSpeed => Some((Quantity::Speed, Flags::ATTACK)),
            Self::CastSpeed => Some((Quantity::Speed, Flags::CAST)),
            _ => None,
        }
    }

    /// The minion's quantity the kind adds to (P6-7b3b), and the flags its
    /// effects take on (P6-7c: damage, attack and cast speed, PoB's
    /// `Damage` and `Speed` with `ModFlag.Attack` or `ModFlag.Cast`); `None`
    /// for a kind no minion field reads (regeneration, movement speed, and
    /// cooldown recovery).
    #[must_use]
    pub const fn on_minion(self) -> Option<(Quantity, Flags)> {
        let q = match self {
            Self::Life => Quantity::Life,
            Self::EnergyShield => Quantity::EnergyShield,
            Self::Armour => Quantity::Armour,
            Self::Evasion => Quantity::Evasion,
            Self::Resistance(t) => Quantity::Resistance(t),
            Self::MaximumResistance(t) => Quantity::MaximumResistance(t),
            Self::BlockChance => Quantity::BlockChance,
            Self::SpellBlockChance => Quantity::Poe1(Poe1Quantity::SpellBlockChance),
            Self::AddedDamage(t, b) => Quantity::AddedDamage(t, b),
            Self::DoubleDamageChance => Quantity::Poe1(Poe1Quantity::DoubleDamageChance),
            Self::Damage | Self::AttackSpeed | Self::CastSpeed => return self.target(),
            Self::LifeRegen | Self::MovementSpeed | Self::CooldownRecovery => return None,
        };
        Some((q, Flags::NONE))
    }
}

const fn minion(value: MinionValue) -> Poe1Quantity {
    Poe1Quantity::Minion(value)
}

const fn to_you(value: MinionValue, percent: u16) -> Poe1Quantity {
    Poe1Quantity::MinionToYou(value, percent)
}

const fn golem(value: MinionValue) -> Poe1Quantity {
    Poe1Quantity::GolemMinion(value)
}

const INC: Op = Op::Increased;
const MORE: Op = Op::More;
const BASE: Op = Op::Base;
const FLAG: Op = Op::Flag;
const FIRE: DamageType = DamageType::Fire;
const COLD: DamageType = DamageType::Cold;
const LIGHTNING: DamageType = DamageType::Lightning;
const CHAOS: DamageType = DamageType::Chaos;

/// One stat: its quantities, PoB's mod type, and the scale per unit of
/// its value.
type MinionRow = (&'static str, &'static [(Poe1Quantity, Op, f64)]);

/// The minion stats and the redirects, sorted by stat ID (module docs).
/// Each description is the game's (`stat_descriptions.txt`):
/// - "Minions deal #% increased Damage", "Minions have #% increased Attack
///   Speed", "... Cast Speed", "... Attack and Cast Speed";
/// - "Increases and Reductions to Minion Damage also affect you" (its
///   value 1, so 100%), "... at 150% of their value", "Increases and
///   Reductions to Minion Attack Speed also affect you", "... Cast Speed
///   ...";
/// - P6-7b3b: "Minions have #% increased maximum Life", "... more Maximum
///   Life", "... +#% to all Elemental Resistances", "... +#% Chance to
///   Block Attack Damage", "Golems have #% increased Maximum Life", and
///   kin, each PoB's `MinionModifier` with the mod type as the op (module
///   docs). "Minions Regenerate #% of Life per second" is PoB's
///   `LifeRegenPercent` per minute over 60 (`SkillStatMap.lua` 1967).
pub const STATS: [MinionRow; 43] = [
    (
        "active_skill_minion_energy_shield_+%_final",
        &[(minion(MinionValue::EnergyShield), MORE, 1.0)],
    ),
    (
        "active_skill_minion_life_+%_final",
        &[(minion(MinionValue::Life), MORE, 1.0)],
    ),
    (
        "active_skill_minion_movement_velocity_+%_final",
        &[(minion(MinionValue::MovementSpeed), MORE, 1.0)],
    ),
    (
        "additive_minion_damage_modifiers_apply_to_you_at_150%_value",
        &[(to_you(MinionValue::Damage, 150), FLAG, 1.0)],
    ),
    (
        "additive_modifiers_to_minion_attack_speed_also_affect_you",
        &[(to_you(MinionValue::AttackSpeed, 100), FLAG, 1.0)],
    ),
    (
        "additive_modifiers_to_minion_cast_speed_also_affect_you",
        &[(to_you(MinionValue::CastSpeed, 100), FLAG, 1.0)],
    ),
    (
        "golem_maximum_energy_shield_+%",
        &[(golem(MinionValue::EnergyShield), INC, 1.0)],
    ),
    (
        "golem_maximum_life_+%",
        &[(golem(MinionValue::Life), INC, 1.0)],
    ),
    (
        "golem_physical_damage_reduction_rating",
        &[(golem(MinionValue::Armour), BASE, 1.0)],
    ),
    (
        "minion_additional_spell_block_%",
        &[(minion(MinionValue::SpellBlockChance), BASE, 1.0)],
    ),
    (
        "minion_attack_and_cast_speed_+%",
        &[
            (minion(MinionValue::AttackSpeed), INC, 1.0),
            (minion(MinionValue::CastSpeed), INC, 1.0),
        ],
    ),
    (
        "minion_attack_speed_+%",
        &[(minion(MinionValue::AttackSpeed), INC, 1.0)],
    ),
    (
        "minion_block_%",
        &[(minion(MinionValue::BlockChance), BASE, 1.0)],
    ),
    (
        "minion_cast_speed_+%",
        &[(minion(MinionValue::CastSpeed), INC, 1.0)],
    ),
    // P6-11as: "Minions have #% chance to deal Double Damage"
    // (`MinionModifier` `DoubleDamageChance` BASE).
    (
        "minion_chance_to_deal_double_damage_%",
        &[(minion(MinionValue::DoubleDamageChance), BASE, 1.0)],
    ),
    (
        "minion_chaos_resistance_%",
        &[(minion(MinionValue::Resistance(CHAOS)), BASE, 1.0)],
    ),
    (
        "minion_cold_damage_resistance_%",
        &[(minion(MinionValue::Resistance(COLD)), BASE, 1.0)],
    ),
    (
        "minion_cooldown_recovery_+%",
        &[(minion(MinionValue::CooldownRecovery), INC, 1.0)],
    ),
    (
        "minion_damage_+%",
        &[(minion(MinionValue::Damage), INC, 1.0)],
    ),
    (
        "minion_damage_increases_and_reductions_also_affects_you",
        &[(to_you(MinionValue::Damage, 100), FLAG, 1.0)],
    ),
    (
        "minion_elemental_resistance_%",
        &[
            (minion(MinionValue::Resistance(FIRE)), BASE, 1.0),
            (minion(MinionValue::Resistance(COLD)), BASE, 1.0),
            (minion(MinionValue::Resistance(LIGHTNING)), BASE, 1.0),
        ],
    ),
    (
        "minion_evasion_rating_+%",
        &[(minion(MinionValue::Evasion), INC, 1.0)],
    ),
    (
        "minion_fire_damage_resistance_%",
        &[(minion(MinionValue::Resistance(FIRE)), BASE, 1.0)],
    ),
    (
        "minion_global_maximum_added_chaos_damage",
        &[(
            minion(MinionValue::AddedDamage(CHAOS, Bound::Max)),
            BASE,
            1.0,
        )],
    ), // P6-11as (cold) and P6-11bh (the other types): "Minions deal # to #
    // additional <Type> Damage" (`MinionModifier` `<Type>Min` and
    // `<Type>Max` BASE, no flags).
    (
        "minion_global_maximum_added_cold_damage",
        &[(
            minion(MinionValue::AddedDamage(COLD, Bound::Max)),
            BASE,
            1.0,
        )],
    ),
    (
        "minion_global_maximum_added_fire_damage",
        &[(
            minion(MinionValue::AddedDamage(FIRE, Bound::Max)),
            BASE,
            1.0,
        )],
    ),
    (
        "minion_global_maximum_added_lightning_damage",
        &[(
            minion(MinionValue::AddedDamage(LIGHTNING, Bound::Max)),
            BASE,
            1.0,
        )],
    ),
    (
        "minion_global_maximum_added_physical_damage",
        &[(
            minion(MinionValue::AddedDamage(DamageType::Physical, Bound::Max)),
            BASE,
            1.0,
        )],
    ),
    (
        "minion_global_minimum_added_chaos_damage",
        &[(
            minion(MinionValue::AddedDamage(CHAOS, Bound::Min)),
            BASE,
            1.0,
        )],
    ),
    (
        "minion_global_minimum_added_cold_damage",
        &[(
            minion(MinionValue::AddedDamage(COLD, Bound::Min)),
            BASE,
            1.0,
        )],
    ),
    (
        "minion_global_minimum_added_fire_damage",
        &[(
            minion(MinionValue::AddedDamage(FIRE, Bound::Min)),
            BASE,
            1.0,
        )],
    ),
    (
        "minion_global_minimum_added_lightning_damage",
        &[(
            minion(MinionValue::AddedDamage(LIGHTNING, Bound::Min)),
            BASE,
            1.0,
        )],
    ),
    (
        "minion_global_minimum_added_physical_damage",
        &[(
            minion(MinionValue::AddedDamage(DamageType::Physical, Bound::Min)),
            BASE,
            1.0,
        )],
    ),
    (
        "minion_life_regeneration_rate_per_minute_%",
        &[(minion(MinionValue::LifeRegen), BASE, 1.0 / 60.0)],
    ),
    (
        "minion_maximum_all_elemental_resistances_%",
        &[
            (minion(MinionValue::MaximumResistance(FIRE)), BASE, 1.0),
            (minion(MinionValue::MaximumResistance(COLD)), BASE, 1.0),
            (minion(MinionValue::MaximumResistance(LIGHTNING)), BASE, 1.0),
        ],
    ),
    (
        "minion_maximum_energy_shield_+%",
        &[(minion(MinionValue::EnergyShield), INC, 1.0)],
    ),
    (
        "minion_maximum_life_+%",
        &[(minion(MinionValue::Life), INC, 1.0)],
    ),
    (
        "minion_movement_speed_+%",
        &[(minion(MinionValue::MovementSpeed), INC, 1.0)],
    ),
    (
        "minion_physical_damage_reduction_rating",
        &[(minion(MinionValue::Armour), BASE, 1.0)],
    ),
    (
        "necromancer_defensive_notable_minion_maximum_life_+%_final",
        &[(minion(MinionValue::Life), MORE, 1.0)],
    ),
    // P6-7c2: Minion Damage's own `statMap` (`Data/Skills/sup_int.lua`
    // 4813): `MinionModifier` holding `Damage` MORE.
    (
        "support_minion_damage_+%_final",
        &[(minion(MinionValue::Damage), MORE, 1.0)],
    ),
    (
        "support_minion_damage_minion_life_+%_final",
        &[(minion(MinionValue::Life), MORE, 1.0)],
    ),
    (
        "support_minion_maximum_life_+%_final",
        &[(minion(MinionValue::Life), MORE, 1.0)],
    ),
];

/// Redirects PoB reads that the engine does not, sorted: they must stay
/// reasons while minion modifiers are only redirected by [`STATS`]. "...
/// also affect Dominating Blow and Absolution at #% of their value"
/// (`ModParser.lua` 3457: Dominating Blow and Absolution only), the skill
/// stat that applies them to all damage (`Data/SkillStatMap.lua` 905),
/// "Modifiers to Minion Damage also affect you", which PoB does not read
/// at all (a line the engine would apply where PoB does not: a divergence
/// to settle first), and `additive_minion_damage_modifiers_apply_to_you_at_%_value`,
/// which has no description, so no line resolves to it.
pub const DEPENDS_ON: &[&str] = &[
    "active_skill_additive_minion_damage_modifiers_apply_to_all_damage_at_%_value",
    "additive_minion_damage_modifiers_apply_to_you_at_%_value",
    "dominating_blow_and_absolution_additive_minion_damage_modifiers_apply_to_you_at_150%_value",
    "modifiers_to_minion_damage_also_affect_you",
];

/// The effects of a stat of [`STATS`]: `(quantity, op, scale)`.
#[must_use]
pub fn effects(stat: &str) -> Option<Vec<(Quantity, Op, f64)>> {
    let i = STATS.binary_search_by(|(id, _)| (*id).cmp(stat)).ok()?;
    Some(
        STATS[i]
            .1
            .iter()
            .map(|&(q, op, scale)| (Quantity::Poe1(q), op, scale))
            .collect(),
    )
}

/// Whether `quantity` is a minion modifier (no field of the character
/// reads it but [`minions`]).
#[must_use]
pub const fn is_minion_quantity(quantity: Quantity) -> bool {
    matches!(
        quantity,
        Quantity::Poe1(
            Poe1Quantity::Minion(_) | Poe1Quantity::GolemMinion(_) | Poe1Quantity::MinionToYou(..)
        )
    )
}

/// Adds the minion modifiers that also affect the character (module
/// docs). Runs after every other source has been ingested.
pub fn minions(ingest: &mut Ingest<'_>) {
    for kind in MinionValue::REDIRECTED {
        // The largest percent (PoB's flag, with `Improved...` MAX).
        // Every redirect row is unconditional: a conditional one would be
        // a new row, which must come with its condition here.
        let percent = ingest
            .effects()
            .iter()
            .filter_map(|e| match (e.quantity, e.op) {
                (Quantity::Poe1(Poe1Quantity::MinionToYou(k, p)), Op::Flag) if k == kind => Some(p),
                _ => None,
            })
            .max();
        let Some(percent) = percent else { continue };
        let percent = f64::from(percent);
        let Some((quantity, flags)) = kind.target() else {
            continue;
        };
        let added: Vec<Effect> = ingest
            .effects()
            .iter()
            .filter(|e| e.quantity == Quantity::Poe1(minion(kind)) && e.op == Op::Increased)
            .map(|e| Effect {
                quantity,
                op: Op::Increased,
                value: e.value * percent / 100.0,
                conditions: e.conditions.clone().with_all(flags),
                source: e.source.clone(),
            })
            .collect();
        for e in added {
            ingest.effect(e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition::Conditions;
    use crate::ingest::{StatMap, StatMapping};
    use crate::poe1::ArmourSlot;
    use crate::poe1::mapping::{PieceMapping, Poe1Mapping, SkillMapping, WeaponMapping};
    use crate::source::Source;
    use loom_domain::StatId;

    #[test]
    fn stats_should_be_sorted_and_unique() {
        assert!(STATS.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(DEPENDS_ON.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn the_character_mapping_should_read_every_row() {
        for (stat, row) in STATS {
            let StatMap::Effects(effects) = Poe1Mapping.map(&StatId::new(stat)) else {
                panic!("{stat} is not mapped");
            };
            assert_eq!(effects.len(), row.len(), "{stat}");
            for (t, &(q, op, scale)) in effects.iter().zip(row) {
                assert_eq!(t.quantity, Quantity::Poe1(q), "{stat}");
                assert_eq!(t.op, op, "{stat}");
                assert!((t.scale - scale).abs() < f64::EPSILON, "{stat}");
                assert_eq!(t.conditions, Conditions::none(), "{stat}");
            }
        }
    }

    #[test]
    fn a_main_skill_gems_minion_stats_should_be_minion_effects() {
        for (stat, _) in STATS {
            assert_eq!(
                SkillMapping.map(&StatId::new(stat)),
                Poe1Mapping.map(&StatId::new(stat)),
                "{stat}"
            );
        }
    }

    #[test]
    fn a_gems_minion_damage_should_follow_the_redirect() {
        let gem = Source::Gem { group: 0, gem: 1 };
        let got = redirected(vec![
            effect(DAMAGE, Op::Increased, 30.0, gem.clone()),
            effect(to_you(MinionValue::Damage, 100), Op::Flag, 0.0, passive(1)),
        ]);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].quantity, Quantity::Damage);
        assert!((got[0].value - 30.0).abs() < 1e-9);
        assert_eq!(got[0].source, gem);
    }

    #[test]
    fn other_minion_damage_stats_should_stay_unmapped() {
        for stat in ["minion_melee_damage_+%", "minion_damage_+%_on_full_life"] {
            assert_eq!(
                Poe1Mapping.map(&StatId::new(stat)),
                StatMap::Unmapped,
                "{stat}"
            );
        }
    }

    fn passive(node: u32) -> Source {
        Source::Passive {
            node: loom_domain::NodeId(node),
        }
    }

    fn effect(quantity: Poe1Quantity, op: Op, value: f64, source: Source) -> Effect {
        Effect {
            quantity: Quantity::Poe1(quantity),
            op,
            value,
            conditions: Conditions::none(),
            source,
        }
    }

    /// The character's `Damage` and `Speed` effects after [`minions`].
    fn redirected(effects: Vec<Effect>) -> Vec<Effect> {
        let mut ingest = Ingest::new(&Poe1Mapping);
        for e in effects {
            ingest.effect(e);
        }
        minions(&mut ingest);
        ingest
            .effects()
            .iter()
            .filter(|e| matches!(e.quantity, Quantity::Damage | Quantity::Speed))
            .cloned()
            .collect()
    }

    const DAMAGE: Poe1Quantity = Poe1Quantity::Minion(MinionValue::Damage);
    const ATTACK: Poe1Quantity = Poe1Quantity::Minion(MinionValue::AttackSpeed);
    const CAST: Poe1Quantity = Poe1Quantity::Minion(MinionValue::CastSpeed);

    #[test]
    fn minion_modifiers_without_a_redirect_should_reach_nothing() {
        assert!(
            redirected(vec![
                effect(DAMAGE, Op::Increased, 20.0, passive(1)),
                effect(ATTACK, Op::Increased, 10.0, passive(1)),
                effect(CAST, Op::Increased, 10.0, passive(1)),
            ])
            .is_empty()
        );
    }

    #[test]
    fn the_largest_damage_redirect_should_count_once() {
        let got = redirected(vec![
            effect(DAMAGE, Op::Increased, 20.0, passive(1)),
            effect(DAMAGE, Op::Increased, -4.0, passive(2)),
            effect(to_you(MinionValue::Damage, 100), Op::Flag, 0.0, passive(3)),
            effect(to_you(MinionValue::Damage, 150), Op::Flag, 1.0, passive(4)),
            effect(to_you(MinionValue::Damage, 100), Op::Flag, 1.0, passive(5)),
        ]);
        let want = [(20.0 * 1.5, passive(1)), (-4.0 * 1.5, passive(2))];
        assert_eq!(got.len(), want.len());
        for (e, (value, source)) in got.iter().zip(want) {
            assert_eq!(e.quantity, Quantity::Damage);
            assert_eq!(e.op, Op::Increased);
            assert!((e.value - value).abs() < 1e-9, "{e:?}");
            assert_eq!(e.conditions, Conditions::none());
            assert_eq!(e.source, source);
        }
    }

    #[test]
    fn each_speed_redirect_should_give_its_own_kind_only() {
        let got = redirected(vec![
            effect(ATTACK, Op::Increased, 10.0, passive(1)),
            effect(CAST, Op::Increased, 7.0, passive(2)),
            effect(DAMAGE, Op::Increased, 20.0, passive(3)),
            effect(
                to_you(MinionValue::AttackSpeed, 100),
                Op::Flag,
                0.0,
                passive(4),
            ),
        ]);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].quantity, Quantity::Speed);
        assert!((got[0].value - 10.0).abs() < 1e-9);
        assert_eq!(
            got[0].conditions,
            Conditions::none().with_all(Flags::ATTACK)
        );
        assert_eq!(got[0].source, passive(1));

        let got = redirected(vec![
            effect(ATTACK, Op::Increased, 10.0, passive(1)),
            effect(CAST, Op::Increased, 7.0, passive(2)),
            effect(
                to_you(MinionValue::CastSpeed, 100),
                Op::Flag,
                0.0,
                passive(4),
            ),
        ]);
        assert_eq!(got.len(), 1);
        assert!((got[0].value - 7.0).abs() < 1e-9);
        assert_eq!(got[0].conditions, Conditions::none().with_all(Flags::CAST));
    }

    #[test]
    fn attack_and_cast_speed_should_reach_both_redirects() {
        let StatMap::Effects(templates) =
            Poe1Mapping.map(&StatId::new("minion_attack_and_cast_speed_+%"))
        else {
            panic!("not mapped");
        };
        let mut effects: Vec<Effect> = templates
            .iter()
            .map(|t| Effect {
                quantity: t.quantity,
                op: t.op,
                value: 6.0,
                conditions: t.conditions.clone(),
                source: passive(1),
            })
            .collect();
        effects.push(effect(
            to_you(MinionValue::AttackSpeed, 100),
            Op::Flag,
            0.0,
            passive(2),
        ));
        effects.push(effect(
            to_you(MinionValue::CastSpeed, 100),
            Op::Flag,
            0.0,
            passive(2),
        ));
        let got = redirected(effects);
        let flags: Vec<Flags> = got.iter().map(|e| e.conditions.all).collect();
        assert_eq!(flags, [Flags::ATTACK, Flags::CAST]);
        assert!(got.iter().all(|e| (e.value - 6.0).abs() < 1e-9));
    }

    #[test]
    fn a_minion_modifiers_conditions_should_be_kept() {
        let key = crate::condition::ConditionKey::new("UsingStaff");
        let mut e = effect(ATTACK, Op::Increased, 10.0, passive(1));
        e.conditions = Conditions::none().with_state(key.clone(), true);
        let got = redirected(vec![
            e,
            effect(
                to_you(MinionValue::AttackSpeed, 100),
                Op::Flag,
                0.0,
                passive(2),
            ),
        ]);
        assert_eq!(
            got[0].conditions,
            Conditions::none()
                .with_state(key, true)
                .with_all(Flags::ATTACK)
        );
    }

    /// P6-7b3b: every life and defence kind reaches a minion quantity, and
    /// the redirected kinds reach none yet (P6-7c).
    #[test]
    fn each_kind_should_name_its_minion_quantity() {
        for (stat, row) in STATS {
            for &(q, op, _) in row {
                let (Poe1Quantity::Minion(v) | Poe1Quantity::GolemMinion(v)) = q else {
                    assert_eq!(op, Op::Flag, "{stat}");
                    continue;
                };
                let no_field = matches!(
                    v,
                    MinionValue::LifeRegen
                        | MinionValue::MovementSpeed
                        | MinionValue::CooldownRecovery
                );
                assert_eq!(v.on_minion().is_none(), no_field, "{stat}");
                assert!(matches!(op, Op::Increased | Op::More | Op::Base), "{stat}");
            }
        }
    }

    #[test]
    fn the_redirects_left_should_stay_unmapped() {
        let piece = PieceMapping {
            slot: ArmourSlot::Helmet,
        };
        let mappings: [&dyn StatMapping; 4] = [&Poe1Mapping, &SkillMapping, &piece, &WeaponMapping];
        for &stat in DEPENDS_ON {
            for m in mappings {
                assert_eq!(m.map(&StatId::new(stat)), StatMap::Unmapped, "{stat}");
            }
        }
    }
}
