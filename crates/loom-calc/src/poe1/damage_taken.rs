//! The enemy's damage taken from the config (Phase 3 plan, step 12;
//! P3-12f): shock, the Uber boss kind, and delirium.
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
//! - **Shock** (`Modules/ConfigOptions.lua` 1943 to 1950): the
//!   `conditionEnemyShocked` check sets the enemy's `Condition:Shocked`
//!   and `Condition:ShockedConfig`; the `conditionShockEffect` count sets
//!   the enemy's `ShockVal` BASE, only while `ShockedConfig`. Then
//!   `calcs.perform` (`Modules/CalcPerform.lua` 3545 to 3640): the shock
//!   is `floor(min(max(override, ShockVal), max))` (`ailmentData.Shock`:
//!   precision 0, `Modules/Data.lua` 427), `override` the strongest
//!   guaranteed shock (`ShockBase`, `ShockOverride`, `ShockMinimum`:
//!   modifiers), `max` the `ShockMax` override, else 50 plus each active
//!   skill's `ShockMax` BASE, the largest. The enemy takes `DamageTaken`
//!   INC equal to the shock (with `ShockCanStack`, per stack: a
//!   modifier's flag). Nothing happens when the enemy is already shocked,
//!   immune, or avoids shock (enemy modifiers only). The guard `ShockVal >
//!   0 or modDB:Sum(...)` is always true in Lua (0 is true), so with no
//!   configured effect the shock is 0.
//! - **The boss kind** (`enemyIsBoss`, `Modules/ConfigOptions.lua` 2086 to
//!   2221): the Uber kind gives the enemy `DamageTaken` MORE -70. The
//!   kinds' other modifiers reach no slice field ([`super::enemy`] module
//!   docs: conditions and multipliers modifiers read, warcry power,
//!   the ailment threshold).
//! - **Delirium** (`deliriousPercentage`, 2222 to 2243): 20% to 100% give
//!   the enemy `DamageTaken` MORE -16 to -80 and `Damage` INC 6 to 30 (the
//!   enemy's damage: only PoB's damage-taken estimates read it, no slice
//!   field); `0` ("None") nothing.
//!
//! What the engine does: one quantity, [`Poe1Quantity::EnemyDamageTaken`]
//! (PoB's `DamageTaken` on the enemy, every damage type), whose effects
//! only this module adds; a hit multiplies each type's enemy effect by
//! `(1 + inc / 100) x more` of it ([`super::hit`]). Damage taken of one
//! type, elemental, projectile, or trap and mine damage taken, and
//! everything else that sets a shock (a guaranteed shock, a different
//! maximum, stacks, immunity, the enemy's shock effect) comes only from
//! modifiers, so from stats: no mapping produces them (tests in this
//! module), so such a stat is a reason or reaches no slice field.

use loom_domain::{Build, ConfigInputKind, ConfigSet, ConfigValue};

use super::Poe1Quantity;
use super::config::last_input as last;
use super::enemy::{self, BossKind};
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

/// The largest shock without modifiers (`ailmentData.Shock.max`,
/// `Modules/Data.lua` 427).
pub const SHOCK_MAX: f64 = 50.0;

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
        SHOCKED_INPUT | SHOCK_EFFECT_INPUT | DELIRIUM_INPUT => true,
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

/// The shock the config gives the enemy (module docs): `None` when the
/// enemy is not shocked or the shock is 0, `Err` with the input whose
/// value PoB would read otherwise than the engine.
///
/// # Errors
/// The input's name, for a check that is not a boolean or a count that
/// is not a finite number.
pub fn shock(set: &ConfigSet) -> Result<Option<f64>, &'static str> {
    match last(set, SHOCKED_INPUT, ConfigInputKind::Input) {
        None | Some(ConfigValue::Boolean(false)) => return Ok(None),
        Some(ConfigValue::Boolean(true)) => {}
        // PoB reads any other value as on, and a string effect as a
        // string: not modelled, a reason.
        Some(ConfigValue::Number(_) | ConfigValue::String(_)) => return Err(SHOCKED_INPUT),
    }
    let value = count(set, SHOCK_EFFECT_INPUT).map_err(|()| SHOCK_EFFECT_INPUT)?;
    // `floor(min(max(0, ShockVal), max))`: the override is 0 without a
    // guaranteed shock, the maximum 50 without modifiers.
    let shock = crate::math::floor(value.unwrap_or(0.0).clamp(0.0, SHOCK_MAX));
    Ok(Some(shock).filter(|s| *s > 0.0))
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
    match shock(set) {
        Ok(Some(shock)) => ingest.effect(taken(Op::Increased, shock, SHOCK_EFFECT_INPUT)),
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
        // a guaranteed source (P3-10, P3-8c).
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

    #[test]
    fn the_inputs_should_not_be_named_only() {
        for input in [
            SHOCKED_INPUT,
            SHOCK_EFFECT_INPUT,
            DELIRIUM_INPUT,
            BOSS_INPUT,
        ] {
            assert!(!super::super::config::named_only(input), "{input}");
            assert!(!super::super::config::enemy_damage(input), "{input}");
            assert!(!super::super::charges::is_charge_input(input), "{input}");
        }
    }
}
