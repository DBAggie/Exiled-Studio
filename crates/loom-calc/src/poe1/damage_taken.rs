//! The enemy's damage taken and evasion from the config (Phase 3 plan,
//! step 12): shock, the Uber boss kind, and delirium (P3-12f);
//! Intimidate, Unnerve, and Blind (P3-12g).
//!
//! What PoB v2.67.2 does:
//! - **The hit** (`Modules/CalcOffence.lua` 3515 to 3620, in the effective
//!   mode, PoB's main view): each damage type's hit is multiplied by
//!   `effMult = (1 + takenInc / 100) x takenMore x (1 - effectiveResist /
//!   100)`, `takenInc` the enemy's `DamageTaken` and `<Type>DamageTaken`
//!   INC (plus `ElementalDamageTaken` for an elemental type, and
//!   projectile and trap or mine damage taken for such skills),
//!   `takenMore` their MORE product (`ModDB:More`, each name rounded to two
//!   decimals). For physical damage the resistance is the armour's
//!   reduction, computed on the hit before `effMult`.
//! - **Shock** (`Modules/ConfigOptions.lua` 1917 to 1924 in the pinned
//!   checkout): the `conditionEnemyShocked` check sets the enemy's
//!   `Condition:Shocked` and `Condition:ShockedConfig`; the
//!   `conditionShockEffect` count sets the enemy's `ShockVal` BASE, only
//!   while `ShockedConfig`. Then `calcs.perform` (`Modules/CalcPerform.lua`
//!   3499 to 3546): the shock is `floor(min(max(override, ShockVal),
//!   max))` (`ailmentData.Shock`: precision 0, `Modules/Data.lua` 427),
//!   `override` the strongest guaranteed shock (`ShockBase`,
//!   `ShockOverride`, `ShockMinimum`: modifiers), `max` the `ShockMax`
//!   override, else 50 plus each active skill's `ShockMax` BASE, the
//!   largest. A `ShockMinimum` (the Elementalist's Shaper of Storms:
//!   "Shocks from your Hits always increase Damage taken by at least 25%")
//!   is scaled by the character's `EnemyShockEffect` (`calcLib.mod`, no
//!   `cfg`) and the enemy's `SelfShockEffect`, and the minimums add up;
//!   unlike `ShockOverride` it does not shock the enemy on its own, so it
//!   counts only with the check on. The enemy takes `DamageTaken` INC
//!   equal to the shock while shocked (with `ShockCanStack`, per stack: a
//!   modifier's flag). Nothing happens when the enemy is already shocked,
//!   immune, or avoids shock (enemy modifiers only). The guard `ShockVal >
//!   0 or modDB:Sum(...)` is always true in Lua (0 is true), so with no
//!   configured effect and no guaranteed shock the shock is 0.
//! - **The boss kind** (`enemyIsBoss`, `Modules/ConfigOptions.lua` 2086 to
//!   2221): the Uber kind gives the enemy `DamageTaken` MORE -70. The
//!   kinds' other modifiers reach no slice field ([`super::enemy`] module
//!   docs: conditions and multipliers modifiers read, warcry power,
//!   the ailment threshold).
//! - **Delirium** (`deliriousPercentage`, 2222 to 2243): 20% to 100% give
//!   the enemy `DamageTaken` MORE -16 to -80 and `Damage` INC 6 to 30 (the
//!   enemy's damage: only PoB's damage-taken estimates read it, no slice
//!   field); `0` ("None") nothing.
//! - **Intimidate and Unnerve** (`conditionEnemyIntimidated`,
//!   `conditionEnemyUnnerved`, `Modules/ConfigOptions.lua` 1956, 1977):
//!   the enemy's `Condition:Intimidated` or `Condition:Unnerved`;
//!   `calcs.initModDB` (`Modules/CalcSetup.lua` 64 to 67) gives every
//!   actor, the enemy too, `DamageTaken` INC 10 with `ModFlag.Attack`
//!   while intimidated and with `ModFlag.Spell` while unnerved: an
//!   attack's or a spell's hit takes it (the party's variants need a party
//!   member's condition: not modelled, no party).
//! - **Blind** (`conditionEnemyBlinded`, 1834): the enemy's
//!   `Condition:Blinded`, which `calcs.initModDB` (74) turns into the
//!   `Blind` flag; `doActorMisc` (`Modules/CalcPerform.lua` 824 to 835,
//!   run for the enemy at 3559) then gives the enemy `Accuracy` and
//!   `Evasion` MORE `floor(-20 x effect)`, `effect` 1 plus the enemy's
//!   `BlindEffect` and `BuffEffectOnSelf` INC (`blind_effect_+%`, "#%
//!   increased Blind Effect", is an `EnemyModifier` of `BlindEffect`,
//!   `ModParser.lua` 2238: it must stay a reason, [`BLIND_DEPENDS_ON`]),
//!   unless the enemy cannot be blinded or is unaffected (enemy modifiers
//!   only). The enemy's evasion is an attack's chance to hit; its accuracy
//!   reaches no slice field.
//!
//! What the engine does: one quantity, [`Poe1Quantity::EnemyDamageTaken`]
//! (PoB's `DamageTaken` on the enemy, every damage type), whose effects
//! only this module adds; a hit multiplies each type's enemy effect by
//! `(1 + inc / 100) x more` of it ([`super::hit`]). The shock (P3-8k) is
//! PoB's from the configured effect and the guaranteed minimums of
//! [`SHOCK_STATS`] ([`Poe1Quantity::ShockMinimum`], scaled by
//! [`Poe1Quantity::ShockEffect`]), computed once every source is in; its
//! effect's source is the configured effect's input, or the first
//! minimum's when the minimum is stronger (the shock's other sources are
//! no graph node: a documented gap, as the charges' count). Damage taken
//! of one type, elemental, projectile, or trap and mine damage taken, and
//! everything else that sets a shock (another guaranteed shock, a
//! different maximum, stacks, immunity, the enemy's shock effect) comes
//! only from modifiers, so from stats: no mapping produces them (tests in
//! this module, [`SHOCK_DEPENDS_ON`]), so such a stat is a reason or
//! reaches no slice field.

use loom_domain::{Build, ConfigInputKind, ConfigSet, ConfigValue};

use super::Poe1Quantity;
use super::config::last_input as last;
use super::enemy::{self, BossKind};
use crate::condition::Flags;
use crate::ingest::Ingest;
use crate::input::CalcData;
use crate::quantity::{Op, Quantity};
use crate::source::Source;
use crate::state::UnsupportedReason;
use crate::store::{Conditions, Effect};

/// The check that shocks the enemy.
pub const SHOCKED_INPUT: &str = "conditionEnemyShocked";
/// The count of the shock's effect.
pub const SHOCK_EFFECT_INPUT: &str = "conditionShockEffect";
/// The delirium list.
pub const DELIRIUM_INPUT: &str = "deliriousPercentage";
/// The boss kind list.
pub const BOSS_INPUT: &str = "enemyIsBoss";
/// The check that intimidates the enemy.
pub const INTIMIDATED_INPUT: &str = "conditionEnemyIntimidated";
/// The check that unnerves the enemy.
pub const UNNERVED_INPUT: &str = "conditionEnemyUnnerved";
/// The check that blinds the enemy.
pub const BLINDED_INPUT: &str = "conditionEnemyBlinded";

/// Intimidate's and Unnerve's increased damage taken (`CalcSetup.lua` 64,
/// 66).
pub const INTIMIDATE_UNNERVE_DAMAGE_TAKEN: f64 = 10.0;

/// Blind's less evasion without modifiers (`floor(-20 x 1)`).
pub const BLIND_LESS_EVASION: f64 = -20.0;

/// The stats through which a blinded enemy's evasion would differ from
/// [`BLIND_LESS_EVASION`]: they must stay reasons (a test checks each is
/// `Unmapped` through every mapping; the real-bundle test that they are
/// stats).
pub const BLIND_DEPENDS_ON: &[&str] = &["blind_effect_+%"];

/// The largest shock without modifiers (`ailmentData.Shock.max`,
/// `Modules/Data.lua` 427).
pub const SHOCK_MAX: f64 = 50.0;

/// One stat: its quantity and op (scale 1).
type ShockRow = (&'static str, Poe1Quantity, Op);

/// The stats of the guaranteed shock (P3-8k, module docs), sorted by stat
/// ID. Each description is the game's, read by PoB's text:
/// - "#% increased Effect of Lightning Ailments" (`ModParser.lua` 776:
///   `EnemyShockEffect` and `EnemySapEffect`), "#% increased Effect of
///   Non-Damaging Ailments" (785: every non-damaging ailment's effect),
///   "#% increased Effect of Shock" (772): `EnemyShockEffect` INC. The
///   other ailments' effects reach no slice field: chill slows the enemy,
///   sap lessens its damage, and scorch, brittle, and a chill's damage
///   taken come only from config inputs that stay reasons or from
///   guaranteed sources that are stats PoB reads otherwise (reasons);
/// - "Shocks from your Hits always increase Damage taken by at least #%"
///   (2768): `ShockMinimum` BASE.
pub const SHOCK_STATS: [ShockRow; 4] = [
    (
        "lightning_ailment_effect_+%",
        Poe1Quantity::ShockEffect,
        Op::Increased,
    ),
    (
        "non_damaging_ailment_effect_+%",
        Poe1Quantity::ShockEffect,
        Op::Increased,
    ),
    ("shock_effect_+%", Poe1Quantity::ShockEffect, Op::Increased),
    (
        "shock_minimum_damage_taken_increase_%",
        Poe1Quantity::ShockMinimum,
        Op::Base,
    ),
];

/// Stats PoB reads that would change the shock otherwise than [`shock`]
/// does, sorted: they must stay reasons (a test checks each is `Unmapped`
/// through every mapping; a maintainer-run test that each is a stat of
/// the real bundle, and that every other stat whose text names shock or
/// the effect of lightning or non-damaging ailments is a row, `Unmapped`,
/// or a listed `NoEffect` row):
/// - a more effect (Pure Elementalist's "30% more Effect of
///   non-Damaging Ailments" for elemental spells) and the Elementalist's
///   "more Effect of Lightning Ailments ... if the highest Damage Type is
///   Lightning", whose `ChecksHighestDamage` flag makes PoB read the main
///   skill's list instead (`CalcPerform.lua` 3517);
/// - effects with a condition: against chilled enemies, per Elder item,
///   per blue gem, with critical strikes, a flask's during its effect;
/// - the maximum: "+#% to Maximum Effect of Shock" (PoB reads it per
///   active skill, `CalcPerform.lua` 3532 to 3539) and its override;
/// - stacking shocks (`ShockCanStack`, also in
///   [`super::config::DEPENDS_ON`]).
pub const SHOCK_DEPENDS_ON: &[&str] = &[
    "keystone_pure_elementalist",
    "lightning_ailment_effect_+%_against_chilled_enemies",
    "lightning_ailments_effect_+%_final_if_hit_highest_lightning",
    "local_flask_non_damaging_ailment_effect_+%_during_flask_effect",
    "local_unique_vinktars_flask_shock_effect_+%_during_flask_effect",
    "non_damaging_ailment_effect_+%_per_blue_skill_gem",
    "non_damaging_ailment_effect_+%_per_equipped_elder_item",
    "non_damaging_ailment_effect_+%_with_critical_strikes",
    "non_damaging_ailment_effect_+%_with_critical_strikes_per_100_max_player_life",
    "raider_shock_maximum_effect_override_%",
    "raider_shocks_you_apply_can_stack_up_to_50_times",
    "shock_maximum_magnitude_+",
];

/// The effects of a stat of [`SHOCK_STATS`]: `(quantity, op, scale)`.
#[must_use]
pub fn shock_effects(stat: &str) -> Option<Vec<(Quantity, Op, f64)>> {
    let i = SHOCK_STATS
        .binary_search_by(|(id, ..)| (*id).cmp(stat))
        .ok()?;
    let (_, quantity, op) = SHOCK_STATS[i];
    Some(vec![(Quantity::Poe1(quantity), op, 1.0)])
}

/// The Uber boss's less damage taken (`DamageTaken` MORE).
pub const UBER_DAMAGE_TAKEN: f64 = -70.0;

/// Delirium's values and their less damage taken (`DamageTaken` MORE).
pub const DELIRIUM: [(&str, f64); 5] = [
    ("20Percent", -16.0),
    ("40Percent", -32.0),
    ("60Percent", -48.0),
    ("80Percent", -64.0),
    ("100Percent", -80.0),
];

/// Whether this module handles `input` (the boss kind only as a string:
/// the enemy model reports another type, [`super::enemy`]).
#[must_use]
pub fn handles(input: &str, value: &ConfigValue) -> bool {
    match input {
        SHOCKED_INPUT | SHOCK_EFFECT_INPUT | DELIRIUM_INPUT | INTIMIDATED_INPUT
        | UNNERVED_INPUT | BLINDED_INPUT => true,
        BOSS_INPUT => matches!(value, ConfigValue::String(_)),
        _ => false,
    }
}

/// A count option's value (`BuildModList`: the input when set and not 0,
/// else the placeholder when set and not 0): `Ok(None)` when neither is,
/// `Err(())` for a value that is not a finite number.
fn count(set: &ConfigSet, name: &str) -> Result<Option<f64>, ()> {
    let read = |kind| match last(set, name, kind) {
        None => Ok(None),
        Some(ConfigValue::Number(n)) if !n.value().0.is_finite() => Err(()),
        Some(ConfigValue::Number(n)) => Ok(Some(n.value().0).filter(|n| *n != 0.0)),
        Some(ConfigValue::Boolean(_) | ConfigValue::String(_)) => Err(()),
    };
    match read(ConfigInputKind::Input)? {
        Some(n) => Ok(Some(n)),
        None => read(ConfigInputKind::Placeholder),
    }
}

/// An effect on the enemy's damage taken from a config input.
fn taken(op: Op, value: f64, input: &str) -> Effect {
    Effect {
        quantity: Quantity::Poe1(Poe1Quantity::EnemyDamageTaken),
        op,
        value,
        conditions: Conditions::none(),
        source: Source::Config {
            input: input.to_owned(),
        },
    }
}

/// A check input's value: `Ok(false)` when unset or off, `Err(())` for a
/// value that is not a boolean (PoB reads any other value as on; not
/// modelled, a reason).
fn check(set: &ConfigSet, name: &str) -> Result<bool, ()> {
    match last(set, name, ConfigInputKind::Input) {
        None | Some(ConfigValue::Boolean(false)) => Ok(false),
        Some(ConfigValue::Boolean(true)) => Ok(true),
        Some(ConfigValue::Number(_) | ConfigValue::String(_)) => Err(()),
    }
}

/// Intimidate, Unnerve, and Blind (module docs).
fn conditions(set: &ConfigSet, ingest: &mut Ingest<'_>) {
    let checks = [
        (INTIMIDATED_INPUT, Flags::ATTACK),
        (UNNERVED_INPUT, Flags::SPELL),
        (BLINDED_INPUT, Flags::NONE),
    ];
    for (input, flag) in checks {
        match check(set, input) {
            Ok(false) => {}
            Err(()) => ingest.reason(UnsupportedReason::UnmodelledConfig {
                input: input.to_owned(),
            }),
            Ok(true) if input == BLINDED_INPUT => ingest.effect(Effect {
                quantity: Quantity::Poe1(Poe1Quantity::EnemyEvasion),
                op: Op::More,
                value: BLIND_LESS_EVASION,
                conditions: Conditions::none(),
                source: Source::Config {
                    input: input.to_owned(),
                },
            }),
            Ok(true) => ingest.effect(Effect {
                conditions: Conditions::none().with_all(flag),
                ..taken(Op::Increased, INTIMIDATE_UNNERVE_DAMAGE_TAKEN, input)
            }),
        }
    }
}

/// The unconditional effects on `quantity` with `op` ingested so far, in
/// order. Every row of [`SHOCK_STATS`] is unconditional (a test), so a
/// condition can only come from a later row, which must bring its
/// handling here.
fn unconditional<'a>(
    ingest: &'a Ingest<'_>,
    quantity: Poe1Quantity,
    op: Op,
) -> impl Iterator<Item = &'a Effect> {
    ingest.effects().iter().filter(move |e| {
        e.quantity == Quantity::Poe1(quantity) && e.op == op && e.conditions == Conditions::none()
    })
}

/// The shock the config gives the enemy (module docs) and its source: the
/// configured effect's input, or the first guaranteed shock's source when
/// the guaranteed shock is stronger. `None` when the enemy is not shocked
/// or the shock is 0, `Err` with the input whose value PoB would read
/// otherwise than the engine.
///
/// # Errors
/// The input's name, for a check that is not a boolean or a count that
/// is not a finite number.
pub fn shock(set: &ConfigSet, ingest: &Ingest<'_>) -> Result<Option<(f64, Source)>, &'static str> {
    match last(set, SHOCKED_INPUT, ConfigInputKind::Input) {
        None | Some(ConfigValue::Boolean(false)) => return Ok(None),
        Some(ConfigValue::Boolean(true)) => {}
        // PoB reads any other value as on, and a string effect as a
        // string: not modelled, a reason.
        Some(ConfigValue::Number(_) | ConfigValue::String(_)) => return Err(SHOCKED_INPUT),
    }
    // `ShockVal` BASE: the configured effect, 0 without one.
    let configured = count(set, SHOCK_EFFECT_INPUT)
        .map_err(|()| SHOCK_EFFECT_INPUT)?
        .unwrap_or(0.0);
    // The guaranteed shocks (`CalcPerform.lua` 3503 to 3530): each
    // `ShockMinimum` times `calcLib.mod(modDB, nil, "EnemyShockEffect")`,
    // `(1 + inc / 100) x round(more, 2)` (no row is `More`: 1), times the
    // enemy's `SelfShockEffect` (1: no enemy modifier); `minimum` sums
    // them and `override` keeps the largest of either.
    let inc: f64 = unconditional(ingest, Poe1Quantity::ShockEffect, Op::Increased)
        .map(|e| e.value)
        .sum();
    let scale = 1.0 + inc / 100.0;
    let mut guaranteed = 0.0_f64;
    let mut minimum = 0.0_f64;
    let mut first = None;
    for e in unconditional(ingest, Poe1Quantity::ShockMinimum, Op::Base) {
        let effect = e.value * scale;
        minimum += effect;
        guaranteed = crate::math::max(crate::math::max(guaranteed, effect), minimum);
        first.get_or_insert_with(|| e.source.clone());
    }
    // `floor(min(max(override, ShockVal), max))`, the maximum 50 without
    // modifiers (precision 0).
    let shock = crate::math::floor(crate::math::min(
        crate::math::max(guaranteed, configured),
        SHOCK_MAX,
    ));
    if shock <= 0.0 {
        return Ok(None);
    }
    let source = match first {
        Some(source) if guaranteed > configured => source,
        _ => Source::Config {
            input: SHOCK_EFFECT_INPUT.to_owned(),
        },
    };
    Ok(Some((shock, source)))
}

/// Adds the enemy's damage taken from the active config set (module
/// docs). A value the engine does not read as PoB does is a reason under
/// its input.
pub fn damage_taken(
    build: &Build,
    set: Option<&ConfigSet>,
    data: &dyn CalcData,
    ingest: &mut Ingest<'_>,
) {
    let Some(set) = set else { return };
    conditions(set, ingest);
    match shock(set, ingest) {
        Ok(Some((shock, source))) => ingest.effect(Effect {
            source,
            ..taken(Op::Increased, shock, SHOCK_EFFECT_INPUT)
        }),
        Ok(None) => {}
        Err(input) => ingest.reason(UnsupportedReason::UnmodelledConfig {
            input: input.to_owned(),
        }),
    }
    // The enemy model reads the kind; an unknown one or another type is
    // its problem (a reason on the fields that read the enemy), read as
    // the default kind, never as Uber.
    if enemy::enemy(build, data).kind == BossKind::Uber {
        ingest.effect(taken(Op::More, UBER_DAMAGE_TAKEN, BOSS_INPUT));
    }
    match last(set, DELIRIUM_INPUT, ConfigInputKind::Input) {
        // The list's "None" (`val = 0`), or absent: nothing.
        None => {}
        Some(ConfigValue::Number(n)) if n.value().0 == 0.0 => {}
        Some(ConfigValue::String(s)) => match DELIRIUM.iter().find(|(v, _)| v == s) {
            Some(&(_, more)) => ingest.effect(taken(Op::More, more, DELIRIUM_INPUT)),
            None => ingest.reason(UnsupportedReason::UnmodelledConfig {
                input: DELIRIUM_INPUT.to_owned(),
            }),
        },
        Some(_) => ingest.reason(UnsupportedReason::UnmodelledConfig {
            input: DELIRIUM_INPUT.to_owned(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poe1::mapping::{HIT_ROWS, LOCAL_ROWS, NO_EFFECT, ROWS, WEAPON_ROWS};

    /// A word of the stat ID is one of `words`.
    fn names(id: &str, words: &[&str]) -> bool {
        id.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|w| words.contains(&w))
    }

    /// `NoEffect` rows that name shock or damage taken, each checked: none
    /// sets the enemy's shock or damage taken.
    const NO_EFFECT_ROWS: &[&str] = &[
        // A chance to shock: PoB shocks the enemy only from its config or
        // a guaranteed source (P3-10, P3-8c; "All Damage can Shock" and
        // "Hits always Shock", P3-8k).
        "all_damage_can_shock",
        "always_shock",
        "base_chance_to_shock_%_from_skill",
        // Shock on you: not the enemy's.
        "cannot_be_shocked_while_at_maximum_power_charges",
        "chance_to_freeze_shock_ignite_%",
        // Damage taken by you, as another type: not the enemy's.
        "physical_damage_taken_%_as_fire",
        "physical_damage_taken_%_as_lightning",
        // The pantheon's souls (P3-12g): damage taken by you, its
        // reduction, and shock on you, not the enemy's.
        "chaos_damage_taken_+%",
        "chaos_damage_taken_over_time_+%_while_in_caustic_cloud",
        "elemental_damage_taken_+%_if_been_hit_recently",
        "elemental_damage_taken_+%_if_not_hit_recently",
        "physical_damage_over_time_taken_+%_while_moving",
        "physical_damage_reduction_%_per_hit_you_have_taken_recently",
        "shocked_effect_on_self_+%",
    ];

    /// The module's argument: no mapped stat reads or sets the enemy's
    /// shock or damage taken, so every stat that does is a reason or
    /// reaches no slice field.
    #[test]
    fn no_mapped_stat_should_set_the_enemys_shock_or_damage_taken() {
        let words = ["shock", "shocked", "shocks", "taken"];
        let ids = HIT_ROWS
            .iter()
            .map(|r| r.0)
            .chain(ROWS.iter().map(|r| r.0))
            .chain(LOCAL_ROWS.iter().map(|r| r.0))
            .chain(WEAPON_ROWS.iter().map(|r| r.0));
        for id in ids {
            assert!(!names(id, &words), "{id}: revisit poe1::damage_taken");
        }
        for (id, _) in NO_EFFECT {
            assert!(
                !names(id, &words) || NO_EFFECT_ROWS.contains(id),
                "{id}: a NoEffect row naming shock or damage taken; check it and list it"
            );
        }
        for id in NO_EFFECT_ROWS {
            assert!(NO_EFFECT.iter().any(|(r, _)| r == id), "{id}: not a row");
        }
    }

    /// No mapped row, and no `NoEffect` row but the listed one, names
    /// Intimidate, Unnerve, or Blind (P3-12g): such a stat (intimidating or
    /// blinding on hit, a stronger Blind) is a reason, so the configured
    /// conditions are the only ones the engine applies, as in a build
    /// without those stats in PoB.
    #[test]
    fn no_mapped_stat_should_name_intimidate_unnerve_or_blind() {
        use crate::ingest::{StatMap, StatMapping};
        use crate::poe1::ArmourSlot;
        use crate::poe1::mapping::{PieceMapping, Poe1Mapping, SkillMapping, WeaponMapping};
        use loom_domain::StatId;

        let words = [
            "intimidate",
            "intimidated",
            "intimidates",
            "unnerve",
            "unnerved",
            "unnerves",
            "blind",
            "blinded",
            "blinds",
        ];
        // Blind on you, not the enemy's (the pantheon's Garukhan).
        let listed = ["cannot_be_blinded"];
        let ids = HIT_ROWS
            .iter()
            .map(|r| r.0)
            .chain(ROWS.iter().map(|r| r.0))
            .chain(LOCAL_ROWS.iter().map(|r| r.0))
            .chain(WEAPON_ROWS.iter().map(|r| r.0));
        for id in ids {
            assert!(!names(id, &words), "{id}: revisit poe1::damage_taken");
        }
        for (id, _) in NO_EFFECT {
            assert!(
                !names(id, &words) || listed.contains(id),
                "{id}: a NoEffect row naming Intimidate, Unnerve, or Blind"
            );
        }
        let piece = PieceMapping {
            slot: ArmourSlot::Helmet,
        };
        let mappings: [&dyn StatMapping; 4] = [&Poe1Mapping, &SkillMapping, &piece, &WeaponMapping];
        for stat in BLIND_DEPENDS_ON {
            for m in mappings {
                assert_eq!(m.map(&StatId::new(*stat)), StatMap::Unmapped, "{stat}");
            }
        }
    }

    /// The shock's rows (P3-8k): sorted, read unconditionally by the
    /// character mapping, reasons through the gem mapping (PoB scales a
    /// guaranteed shock by the character's `EnemyShockEffect`, not the
    /// skill's); the stats left stay reasons through every mapping.
    #[test]
    fn the_shock_rows_should_be_read_as_pob_does() {
        use crate::ingest::{StatMap, StatMapping};
        use crate::poe1::ArmourSlot;
        use crate::poe1::mapping::{PieceMapping, Poe1Mapping, SkillMapping, WeaponMapping};
        use loom_domain::StatId;

        assert!(SHOCK_STATS.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(SHOCK_DEPENDS_ON.windows(2).all(|w| w[0] < w[1]));
        for (stat, quantity, op) in SHOCK_STATS {
            let StatMap::Effects(effects) = Poe1Mapping.map(&StatId::new(stat)) else {
                panic!("{stat} is not mapped");
            };
            assert_eq!(effects.len(), 1, "{stat}");
            assert_eq!(effects[0].quantity, Quantity::Poe1(quantity), "{stat}");
            assert_eq!(effects[0].op, op, "{stat}");
            assert_eq!(effects[0].conditions, Conditions::none(), "{stat}");
            assert_eq!(
                SkillMapping.map(&StatId::new(stat)),
                StatMap::Unmapped,
                "{stat}"
            );
            assert!(!SHOCK_DEPENDS_ON.contains(&stat), "{stat}");
        }
        let piece = PieceMapping {
            slot: ArmourSlot::Helmet,
        };
        let mappings: [&dyn StatMapping; 4] = [&Poe1Mapping, &SkillMapping, &piece, &WeaponMapping];
        for &stat in SHOCK_DEPENDS_ON {
            for m in mappings {
                assert_eq!(m.map(&StatId::new(stat)), StatMap::Unmapped, "{stat}");
            }
        }
    }

    /// PoB's arithmetic (`CalcPerform.lua` 3503 to 3546): the minimums
    /// scaled and summed, the larger of them and the configured effect,
    /// at most 50, floored; the source the stronger one's.
    #[test]
    fn the_shock_should_follow_pobs_arithmetic() {
        use crate::poe1::Poe1Mapping;
        use loom_domain::{ConfigInput, ConfigSetId, Number, Ordered, Sourced};

        let set = |inputs: &[(&str, ConfigValue)]| {
            let mut s = ConfigSet {
                id: ConfigSetId(1),
                title: None,
                inputs: Ordered::new(),
                custom_modifiers: Vec::new(),
                attributes: Vec::new(),
            };
            for (name, value) in inputs {
                s.inputs.push(ConfigInput {
                    name: (*name).to_owned(),
                    value: value.clone(),
                    kind: ConfigInputKind::Input,
                    lines: Vec::new(),
                    attributes: Vec::new(),
                });
            }
            s
        };
        let passive = |node| Source::Passive {
            node: loom_domain::NodeId(node),
        };
        let effect = |quantity, op, value, source| Effect {
            quantity: Quantity::Poe1(quantity),
            op,
            value,
            conditions: Conditions::none(),
            source,
        };
        let shocked = |configured: f64| {
            set(&[
                (SHOCKED_INPUT, ConfigValue::Boolean(true)),
                (
                    SHOCK_EFFECT_INPUT,
                    ConfigValue::Number(Sourced::new(Number(configured))),
                ),
            ])
        };
        let config = Source::Config {
            input: SHOCK_EFFECT_INPUT.to_owned(),
        };
        for (effects, configured, want) in [
            (vec![], 20.0, Some((20.0, config.clone()))),
            (
                vec![effect(
                    Poe1Quantity::ShockMinimum,
                    Op::Base,
                    25.0,
                    passive(1),
                )],
                20.0,
                Some((25.0, passive(1))),
            ),
            // 25 x 1.1 = 27.500000000000004, floored.
            (
                vec![
                    effect(Poe1Quantity::ShockEffect, Op::Increased, 10.0, passive(2)),
                    effect(Poe1Quantity::ShockMinimum, Op::Base, 25.0, passive(1)),
                ],
                20.0,
                Some((27.0, passive(1))),
            ),
            // A tie keeps the configured effect's source.
            (
                vec![effect(
                    Poe1Quantity::ShockMinimum,
                    Op::Base,
                    20.0,
                    passive(1),
                )],
                20.0,
                Some((20.0, config.clone())),
            ),
            // Summed minimums, the first one's source.
            (
                vec![
                    effect(Poe1Quantity::ShockMinimum, Op::Base, 10.0, passive(3)),
                    effect(Poe1Quantity::ShockMinimum, Op::Base, 25.0, passive(1)),
                ],
                0.0,
                Some((35.0, passive(3))),
            ),
            // A conditional effect is not read (no row gives one).
            (
                vec![Effect {
                    conditions: Conditions::none().with_all(crate::condition::Flags::SPELL),
                    ..effect(Poe1Quantity::ShockMinimum, Op::Base, 25.0, passive(1))
                }],
                0.0,
                None,
            ),
        ] {
            let mut ingest = Ingest::new(&Poe1Mapping);
            for e in effects.clone() {
                ingest.effect(e);
            }
            assert_eq!(
                shock(&shocked(configured), &ingest),
                Ok(want),
                "{effects:?}"
            );
        }
        // Without the check, a minimum shocks nothing.
        let mut ingest = Ingest::new(&Poe1Mapping);
        ingest.effect(effect(
            Poe1Quantity::ShockMinimum,
            Op::Base,
            25.0,
            passive(1),
        ));
        assert_eq!(shock(&set(&[]), &ingest), Ok(None));
    }

    #[test]
    fn the_inputs_should_not_be_named_only() {
        for input in [
            SHOCKED_INPUT,
            SHOCK_EFFECT_INPUT,
            DELIRIUM_INPUT,
            BOSS_INPUT,
            INTIMIDATED_INPUT,
            UNNERVED_INPUT,
            BLINDED_INPUT,
        ] {
            assert!(!super::super::config::named_only(input), "{input}");
            assert!(!super::super::config::enemy_damage(input), "{input}");
            assert!(!super::super::charges::is_charge_input(input), "{input}");
        }
    }
}
