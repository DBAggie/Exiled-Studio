//! Buffs the config turns on (Phase 3 plan, step 12; P3-12f): Onslaught;
//! since P3-8n, Arcane Surge and Convergence.
//!
//! What PoB v2.67.2 does:
//! - `buffOnslaught` (`Modules/ConfigOptions.lua` 1010 to 1012), a check,
//!   sets `Condition:Onslaught` in combat; `calcs.initModDB`
//!   (`Modules/CalcSetup.lua` 97) turns it into the `Onslaught` flag;
//! - `calcs.perform` (`Modules/CalcPerform.lua` 682 to 711): with the
//!   flag, the effect is `floor(20 x (1 + inc / 100))`, `inc` the
//!   `OnslaughtEffect` and `BuffEffectOnSelf` INC sums (and a Silver
//!   Flask's effect when one is active), given as increased attack speed,
//!   increased cast speed, and increased movement speed (not a slice
//!   field).
//!
//! - a modifier "Onslaught" (`base_should_have_onslaught_from_stat`; PoB
//!   `ModParser.lua` 3973, in a custom block or on an item) sets the same
//!   condition without the combat tag.
//!
//! What the engine does: with the check on, or any unconditional `Flag`
//! effect on [`Poe1Quantity::Onslaught`] (the stat's mapping row, P3-12j),
//! `Speed` INC 20 for attacks and for casts, once: PoB's flag counts once
//! whatever the number of sources. Its source is the config input, else
//! the first flag's. Everything else that changes it is a modifier, so a
//! stat ([`DEPENDS_ON`], which must stay reasons: a test), or an active
//! flask, which is a reason on its own (`collect`). Other sources of
//! Onslaught (on kill, from a flask, a skill) are modifiers or other gems,
//! reasons too.
//!
//! **Arcane Surge** (P3-8n). `buffArcaneSurge` (`ConfigOptions.lua` 1002)
//! sets `Condition:ArcaneSurge`; `CalcPerform.lua` (686 to 705) then sets
//! `AffectedByArcaneSurge` (read by Arcane Surge Support's more spell
//! damage, a mapping row with [`super::conditions::ARCANE_SURGE`]) and
//! gives cast `Speed` INC `(Max(ArcaneSurgeCastSpeed) or 20) x effect`,
//! the effect `1 + (ArcaneSurgeEffect + BuffEffectOnSelf) / 100` (and a
//! flask's share): no effect stat is mapped ([`ARCANE_SURGE_DEPENDS_ON`]),
//! so 1. Its mana regeneration and `ArcaneSurgeDamage` (unmapped) are not
//! here. Its source is the input.
//!
//! **Convergence** (P3-8n). `conditionConvergence` (1430) sets
//! `Condition:Convergence` only with `Condition:CanGainConvergence` ("Gain
//! Convergence when you Hit a Unique Enemy", [`Poe1Quantity::CanGainConvergence`]);
//! `CalcSetup.lua` (108) makes it the `Convergence` flag and
//! `CalcPerform.lua` (755 to 758) gives `ElementalDamage` MORE `floor(30 x
//! (1 + BuffEffectOnSelf / 100))`, here 30 (`BuffEffectOnSelf` unmapped).

use loom_domain::{ConfigInputKind, ConfigSet, ConfigValue};

use super::Poe1Quantity;
use crate::condition::Flags;
use crate::ingest::Ingest;
use crate::quantity::{Op, Quantity};
use crate::source::Source;
use crate::state::UnsupportedReason;
use crate::store::{Conditions, Effect};

/// The check that grants Onslaught.
pub const ONSLAUGHT_INPUT: &str = "buffOnslaught";

/// Onslaught's increased attack and cast speed without modifiers (PoB's
/// constant, `CalcPerform.lua` 704).
pub const ONSLAUGHT_SPEED: f64 = 20.0;

/// The stats that change Onslaught's effect (PoB's `OnslaughtEffect` and
/// `BuffEffectOnSelf`, `ModParser.lua` 478, 682, 683), sorted: they must
/// stay reasons while the engine adds the constant effect.
pub const DEPENDS_ON: &[&str] = &[
    "buff_effect_on_self_+%",
    "map_monster_and_player_onslaught_effect_+%",
    "onslaught_effect_+%",
];

/// The check that grants Arcane Surge (its condition is
/// [`super::conditions`]' too).
pub const ARCANE_SURGE_INPUT: &str = "buffArcaneSurge";

/// Arcane Surge's increased cast speed without a support (PoB's `or 20`).
pub const ARCANE_SURGE_SPEED: f64 = 20.0;

/// The stats that change Arcane Surge's effect (PoB's `ArcaneSurgeEffect`,
/// `BuffEffectOnSelf`, and the flask's share), sorted: they must stay
/// reasons while the engine gives the effect 1.
pub const ARCANE_SURGE_DEPENDS_ON: &[&str] = &[
    "additive_flask_effect_modifiers_apply_to_arcane_surge",
    "additive_flask_effect_modifiers_apply_to_arcane_surge_at_%_value",
    "arcane_surge_cast_speed_+%",
    "arcane_surge_effect_+%",
    "buff_effect_on_self_+%",
];

/// The check that gives Convergence.
pub const CONVERGENCE_INPUT: &str = "conditionConvergence";

/// Convergence's more elemental damage (`CalcPerform.lua` 756).
pub const CONVERGENCE_MORE: f64 = 30.0;

/// Whether this module handles `input` (Arcane Surge's check is
/// [`super::conditions`]').
#[must_use]
pub fn is_buff_input(input: &str) -> bool {
    input == ONSLAUGHT_INPUT || input == CONVERGENCE_INPUT
}

/// A check's value: `Ok(false)` when unset or off, `Err(())` for a value
/// that is not a boolean.
fn check(set: Option<&ConfigSet>, input: &str) -> Result<bool, ()> {
    match set.and_then(|set| super::config::last_input(set, input, ConfigInputKind::Input)) {
        None | Some(ConfigValue::Boolean(false)) => Ok(false),
        Some(ConfigValue::Boolean(true)) => Ok(true),
        Some(ConfigValue::Number(_) | ConfigValue::String(_)) => Err(()),
    }
}

/// Arcane Surge's cast speed (module docs), with the check on.
fn arcane_surge(set: Option<&ConfigSet>, ingest: &mut Ingest<'_>) {
    // An Arcane Surge Support's buff (`poe1::gems`, close-out): its cast
    // speed is a gem's effect; its source is that gem.
    let support = ingest
        .effects()
        .iter()
        .find(|e| {
            e.quantity == Quantity::Poe1(Poe1Quantity::ArcaneSurgeCastSpeed)
                && matches!(e.source, Source::Gem { .. })
        })
        .map(|e| e.source.clone());
    // A value of another type: `conditions::handles` leaves it a reason.
    let checked = check(set, ARCANE_SURGE_INPUT) == Ok(true);
    if !checked && support.is_none() {
        return;
    }
    let speed = ingest
        .effects()
        .iter()
        .filter(|e| {
            e.quantity == Quantity::Poe1(Poe1Quantity::ArcaneSurgeCastSpeed)
                && e.op == Op::Base
                && e.conditions == Conditions::none()
        })
        .map(|e| e.value)
        .fold(None, |m: Option<f64>, v| {
            Some(m.map_or(v, |m| crate::math::max(m, v)))
        })
        .unwrap_or(ARCANE_SURGE_SPEED);
    ingest.effect(Effect {
        quantity: Quantity::Speed,
        op: Op::Increased,
        value: speed,
        conditions: Conditions::none().with_all(Flags::CAST),
        source: if checked {
            Source::Config {
                input: ARCANE_SURGE_INPUT.to_owned(),
            }
        } else {
            support.unwrap_or(Source::Rule("arcane-surge"))
        },
    });
}

/// Convergence's more elemental damage (module docs).
fn convergence(set: Option<&ConfigSet>, ingest: &mut Ingest<'_>) {
    match check(set, CONVERGENCE_INPUT) {
        Ok(false) => {}
        Err(()) => ingest.reason(UnsupportedReason::UnmodelledConfig {
            input: CONVERGENCE_INPUT.to_owned(),
        }),
        Ok(true) => {
            // The line's presence is PoB's flag: its description shows it
            // for any value (an unbounded range), so the import assumes the
            // hidden value 0.
            let can_gain = ingest.effects().iter().any(|e| {
                e.quantity == Quantity::Poe1(Poe1Quantity::CanGainConvergence)
                    && e.op == Op::Flag
                    && e.conditions == Conditions::none()
            });
            if can_gain {
                ingest.effect(Effect {
                    quantity: Quantity::Damage,
                    op: Op::More,
                    value: CONVERGENCE_MORE,
                    conditions: Conditions::none().with_any(Flags::ELEMENTAL),
                    source: Source::Config {
                        input: CONVERGENCE_INPUT.to_owned(),
                    },
                });
            }
        }
    }
}

/// The stat that grants Onslaught (module docs).
pub const ONSLAUGHT_STAT: &str = "base_should_have_onslaught_from_stat";

/// Adds the buffs the active config set or a modifier turns on (module
/// docs). A check value that is not a boolean is a reason under its input
/// (PoB reads any value other than `false` as on). Runs after every other
/// source has been ingested.
pub fn buffs(set: Option<&ConfigSet>, ingest: &mut Ingest<'_>) {
    arcane_surge(set, ingest);
    convergence(set, ingest);
    // The last input counts; a check's placeholder is not applied.
    let mut source = match set
        .and_then(|set| super::config::last_input(set, ONSLAUGHT_INPUT, ConfigInputKind::Input))
    {
        None | Some(ConfigValue::Boolean(false)) => None,
        Some(ConfigValue::Boolean(true)) => Some(Source::Config {
            input: ONSLAUGHT_INPUT.to_owned(),
        }),
        Some(ConfigValue::Number(_) | ConfigValue::String(_)) => {
            ingest.reason(UnsupportedReason::UnmodelledConfig {
                input: ONSLAUGHT_INPUT.to_owned(),
            });
            None
        }
    };
    // Every Onslaught flag is unconditional: the one mapping row has no
    // condition (`flag_tests` pins it). A conditional one would be a new
    // row, which must come with its condition here.
    if source.is_none() {
        source = ingest
            .effects()
            .iter()
            .find(|e| {
                e.quantity == Quantity::Poe1(Poe1Quantity::Onslaught)
                    && e.op == Op::Flag
                    && e.conditions == Conditions::none()
            })
            .map(|e| e.source.clone());
    }
    let Some(source) = source else { return };
    for flags in [Flags::ATTACK, Flags::CAST] {
        ingest.effect(Effect {
            quantity: Quantity::Speed,
            op: Op::Increased,
            value: ONSLAUGHT_SPEED,
            conditions: Conditions::none().with_all(flags),
            source: source.clone(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{StatMap, StatMapping};
    use crate::poe1::ArmourSlot;
    use crate::poe1::mapping::{
        HIT_ROWS, LOCAL_ROWS, NO_EFFECT, PieceMapping, Poe1Mapping, ROWS, SkillMapping,
        WEAPON_ROWS, WeaponMapping,
    };
    use loom_domain::StatId;

    /// No mapped or `NoEffect` stat but [`ONSLAUGHT_STAT`] (a flag) names
    /// Onslaught, and the stats that change its effect stay `Unmapped`
    /// through every mapping.
    #[test]
    fn no_mapped_stat_should_grant_or_change_onslaught() {
        assert!(DEPENDS_ON.windows(2).all(|w| w[0] < w[1]));
        let names = |id: &str| id.contains("onslaught") && id != ONSLAUGHT_STAT;
        let ids = HIT_ROWS
            .iter()
            .map(|r| r.0)
            .chain(ROWS.iter().map(|r| r.0))
            .chain(LOCAL_ROWS.iter().map(|r| r.0))
            .chain(WEAPON_ROWS.iter().map(|r| r.0))
            .chain(NO_EFFECT.iter().map(|r| r.0));
        for id in ids {
            assert!(!names(id), "{id}: revisit poe1::buffs");
        }
        let piece = PieceMapping {
            slot: ArmourSlot::Helmet,
        };
        let mappings: [&dyn StatMapping; 4] = [&Poe1Mapping, &SkillMapping, &piece, &WeaponMapping];
        for stat in DEPENDS_ON.iter().chain(ARCANE_SURGE_DEPENDS_ON) {
            for m in mappings {
                assert_eq!(m.map(&StatId::new(*stat)), StatMap::Unmapped, "{stat}");
            }
        }
    }

    #[test]
    fn the_inputs_should_be_apart_from_the_other_tables() {
        assert!(ARCANE_SURGE_DEPENDS_ON.windows(2).all(|w| w[0] < w[1]));
        assert!(!super::super::config::named_only(CONVERGENCE_INPUT));
        assert!(!super::super::config::named_only(ARCANE_SURGE_INPUT));
        assert!(!super::super::config::named_only(ONSLAUGHT_INPUT));
        assert!(!super::super::charges::is_charge_input(ONSLAUGHT_INPUT));
    }

    fn set(inputs: &[(&str, ConfigValue)]) -> ConfigSet {
        use loom_domain::{ConfigInput, ConfigSetId, Ordered};
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
    }

    /// Convergence (P3-8n): the "Gain Convergence" line's presence is the
    /// flag, whatever its (hidden, assumed 0) value; the check alone gives
    /// nothing.
    #[test]
    fn convergence_should_need_the_line_and_the_check() {
        let flag = Effect {
            quantity: Quantity::Poe1(Poe1Quantity::CanGainConvergence),
            op: Op::Flag,
            value: 0.0,
            conditions: Conditions::none(),
            source: Source::Character,
        };
        let more = |ingest: &Ingest<'_>| {
            ingest
                .effects()
                .iter()
                .filter(|e| e.quantity == Quantity::Damage && e.op == Op::More)
                .map(|e| e.value)
                .collect::<Vec<_>>()
        };
        let on = set(&[(CONVERGENCE_INPUT, ConfigValue::Boolean(true))]);
        let mut with_line = Ingest::new(&Poe1Mapping);
        with_line.effect(flag);
        buffs(Some(&on), &mut with_line);
        assert_eq!(more(&with_line), [CONVERGENCE_MORE]);
        let mut without = Ingest::new(&Poe1Mapping);
        buffs(Some(&on), &mut without);
        assert!(more(&without).is_empty());
    }

    /// The stat's one row: an unconditional flag (`buffs` reads only
    /// those).
    #[test]
    fn the_onslaught_stat_should_be_a_flag() {
        let StatMap::Effects(t) =
            crate::poe1::mapping::Poe1Mapping.map(&StatId::new(ONSLAUGHT_STAT))
        else {
            panic!("{ONSLAUGHT_STAT}");
        };
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].quantity, Quantity::Poe1(Poe1Quantity::Onslaught));
        assert_eq!(t[0].op, Op::Flag);
        assert_eq!(t[0].conditions, Conditions::none());
    }
}
