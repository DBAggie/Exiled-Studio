//! Power, frenzy, and endurance charges (Phase 3 plan, step 12; P3-12e):
//! the config's `use<Kind>Charges` and `override<Kind>Charges` inputs.
//!
//! What PoB v2.67.2 does:
//! - `usePowerCharges` and its siblings (`Modules/ConfigOptions.lua` 900
//!   to 917) set the `UsePowerCharges` flag in combat; the override inputs
//!   (count options, applied when not 0, or their placeholder) set
//!   `PowerCharges` OVERRIDE, read only inside that flag's branch;
//! - `doActorCharges` (`Modules/CalcPerform.lua` 980 to 1120): with the
//!   flag, the charges are the override or the maximum, the maximum the
//!   `PowerChargesMax` BASE sum (the character's `max_power_charges`,
//!   `Modules/CalcSetup.lua` 47 to 49, plus modifiers), and at least the
//!   minimum (0 without modifiers); then `Multiplier:PowerCharge` is the
//!   count;
//! - `calcs.initModDB` (`Modules/CalcSetup.lua` 625 to 630) gives each
//!   charge its effect per charge, from the character's constants:
//!   `CritChance` INC `critical_strike_chance_+%_per_power_charge`;
//!   attack `Speed` INC `base_attack_speed_+%_per_frenzy_charge`, cast
//!   `Speed` INC `base_cast_speed_+%_per_frenzy_charge`, `Damage` MORE
//!   `object_inherent_damage_+%_final_per_frenzy_charge`;
//!   `PhysicalDamageReduction` and `ElementalDamageReduction` BASE per
//!   endurance charge, which no slice field reads (damage taken).
//!
//! The engine has no multipliers: each effect is one effect valued per
//! charge times the count, from its `use` input. What else reads or
//! changes the counts (a different maximum or minimum, "at maximum"
//! flags, conversions to absorption, affliction, or brutal charges,
//! modifiers per charge) is a modifier, so a stat: no mapping reads or
//! changes a charge (a test in this module), so every such stat is a
//! reason (`unmapped-stat`) or a `NoEffect` row that reaches no slice
//! field.
//!
//! The endurance inputs change no computed field: like
//! [`super::config::NAMED_ONLY`], their only other effect is a multiplier
//! modifiers read.

use loom_domain::{ConfigInputKind, ConfigSet, ConfigValue};

use crate::condition::Flags;
use crate::ingest::Ingest;
use crate::input::CalcData;
use crate::quantity::{Op, Quantity};
use crate::source::Source;
use crate::state::{IndeterminateReason, UnsupportedReason};
use crate::store::{Conditions, Effect};

/// One kind of charge.
#[derive(Debug, Clone, Copy)]
pub struct Charge {
    /// The config input that uses the charges.
    pub use_input: &'static str,
    /// The config input that sets their count.
    pub override_input: &'static str,
    /// The character constant of their maximum.
    pub max_stat: &'static str,
    /// Each effect per charge: the character constant of its value, the
    /// quantity, the operation, and the flags a query needs.
    pub effects: &'static [(&'static str, Quantity, Op, Flags)],
}

/// The three charges, in PoB's order.
pub const CHARGES: &[Charge] = &[
    Charge {
        use_input: "usePowerCharges",
        override_input: "overridePowerCharges",
        max_stat: "max_power_charges",
        effects: &[(
            "critical_strike_chance_+%_per_power_charge",
            Quantity::CritChance,
            Op::Increased,
            Flags::NONE,
        )],
    },
    Charge {
        use_input: "useFrenzyCharges",
        override_input: "overrideFrenzyCharges",
        max_stat: "max_frenzy_charges",
        effects: &[
            (
                "base_attack_speed_+%_per_frenzy_charge",
                Quantity::Speed,
                Op::Increased,
                Flags::ATTACK,
            ),
            (
                "base_cast_speed_+%_per_frenzy_charge",
                Quantity::Speed,
                Op::Increased,
                Flags::CAST,
            ),
            (
                "object_inherent_damage_+%_final_per_frenzy_charge",
                Quantity::Damage,
                Op::More,
                Flags::NONE,
            ),
        ],
    },
    // Physical and elemental damage reduction only: no slice field.
    Charge {
        use_input: "useEnduranceCharges",
        override_input: "overrideEnduranceCharges",
        max_stat: "max_endurance_charges",
        effects: &[],
    },
];

/// Whether `input` is one of the charges' inputs, which [`charges`]
/// handles.
#[must_use]
pub fn is_charge_input(input: &str) -> bool {
    CHARGES
        .iter()
        .any(|c| c.use_input == input || c.override_input == input)
}

/// The last value of `name` of `kind` in `set` (PoB keeps inputs in a
/// table: the last one read counts).
fn last<'a>(set: &'a ConfigSet, name: &str, kind: ConfigInputKind) -> Option<&'a ConfigValue> {
    set.inputs
        .typed()
        .filter(|i| i.kind == kind && i.name == name)
        .last()
        .map(|i| &i.value)
}

/// What an override input gives: `Ok(None)` when unset (absent or 0, as
/// PoB's count options), `Err(())` for a value PoB would read otherwise
/// than a number, or a number that is not finite.
fn count(value: Option<&ConfigValue>) -> Result<Option<f64>, ()> {
    match value {
        None => Ok(None),
        // A non-finite number (a build document can carry one) is not a
        // count PoB could hold: a reason, never a silent 0.
        Some(ConfigValue::Number(n)) if !n.value().0.is_finite() => Err(()),
        Some(ConfigValue::Number(n)) => Ok(Some(n.value().0).filter(|n| *n != 0.0)),
        Some(ConfigValue::Boolean(_) | ConfigValue::String(_)) => Err(()),
    }
}

/// Adds the effects of the charges the active config set uses (module
/// docs). The effects' source is the `use` input, whichever input set
/// the count (an override or its placeholder): the `use` input is what
/// turns them on, and the count is part of the effect's value. A `use` input that is not a flag, or an override that is not a
/// number, is a reason under the `use` input; a missing constant is
/// indeterminate.
pub fn charges(set: Option<&ConfigSet>, data: &dyn CalcData, ingest: &mut Ingest<'_>) {
    let Some(set) = set else { return };
    for charge in CHARGES {
        let reason = || UnsupportedReason::UnmodelledConfig {
            input: charge.use_input.to_owned(),
        };
        match last(set, charge.use_input, ConfigInputKind::Input) {
            None | Some(ConfigValue::Boolean(false)) => continue,
            Some(ConfigValue::Boolean(true)) => {}
            Some(ConfigValue::Number(_) | ConfigValue::String(_)) => {
                ingest.reason(reason());
                continue;
            }
        }
        // No effect on a slice field (endurance): nothing to count.
        if charge.effects.is_empty() {
            continue;
        }
        // The input, else its placeholder (PoB's `BuildModList`).
        let overridden = match count(last(set, charge.override_input, ConfigInputKind::Input)) {
            Ok(None) => count(last(
                set,
                charge.override_input,
                ConfigInputKind::Placeholder,
            )),
            other => other,
        };
        let count = match overridden {
            Ok(Some(n)) => n,
            Ok(None) => match data.character_constant(charge.max_stat) {
                #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
                Some(max) => max as f64,
                None => {
                    ingest.indeterminate(IndeterminateReason::MissingData {
                        table: "character_constants",
                        key: charge.max_stat.to_owned(),
                    });
                    continue;
                }
            },
            Err(()) => {
                ingest.reason(reason());
                continue;
            }
        };
        // At least the minimum, 0 without modifiers.
        let count = count.max(0.0);
        for &(stat, quantity, op, flags) in charge.effects {
            match data.character_constant(stat) {
                #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
                Some(per) => ingest.effect(Effect {
                    quantity,
                    op,
                    value: per as f64 * count,
                    conditions: Conditions::none().with_all(flags),
                    source: Source::Config {
                        input: charge.use_input.to_owned(),
                    },
                }),
                None => ingest.indeterminate(IndeterminateReason::MissingData {
                    table: "character_constants",
                    key: stat.to_owned(),
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poe1::mapping::{HIT_ROWS, LOCAL_ROWS, ROWS, WEAPON_ROWS};

    #[test]
    fn charge_inputs_should_be_known_and_not_named_only() {
        for c in CHARGES {
            assert!(is_charge_input(c.use_input));
            assert!(is_charge_input(c.override_input));
            assert!(!super::super::config::named_only(c.use_input));
            assert!(!super::super::config::named_only(c.override_input));
        }
        assert!(!is_charge_input("useSiphoningCharges"));
    }

    /// A word of the stat ID is `charge` or `charges` (not `recharge`).
    fn names_a_charge(id: &str) -> bool {
        id.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|w| w == "charge" || w == "charges")
    }

    /// `NoEffect` rows that name a charge, each checked: none scales a
    /// slice field per charge or changes a count PoB's calc reads.
    const NO_EFFECT_CHARGE_ROWS: &[&str] = &[
        // Chances to gain a charge: PoB's calc never reads them.
        "add_power_charge_on_kill_%_chance",
        // Shock on you: not a slice field.
        "cannot_be_shocked_while_at_maximum_power_charges",
        "chance_to_gain_power_charge_when_block_%",
        "gain_power_charge_for_each_second_channeling_spell",
        // Regeneration per charge: not a slice field.
        "life_regeneration_rate_per_minute_%_per_endurance_charge",
        "mana_regeneration_rate_+%_per_power_charge",
    ];

    /// The module's argument: no mapped stat reads or changes a charge,
    /// so every stat that does is a reason or reaches no slice field. The
    /// maxima themselves stay `Unmapped` (asserted here through every
    /// mapping, and in `mapping`'s tests).
    #[test]
    fn no_mapped_stat_should_read_or_change_a_charge() {
        use crate::ingest::{StatMap, StatMapping};
        use crate::poe1::ArmourSlot;
        use crate::poe1::mapping::{
            NO_EFFECT, PieceMapping, Poe1Mapping, SkillMapping, WeaponMapping,
        };
        use loom_domain::StatId;

        let ids = HIT_ROWS
            .iter()
            .map(|r| r.0)
            .chain(ROWS.iter().map(|r| r.0))
            .chain(LOCAL_ROWS.iter().map(|r| r.0))
            .chain(WEAPON_ROWS.iter().map(|r| r.0));
        for id in ids {
            assert!(!names_a_charge(id), "{id}: revisit poe1::charges");
        }
        for (id, _) in NO_EFFECT {
            assert!(
                !names_a_charge(id) || NO_EFFECT_CHARGE_ROWS.contains(id),
                "{id}: a NoEffect row naming a charge; check it and list it"
            );
        }
        assert!(!names_a_charge("energy_shield_recharge_rate_+%"));
        let piece = PieceMapping {
            slot: ArmourSlot::Helmet,
        };
        let mappings: [&dyn StatMapping; 4] = [&Poe1Mapping, &SkillMapping, &piece, &WeaponMapping];
        for c in CHARGES {
            for m in mappings {
                assert_eq!(
                    m.map(&StatId::new(c.max_stat)),
                    StatMap::Unmapped,
                    "{}",
                    c.max_stat
                );
            }
        }
    }
}
