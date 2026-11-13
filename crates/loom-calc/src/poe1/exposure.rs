//! The enemy's exposure (Phase 3 plan, step 8; P3-8k): PoB's "Is the enemy
//! Exposed to Fire / Cold / Lightning?" checks, the stats that let the
//! character apply exposure, and the Elementalist's "Exposure you inflict
//! applies an extra -25% to the affected Resistance" (Mastermind of
//! Discord) and kin.
//!
//! What PoB v2.67.2 does (line numbers of the pinned checkout):
//! - `conditionEnemyFireExposure`, `conditionEnemyColdExposure`,
//!   `conditionEnemyLightningExposure` (`Modules/ConfigOptions.lua` 1947 to
//!   1955) give the enemy `<Type>Exposure` BASE -10, conditioned on the
//!   player's `CanApply<Type>Exposure` (and `Effective`, always on in
//!   PoB's main view).
//! - `calcs.perform` (`Modules/CalcPerform.lua` 375 to 385) sets
//!   `CanApply<Type>Exposure` when the main skill's `<Type>ExposureChance`
//!   (its `skillModList` with its `skillCfg`) or the character's (no
//!   `cfg`) BASE sum is above 0. The chance's value is never read
//!   otherwise. Modifiers give it ("#% chance to inflict Cold Exposure on
//!   Hit with Cold Damage", 4194; "Cold Skills have a #% chance to apply
//!   Cold Exposure on Hit", 4191 to 4193; "Inflict Fire, Cold, and
//!   Lightning Exposure on Hit", 4200), and so does the
//!   `elementalArmyExposureType` input (413 to 420: 100 for the chosen
//!   type). PoB applies a config input whatever its `ifSkill`
//!   (`Classes/ConfigTab.lua`, `BuildModList`, 1083 to 1101).
//! - Then (3563 to 3589), per type, unless Elemental Equilibrium is on and
//!   the enemy was hit by that type: the strongest exposure on the enemy
//!   (the lowest `<Type>Exposure`), plus each `ExtraExposure` and
//!   `Extra<Type>Exposure` BASE of the character (no `cfg`), becomes the
//!   enemy's `<Type>Resist` BASE, at most `ExposureMin`'s OVERRIDE. With no
//!   override, `modDB:Override` returns no value and `m_min` takes the
//!   exposure alone (a probe on the pinned runtime: -10, and -35 with
//!   -25 extra, against the enemy's 50% cold resistance).
//! - A hit reads the enemy's resistance as the sum of its `<Type>Resist`
//!   BASE (`calcResistForType`), capped at the enemy's maximum.
//!
//! **The engine.** [`STATS`] maps the chances to
//! [`Poe1Quantity::ExposureChance`] and the extras to
//! [`Poe1Quantity::ExtraExposure`] (`Base`, unconditional: a test).
//! [`exposure`] runs once every source is in: for each check that is on,
//! when the chance of its type sums above 0, the enemy gets
//! [`Poe1Quantity::EnemyResistance`] `Base` -10 (the check's source) and
//! one more per extra of every type or of that type (the extra's source);
//! a hit adds their sum to the enemy's base resistance ([`super::hit`]).
//! The check alone, without a chance, gives nothing, as in PoB.
//!
//! Only the config's exposure is on the enemy: every other exposure (a
//! skill's, a stronger one, Elemental Equilibrium's), the minimum
//! override, and the stats that would make the -10 not the strongest are
//! [`DEPENDS_ON`]: they stay reasons, so with one of them the hit fields
//! stay `Partial` and the sum is exact. So are the fire and lightning
//! "with Fire (Lightning) Damage" chances: PoB reads them through its
//! generic parser with a `FireHasDamage` condition that its calc modules
//! set after this check (`ModParser.lua` 1081 to 1083), not as the cold
//! one's own pattern. A gem's chance (PoB's `skillModList`) is a reason
//! through the gem mapping ([`super::mapping::SkillMapping`]).
//!
//! **The chances are read unconditionally, as PoB reads them** (the
//! review asked whether this is looser than the game). In the game "Cold
//! Skills have a #% chance" applies only through cold skills, and "...
//! with Cold Damage" only through hits that deal cold damage. But the
//! check is the user's statement that the enemy *is* exposed, and the
//! chance is PoB's gate on whether the character can expose it at all.
//! Exposure is a debuff on the enemy, so once any of the character's
//! skills applies it, every hit meets it, the main skill's too, whatever
//! its type. So no value the game gives differs; no divergence entry.
//!
//! Provenance: the -10 carries the check's source and each extra its own;
//! the chance that let the check apply is no graph node (a documented
//! gap, as the shock's minimum sources).

use loom_domain::{ConfigInputKind, ConfigSet, ConfigValue};

use super::Poe1Quantity;
use super::config::last_input as last;
use crate::ingest::Ingest;
use crate::quantity::{DamageType, Op, Quantity};
use crate::source::Source;
use crate::state::UnsupportedReason;
use crate::store::{Conditions, Effect};

/// The exposure checks, by type.
pub const CHECKS: [(DamageType, &str); 3] = [
    (DamageType::Fire, "conditionEnemyFireExposure"),
    (DamageType::Cold, "conditionEnemyColdExposure"),
    (DamageType::Lightning, "conditionEnemyLightningExposure"),
];

/// Elemental Army's exposure type list (`Fire`, `Cold`, `Lightning`, or
/// `0` for none).
pub const ARMY_INPUT: &str = "elementalArmyExposureType";

/// The exposure a check gives the enemy (`<Type>Exposure` BASE).
pub const EXPOSURE: f64 = -10.0;

/// The chance Elemental Army's input gives (`<Type>ExposureChance` BASE).
pub const ARMY_CHANCE: f64 = 100.0;

/// Whether this module handles `input`.
#[must_use]
pub fn handles(input: &str) -> bool {
    input == ARMY_INPUT || CHECKS.iter().any(|(_, name)| *name == input)
}

const fn chance(t: DamageType) -> Poe1Quantity {
    Poe1Quantity::ExposureChance(t)
}

const fn extra(t: Option<DamageType>) -> Poe1Quantity {
    Poe1Quantity::ExtraExposure(t)
}

/// One stat: its quantities (each `Base`, scale 1).
type ExposureRow = (&'static str, &'static [Poe1Quantity]);

/// The stats (module docs), sorted by stat ID. Each description is the
/// game's (`stat_descriptions.txt`), read by PoB's pattern:
/// - "#% Chance to Inflict Cold Exposure on Hit with Cold Damage" (4194);
/// - "Cold (Fire, Lightning) Skills have a #% chance to apply Cold (Fire,
///   Lightning) Exposure on Hit" (4191 to 4193);
/// - "Inflict Fire, Cold, and Lightning Exposure on Hit" (4200);
/// - "Inflict Cold (Fire, Lightning) Exposure on Hit, applying #% to Cold
///   (Fire, Lightning) Resistance" (4251, P3-8n): `<Type>ExposureChance`
///   100 and the enemy's `<Type>Exposure` at the line's value, here
///   [`Poe1Quantity::ExposureOnHit`], whose being there is also the
///   chance;
/// - "Exposure you inflict applies an extra #% to the affected Resistance"
///   (2665), "Cold (Fire) Exposure you inflict applies an extra #% to Cold
///   (Fire) Resistance" (4267, 4268). The game writes the value with its
///   sign; PoB's patterns (`(%-?%d+)`) match a negative value or a bare
///   number, not one written "+5", which PoB would not parse while the
///   engine applies it. Every such stat rolls negative, so no build meets
///   the difference.
pub const STATS: [ExposureRow; 11] = [
    (
        "chance_to_inflict_cold_exposure_on_hit_with_cold_damage_%",
        &[chance(DamageType::Cold)],
    ),
    (
        "cold_exposure_on_hit_magnitude",
        &[Poe1Quantity::ExposureOnHit(DamageType::Cold)],
    ),
    (
        "cold_exposure_you_inflict_applies_extra_cold_resistance_+%",
        &[extra(Some(DamageType::Cold))],
    ),
    (
        "cold_skill_chance_to_inflict_cold_exposure_%",
        &[chance(DamageType::Cold)],
    ),
    (
        "exposure_you_inflict_applies_extra_%_to_affected_resistance",
        &[extra(None)],
    ),
    (
        "fire_exposure_on_hit_magnitude",
        &[Poe1Quantity::ExposureOnHit(DamageType::Fire)],
    ),
    (
        "fire_exposure_you_inflict_applies_extra_fire_resistance_+%",
        &[extra(Some(DamageType::Fire))],
    ),
    (
        "fire_skill_chance_to_inflict_fire_exposure_%",
        &[chance(DamageType::Fire)],
    ),
    (
        "inflict_all_exposure_on_hit",
        &[
            chance(DamageType::Fire),
            chance(DamageType::Cold),
            chance(DamageType::Lightning),
        ],
    ),
    (
        "lightning_exposure_on_hit_magnitude",
        &[Poe1Quantity::ExposureOnHit(DamageType::Lightning)],
    ),
    (
        "lightning_skill_chance_to_inflict_lightning_exposure_%",
        &[chance(DamageType::Lightning)],
    ),
];

/// Stats PoB reads that would change the enemy's exposure otherwise than
/// [`exposure`] does, sorted: they must stay reasons (a test checks each
/// is `Unmapped` through every mapping; a maintainer-run test that each
/// is a stat of the real bundle). From P3-12f's list (then guarding the
/// checks as named conditions), less the rows of [`STATS`]:
/// - other chances PoB reads as `<Type>ExposureChance`: a gem's or a
///   minion's ("Supported Skills have #% chance to inflict Cold Exposure
///   on Hit", Summon Arbalists), the socketed, devotion, curse-cast, and
///   graft variants, and the fire and lightning "with Fire (Lightning)
///   Damage" chances (module docs);
/// - exposures of their own, so possibly stronger than the check's -10:
///   "Inflict Cold Exposure on Hit, applying #% to Cold Resistance" and
///   kin, "Nearby Enemies have Fire Exposure" and kin, Hydrosphere's,
///   Wave of Conviction's extra, the linked targets', on suppressing, a
///   flask's, at maximum rage;
/// - the minimum override ("Exposure you inflict applies at least #% to
///   the affected Resistance"), Elemental Equilibrium and its effect
///   (PoB skips a type's exposure when the enemy was hit by it), and the
///   map's "Players cannot inflict Exposure".
pub const DEPENDS_ON: &[&str] = &[
    "base_inflict_cold_exposure_on_hit_%_chance",
    "base_inflict_fire_exposure_on_hit_%_chance",
    "base_inflict_lightning_exposure_on_hit_%_chance",
    "chance_to_inflict_fire_exposure_on_hit_with_fire_damage_%",
    "chance_to_inflict_lightning_exposure_on_hit_with_lightning_damage_%",
    "elemental_equilibrium_effect_+%",
    "enemies_near_link_skill_target_have_exposure",
    "exposure_you_inflict_has_minimum_resistance_%",
    "fire_exposure_on_hit_magnitude_vs_max_stacks_resentment_debuff",
    "graft_skill_inflict_fire_exposure_on_hit_%_chance",
    "graft_skill_inflict_lightning_exposure_on_hit_%_chance",
    "inflict_all_exposure_on_enemies_when_suppressing_their_spell",
    "inflict_cold_exposure_if_cast_frostbite_in_past_10_seconds",
    "inflict_cold_exposure_on_hit_%_chance_at_devotion_threshold",
    "inflict_fire_exposure_if_cast_flammability_in_past_10_seconds",
    "inflict_fire_exposure_nearby_on_reaching_maximum_rage",
    "inflict_fire_exposure_on_hit_%_chance_at_devotion_threshold",
    "inflict_lightning_exposure_if_cast_conductivity_in_past_10_seconds",
    "inflict_lightning_exposure_on_hit_%_chance_at_devotion_threshold",
    "keystone_elemental_equilibrium",
    "local_display_nearby_enemies_have_fire_exposure",
    "local_display_socketed_gems_exposure_on_hit",
    "local_display_socketed_gems_have_elemental_equilibrium_effect_pluspercent",
    "local_flask_inflict_fire_cold_lightning_exposure_on_nearby_enemies_on_use",
    "map_player_cannot_expose",
    "minions_inflict_exposure_on_hit_%_chance",
    "nearby_enemies_all_exposure_%_while_phasing",
    "nearby_enemies_have_cold_exposure_while_you_are_affected_by_herald_of_ice",
    "nearby_enemies_have_fire_exposure_while_you_are_affected_by_herald_of_ash",
    "nearby_enemies_have_fire_exposure_while_you_are_at_maximum_rage",
    "nearby_enemies_have_lightning_exposure_while_you_are_affected_by_herald_of_thunder",
    "purge_additional_enemy_resistance_%",
    "summon_arbalist_chance_to_inflict_cold_exposure_on_hit_%",
    "summon_arbalist_chance_to_inflict_fire_exposure_on_hit_%",
    "summon_arbalist_chance_to_inflict_lightning_exposure_on_hit_%",
    "water_sphere_cold_lightning_exposure_%",
];

/// The effects of a stat of [`STATS`]: `(quantity, op, scale)`.
#[must_use]
pub fn effects(stat: &str) -> Option<Vec<(Quantity, Op, f64)>> {
    let i = STATS.binary_search_by(|(id, _)| (*id).cmp(stat)).ok()?;
    Some(
        STATS[i]
            .1
            .iter()
            .map(|&q| (Quantity::Poe1(q), Op::Base, 1.0))
            .collect(),
    )
}

/// The unconditional `Base` effects on `quantity` ingested so far. Every
/// row is unconditional (a test), so a condition can only come from a
/// later row, which must bring its handling here.
fn bases<'a>(ingest: &'a Ingest<'_>, quantity: Poe1Quantity) -> impl Iterator<Item = &'a Effect> {
    ingest.effects().iter().filter(move |e| {
        e.quantity == Quantity::Poe1(quantity)
            && e.op == Op::Base
            && e.conditions == Conditions::none()
    })
}

/// Elemental Army's input: `Ok(None)` for none (`0`) or absent, `Err(())`
/// for a value not in the list (PoB ignores it; a document can carry it,
/// so it is a reason, as the delirium list's).
fn army(set: &ConfigSet) -> Result<Option<DamageType>, ()> {
    match last(set, ARMY_INPUT, ConfigInputKind::Input) {
        None => Ok(None),
        Some(ConfigValue::Number(n)) if n.value().0 == 0.0 => Ok(None),
        // The list's values (`ConfigOptions.lua` 413), as PoB compares
        // them.
        Some(ConfigValue::String(s)) => match s.as_str() {
            "Fire" => Ok(Some(DamageType::Fire)),
            "Cold" => Ok(Some(DamageType::Cold)),
            "Lightning" => Ok(Some(DamageType::Lightning)),
            _ => Err(()),
        },
        Some(_) => Err(()),
    }
}

/// An effect from a config input.
fn from_config(quantity: Poe1Quantity, value: f64, input: &str) -> Effect {
    Effect {
        quantity: Quantity::Poe1(quantity),
        op: Op::Base,
        value,
        conditions: Conditions::none(),
        source: Source::Config {
            input: input.to_owned(),
        },
    }
}

/// Adds Elemental Army's chance and the enemy's exposure from the active
/// config set (module docs). Runs after every other source; a value the
/// engine does not read as PoB does is a reason under its input.
pub fn exposure(set: Option<&ConfigSet>, ingest: &mut Ingest<'_>) {
    if let Some(set) = set {
        match army(set) {
            Ok(None) => {}
            Ok(Some(t)) => ingest.effect(from_config(chance(t), ARMY_CHANCE, ARMY_INPUT)),
            Err(()) => ingest.reason(UnsupportedReason::UnmodelledConfig {
                input: ARMY_INPUT.to_owned(),
            }),
        }
    }
    for (t, input) in CHECKS {
        let on_hit: Vec<Effect> = bases(ingest, Poe1Quantity::ExposureOnHit(t))
            .cloned()
            .collect();
        // The check's -10, when on and the type can be applied.
        let mut candidates = Vec::new();
        match set.and_then(|set| last(set, input, ConfigInputKind::Input)) {
            None | Some(ConfigValue::Boolean(false)) => {}
            Some(ConfigValue::Boolean(true)) => {
                // An exposure on hit is also a chance of 100 (P3-8n).
                let can_apply = bases(ingest, chance(t)).map(|e| e.value).sum::<f64>() > 0.0
                    || !on_hit.is_empty();
                if can_apply {
                    candidates.push(from_config(
                        Poe1Quantity::EnemyResistance(t),
                        EXPOSURE,
                        input,
                    ));
                }
            }
            // PoB reads any other value as on: not modelled, a reason.
            Some(ConfigValue::Number(_) | ConfigValue::String(_)) => {
                ingest.reason(UnsupportedReason::UnmodelledConfig {
                    input: input.to_owned(),
                });
            }
        }
        // Each exposure on hit is on the enemy (P3-8n): its line's value.
        candidates.extend(on_hit.iter().map(|e| Effect {
            quantity: Quantity::Poe1(Poe1Quantity::EnemyResistance(t)),
            op: Op::Base,
            value: e.value,
            conditions: Conditions::none(),
            source: e.source.clone(),
        }));
        // The strongest: PoB keeps the first strictly lowest (3572 to
        // 3577).
        let Some(strongest) = candidates
            .into_iter()
            .reduce(|best, e| if e.value < best.value { e } else { best })
        else {
            continue;
        };
        let mut added = vec![strongest];
        added.extend(
            bases(ingest, extra(None))
                .chain(bases(ingest, extra(Some(t))))
                .map(|e| Effect {
                    quantity: Quantity::Poe1(Poe1Quantity::EnemyResistance(t)),
                    op: Op::Base,
                    value: e.value,
                    conditions: Conditions::none(),
                    source: e.source.clone(),
                }),
        );
        for e in added {
            ingest.effect(e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{StatMap, StatMapping};
    use crate::poe1::ArmourSlot;
    use crate::poe1::mapping::{PieceMapping, Poe1Mapping, SkillMapping, WeaponMapping};
    use loom_domain::StatId;

    #[test]
    fn stats_should_be_sorted_and_unique() {
        assert!(STATS.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(DEPENDS_ON.windows(2).all(|w| w[0] < w[1]));
        for (stat, _) in STATS {
            assert!(!DEPENDS_ON.contains(&stat), "{stat}");
        }
    }

    #[test]
    fn the_character_mapping_should_read_every_row_unconditionally() {
        for (stat, row) in STATS {
            let StatMap::Effects(effects) = Poe1Mapping.map(&StatId::new(stat)) else {
                panic!("{stat} is not mapped");
            };
            assert_eq!(effects.len(), row.len(), "{stat}");
            for (t, &q) in effects.iter().zip(row) {
                assert_eq!(t.quantity, Quantity::Poe1(q), "{stat}");
                assert_eq!(t.op, Op::Base, "{stat}");
                assert!((t.scale - 1.0).abs() < f64::EPSILON, "{stat}");
                assert_eq!(t.conditions, Conditions::none(), "{stat}");
            }
        }
    }

    #[test]
    fn a_gems_exposure_stats_should_be_reasons() {
        for (stat, _) in STATS {
            assert_eq!(
                SkillMapping.map(&StatId::new(stat)),
                StatMap::Unmapped,
                "{stat}"
            );
        }
    }

    #[test]
    fn the_stats_left_should_stay_unmapped() {
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

    #[test]
    fn the_inputs_should_be_handled_here_only() {
        assert!(handles(ARMY_INPUT));
        for (_, input) in CHECKS {
            assert!(handles(input));
            assert!(!super::super::config::named_only(input), "{input}");
            assert!(!super::super::config::enemy_damage(input), "{input}");
            assert!(!super::super::damage_taken::handles(
                input,
                &ConfigValue::Boolean(true)
            ));
        }
        assert!(!super::super::config::named_only(ARMY_INPUT));
    }
}
