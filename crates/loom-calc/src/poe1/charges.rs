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
//! charge times the count. The count is PoB's (P3-8d):
//! `max(use ? (override or maximum) : 0, min(maximum, minimum))`, the
//! maximum the character constant plus the modifiers' (at least 0), the
//! minimum the modifiers' (at least 0). So a minimum gives charges without
//! the `use` input, as in PoB.
//!
//! **Modifiers** ([`MODIFIERS`], P3-8d): the stats of the maximum, the
//! minimum, and effects per charge map to [`Poe1Quantity::Charge`]
//! (`Base`, unconditional); [`charges`] reads them after every other
//! source and turns each per-charge effect into its target
//! ([`PerCharge::target`]) times the count, with the modifier's source.
//! What else reads or changes the counts ("at maximum" flags, conversions
//! to absorption, affliction, or brutal charges, "maximum frenzy charges is
//! maximum power charges", per-charge effects not in the table) is a stat
//! the mapping does not know, so a reason (`unmapped-stat`), or a
//! `NoEffect` row that reaches no slice field (a test in this module).
//!
//! The endurance charges' own effects change no computed field: like
//! [`super::config::NAMED_ONLY`], their only other effect is a multiplier
//! modifiers read; a per-endurance-charge modifier in [`MODIFIERS`] is
//! counted.

use loom_domain::{ConfigInputKind, ConfigSet, ConfigValue};

use super::Poe1Quantity;
use super::config::last_input as last;
use crate::condition::Flags;
use crate::ingest::Ingest;
use crate::input::CalcData;
use crate::quantity::{Op, Quantity};
use crate::source::Source;
use crate::state::{IndeterminateReason, UnsupportedReason};
use crate::store::{Conditions, Effect};

/// A kind of charge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChargeKind {
    /// Power charges.
    Power,
    /// Frenzy charges.
    Frenzy,
    /// Endurance charges.
    Endurance,
}

impl ChargeKind {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Power => "power",
            Self::Frenzy => "frenzy",
            Self::Endurance => "endurance",
        }
    }
}

/// What a charge modifier changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChargeValue {
    /// The maximum (PoB's `<Kind>ChargesMax` BASE).
    Maximum,
    /// The minimum (PoB's `<Kind>ChargesMin` BASE).
    Minimum,
    /// An effect per charge (a mod with PoB's `Multiplier:<Kind>Charge`
    /// tag).
    Per(PerCharge),
}

impl ChargeValue {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Maximum => "maximum",
            Self::Minimum => "minimum",
            Self::Per(p) => p.name(),
        }
    }
}

/// An effect per charge: its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PerCharge {
    /// Increased damage (`Damage` INC).
    Damage,
    /// Increased spell damage (`Damage` INC with `ModFlag.Spell`).
    SpellDamage,
    /// Increased critical strike chance (`CritChance` INC).
    CritChance,
    /// Critical strike multiplier (`CritMultiplier` BASE).
    CritMultiplier,
}

impl PerCharge {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Damage => "per.damage",
            Self::SpellDamage => "per.spell-damage",
            Self::CritChance => "per.crit-chance",
            Self::CritMultiplier => "per.crit-multiplier",
        }
    }

    /// The effect one charge has: quantity, operation, and the flags a
    /// query needs.
    #[must_use]
    pub const fn target(self) -> (Quantity, Op, Flags) {
        match self {
            Self::Damage => (Quantity::Damage, Op::Increased, Flags::NONE),
            Self::SpellDamage => (Quantity::Damage, Op::Increased, Flags::SPELL),
            Self::CritChance => (Quantity::CritChance, Op::Increased, Flags::NONE),
            Self::CritMultiplier => (Quantity::CritMultiplier, Op::Base, Flags::NONE),
        }
    }
}

/// The charge modifiers the engine models (module docs), sorted by stat
/// ID, each checked against PoB v2.67.2's reading of its game text
/// (`Modules/ModParser.lua`: "maximum power charges" `PowerChargesMax`
/// BASE, "minimum power charges" `PowerChargesMin` BASE, and the
/// `per power charge` tag `{ type = "Multiplier", var = "PowerCharge" }`
/// on `Damage` INC, spell `Damage` INC, `CritChance` INC, and
/// `CritMultiplier` BASE; the frenzy and endurance stats alike).
pub const MODIFIERS: &[(&str, ChargeKind, ChargeValue)] = &[
    (
        "base_minimum_endurance_charges",
        ChargeKind::Endurance,
        ChargeValue::Minimum,
    ),
    (
        "base_minimum_frenzy_charges",
        ChargeKind::Frenzy,
        ChargeValue::Minimum,
    ),
    (
        "base_minimum_power_charges",
        ChargeKind::Power,
        ChargeValue::Minimum,
    ),
    (
        "critical_strike_chance_+%_per_power_charge",
        ChargeKind::Power,
        ChargeValue::Per(PerCharge::CritChance),
    ),
    (
        "critical_strike_multiplier_+_per_power_charge",
        ChargeKind::Power,
        ChargeValue::Per(PerCharge::CritMultiplier),
    ),
    (
        "damage_+%_per_endurance_charge",
        ChargeKind::Endurance,
        ChargeValue::Per(PerCharge::Damage),
    ),
    (
        "damage_+%_per_frenzy_charge",
        ChargeKind::Frenzy,
        ChargeValue::Per(PerCharge::Damage),
    ),
    (
        "damage_+%_per_power_charge",
        ChargeKind::Power,
        ChargeValue::Per(PerCharge::Damage),
    ),
    (
        "max_endurance_charges",
        ChargeKind::Endurance,
        ChargeValue::Maximum,
    ),
    (
        "max_frenzy_charges",
        ChargeKind::Frenzy,
        ChargeValue::Maximum,
    ),
    ("max_power_charges", ChargeKind::Power, ChargeValue::Maximum),
    (
        "spell_damage_+%_per_power_charge",
        ChargeKind::Power,
        ChargeValue::Per(PerCharge::SpellDamage),
    ),
];

/// The charge modifier `stat` is, if any ([`MODIFIERS`]).
#[must_use]
pub fn modifier(stat: &str) -> Option<(ChargeKind, ChargeValue)> {
    MODIFIERS
        .binary_search_by(|(id, _, _)| (*id).cmp(stat))
        .ok()
        .map(|i| (MODIFIERS[i].1, MODIFIERS[i].2))
}

/// One kind of charge.
#[derive(Debug, Clone, Copy)]
pub struct Charge {
    /// The kind.
    pub kind: ChargeKind,
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
        kind: ChargeKind::Power,
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
        kind: ChargeKind::Frenzy,
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
        kind: ChargeKind::Endurance,
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

/// The `Base` sum of the unconditional effects on `Charge(kind, value)`
/// so far, and the first one's source.
fn modifiers(ingest: &Ingest<'_>, kind: ChargeKind, value: ChargeValue) -> (f64, Option<Source>) {
    let mut sum = 0.0;
    let mut first = None;
    for e in ingest.effects() {
        if e.quantity == Quantity::Poe1(Poe1Quantity::Charge(kind, value))
            && e.op == Op::Base
            && e.conditions == Conditions::none()
        {
            sum += e.value;
            first.get_or_insert_with(|| e.source.clone());
        }
    }
    (sum, first)
}

/// Adds the effects of the charges (module docs): from the active config
/// set's `use` and override inputs and the charge modifiers ingested so
/// far, so it runs after every other source. The constant effects' source
/// is the `use` input when it is on (whichever input set the count),
/// else the first minimum modifier's; each per-charge modifier keeps its
/// own. A `use` input that is not a flag, or an override that is not a
/// number, is a reason under the `use` input; a missing constant is
/// indeterminate.
pub fn charges(set: Option<&ConfigSet>, data: &dyn CalcData, ingest: &mut Ingest<'_>) {
    for charge in CHARGES {
        let reason = || UnsupportedReason::UnmodelledConfig {
            input: charge.use_input.to_owned(),
        };
        let use_on = match set.and_then(|s| last(s, charge.use_input, ConfigInputKind::Input)) {
            None | Some(ConfigValue::Boolean(false)) => false,
            Some(ConfigValue::Boolean(true)) => true,
            Some(ConfigValue::Number(_) | ConfigValue::String(_)) => {
                ingest.reason(reason());
                continue;
            }
        };
        let per: Vec<Effect> = ingest
            .effects()
            .iter()
            .filter(|e| {
                matches!(
                    e.quantity,
                    Quantity::Poe1(Poe1Quantity::Charge(k, ChargeValue::Per(_))) if k == charge.kind
                ) && e.op == Op::Base
                    && e.conditions == Conditions::none()
            })
            .cloned()
            .collect();
        // No effect on a slice field: nothing to count.
        if charge.effects.is_empty() && per.is_empty() {
            continue;
        }
        let (min, min_source) = modifiers(ingest, charge.kind, ChargeValue::Minimum);
        let min = min.max(0.0);
        if !use_on && min == 0.0 {
            continue;
        }
        let (more, _) = modifiers(ingest, charge.kind, ChargeValue::Maximum);
        // The input, else its placeholder (PoB's `BuildModList`); read only
        // when the charges are used, as PoB does.
        let overridden = if use_on {
            match count(set.and_then(|s| last(s, charge.override_input, ConfigInputKind::Input))) {
                Ok(None) => count(
                    set.and_then(|s| last(s, charge.override_input, ConfigInputKind::Placeholder)),
                ),
                other => other,
            }
        } else {
            Ok(None)
        };
        let overridden = match overridden {
            Ok(n) => n,
            Err(()) => {
                ingest.reason(reason());
                continue;
            }
        };
        let max = match data.character_constant(charge.max_stat) {
            #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
            Some(max) => (max as f64 + more).max(0.0),
            None => {
                ingest.indeterminate(IndeterminateReason::MissingData {
                    table: "character_constants",
                    key: charge.max_stat.to_owned(),
                });
                continue;
            }
        };
        let used = if use_on {
            overridden.unwrap_or(max)
        } else {
            0.0
        };
        let count = used.max(min.min(max));
        if count == 0.0 {
            continue;
        }
        let source = if use_on {
            Source::Config {
                input: charge.use_input.to_owned(),
            }
        } else if let Some(s) = min_source {
            s
        } else {
            continue;
        };
        for &(stat, quantity, op, flags) in charge.effects {
            match data.character_constant(stat) {
                #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
                Some(per) => ingest.effect(Effect {
                    quantity,
                    op,
                    value: per as f64 * count,
                    conditions: Conditions::none().with_all(flags),
                    source: source.clone(),
                }),
                None => ingest.indeterminate(IndeterminateReason::MissingData {
                    table: "character_constants",
                    key: stat.to_owned(),
                }),
            }
        }
        for e in per {
            let Quantity::Poe1(Poe1Quantity::Charge(_, ChargeValue::Per(p))) = e.quantity else {
                continue;
            };
            let (quantity, op, flags) = p.target();
            ingest.effect(Effect {
                quantity,
                op,
                value: e.value * count,
                conditions: Conditions::none().with_all(flags),
                source: e.source,
            });
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
        // Flask charges (Ryslatha's soul, P3-12g): not the power, frenzy,
        // or endurance charges this module counts.
        "life_flasks_gain_X_charges_every_3_seconds_if_you_have_not_used_a_life_flask_recently",
    ];

    /// The module's argument: no row of the mapping's tables reads or
    /// changes a charge, so every stat that does is a reason, reaches no
    /// slice field, or is one of [`MODIFIERS`], which map only to
    /// [`Poe1Quantity::Charge`] (P3-8d).
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
        // The charge modifiers map only to their own quantity, read here,
        // and no other table holds one (the mapping asks them first).
        assert!(MODIFIERS.windows(2).all(|w| w[0].0 < w[1].0));
        for (id, _, _) in MODIFIERS {
            let elsewhere = HIT_ROWS
                .iter()
                .map(|r| r.0)
                .chain(ROWS.iter().map(|r| r.0))
                .chain(NO_EFFECT.iter().map(|r| r.0))
                .any(|other| other == *id);
            assert!(!elsewhere, "{id}");
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
        // The maxima map only to their charge quantity.
        for c in CHARGES {
            let StatMap::Effects(t) = Poe1Mapping.map(&StatId::new(c.max_stat)) else {
                panic!("{}", c.max_stat);
            };
            assert_eq!(
                t.iter().map(|t| t.quantity).collect::<Vec<_>>(),
                [Quantity::Poe1(Poe1Quantity::Charge(
                    c.kind,
                    ChargeValue::Maximum
                ))]
            );
            // A gem's stats never set a maximum; an item's lines hand it
            // to the game mapping.
            assert_eq!(
                SkillMapping.map(&StatId::new(c.max_stat)),
                StatMap::Unmapped
            );
            for m in [&piece as &dyn StatMapping, &WeaponMapping] {
                assert_eq!(
                    m.map(&StatId::new(c.max_stat)),
                    Poe1Mapping.map(&StatId::new(c.max_stat)),
                    "{}",
                    c.max_stat
                );
            }
        }
    }
}
