//! Stacks and config multipliers (Phase 3 plan, step 8; P3-8n): rage,
//! Inspiration charges, Momentum, Intensity, and the non-instant spells
//! cast recently.
//!
//! PoB v2.67.2 reads each as a multiplier (`Multiplier:<Name>`) that
//! modifiers per stack read; `ModStore` scales such a modifier by
//! `floor(multiplier + 0.0001)`, at most its limit (`Classes/ModStore.lua`
//! 326 to 375). The engine has no multipliers: [`stacks`] counts each kind
//! as PoB does, after every other source, and turns each modifier per
//! stack ([`StackValue::Per`]) into its target times the count, with the
//! modifier's source, as [`super::charges`] does.
//!
//! - **Rage** (`Modules/CalcPerform.lua` 881 to 897): only with
//!   `Condition:CanGainRage` (here [`StackValue::CanGain`], "Gain # Rage on
//!   Melee Hit", `ModParser.lua` 2533; `RageRegen` stats are unmapped, so
//!   reasons). The maximum is `floor(MaximumRage)`, the character's
//!   `maximum_rage` plus "+# to Maximum Rage" (no `MORE` mapped); the
//!   stacks `max(min(Multiplier:RageStack, maximum), 0)`, the config's
//!   `multiplierRage` (a count, `ConfigOptions.lua` 1062, conditioned on
//!   `CanGainRage`; no minimum rage mapped); the effect `floor(stacks x
//!   RageEffect)` (no rage effect mapped) as attack `Damage` MORE.
//!   "Nearby Enemies are Intimidated while you have Rage" makes the enemy
//!   Intimidated while `Multiplier:RageStack` is at least 1 (5031; the
//!   effect is [`super::damage_taken`]'s).
//! - **Inspiration charges** (`CalcPerform.lua` 972, 1061, 1095): the
//!   config's `overrideInspirationCharges` (`countAllowZero`), else the
//!   maximum, the character's `maximum_righteous_charges` (no modifier
//!   mapped). Inspiration Support's elemental damage MORE and crit chance
//!   INC per charge (`Data/Skills/sup_str.lua` 4614 to 4619).
//! - **Momentum** (`CalcOffence.lua` 1005 to 1015): with a maximum above 0
//!   (Momentum Support's `MomentumStacksMax`), the config's
//!   `MomentumStacks` (a count) at most the maximum, else `min((maximum +
//!   extra) / 2, maximum)`, the extra `MomentumStacksExtra`; attack
//!   `Speed` INC per stack (`sup_dex.lua` 3123 to 3131).
//! - **Intensity** (`ConfigOptions.lua` 503; `CalcSetup.lua` 522): the
//!   config's `intensifyIntensity` (a count), at most
//!   `Multiplier:IntensityLimit`, 3 plus "+# to maximum Intensity"
//!   (`ModParser.lua` 5572); Intensify's area `Damage` MORE per intensity
//!   (`sup_int.lua` 4040).
//! - **Non-instant spells cast recently** (`ConfigOptions.lua` 1493): the
//!   config's count; "#% increased Cast Speed for each different
//!   Non-Instant Spell you've Cast Recently" (`ModParser.lua` 1850).
//!
//! A config value PoB would read otherwise than a number is a reason under
//! its input. Other modifiers that read or change these counts (rage
//! effect, minimum rage, maximum Inspiration charges, rage on other hits)
//! are unmapped, so reasons.

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

/// A kind of stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StackKind {
    /// Rage.
    Rage,
    /// Inspiration charges.
    Inspiration,
    /// Momentum.
    Momentum,
    /// Intensity.
    Intensity,
    /// Different non-instant spells cast recently.
    NonInstantSpells,
}

impl StackKind {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Rage => "rage",
            Self::Inspiration => "inspiration",
            Self::Momentum => "momentum",
            Self::Intensity => "intensity",
            Self::NonInstantSpells => "non-instant-spells",
        }
    }
}

/// What a stack modifier changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StackValue {
    /// The maximum (or Intensity's limit): `Base`.
    Maximum,
    /// Momentum counted as more than it is (`MomentumStacksExtra`): `Base`.
    Extra,
    /// The stacks can be gained (rage's `Condition:CanGainRage`): a `Flag`.
    CanGain,
    /// The enemy is Intimidated while you have rage: a `Flag`.
    Intimidate,
    /// An effect per stack.
    Per(PerStack),
}

impl StackValue {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Maximum => "maximum",
            Self::Extra => "extra",
            Self::CanGain => "can-gain",
            Self::Intimidate => "intimidate",
            Self::Per(p) => p.name(),
        }
    }
}

/// An effect per stack: its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PerStack {
    /// Increased attack speed (`Speed` INC, `ModFlag.Attack`).
    AttackSpeed,
    /// Increased cast speed (`Speed` INC, `ModFlag.Cast`).
    CastSpeed,
    /// Increased critical strike chance (`CritChance` INC).
    CritChance,
    /// More elemental damage (`ElementalDamage` MORE).
    ElementalDamageMore,
    /// More area damage (`Damage` MORE, `ModFlag.Area`).
    AreaDamageMore,
}

impl PerStack {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::AttackSpeed => "per.attack-speed",
            Self::CastSpeed => "per.cast-speed",
            Self::CritChance => "per.crit-chance",
            Self::ElementalDamageMore => "per.elemental-damage-more",
            Self::AreaDamageMore => "per.area-damage-more",
        }
    }

    /// The effect one stack has: quantity, operation, and the flags a
    /// query needs (all of `.2`, one of `.3`).
    #[must_use]
    pub const fn target(self) -> (Quantity, Op, Flags, Flags) {
        match self {
            Self::AttackSpeed => (Quantity::Speed, Op::Increased, Flags::ATTACK, Flags::NONE),
            Self::CastSpeed => (Quantity::Speed, Op::Increased, Flags::CAST, Flags::NONE),
            Self::CritChance => (
                Quantity::CritChance,
                Op::Increased,
                Flags::NONE,
                Flags::NONE,
            ),
            Self::ElementalDamageMore => {
                (Quantity::Damage, Op::More, Flags::NONE, Flags::ELEMENTAL)
            }
            Self::AreaDamageMore => (Quantity::Damage, Op::More, Flags::AREA, Flags::NONE),
        }
    }
}

/// The stack modifiers the engine models (module docs), sorted by stat
/// ID, each as PoB v2.67.2 reads its text or skill stat.
pub const MODIFIERS: &[(&str, StackKind, StackValue)] = &[
    (
        "cast_speed_+%_per_num_unique_spells_cast_recently",
        StackKind::NonInstantSpells,
        StackValue::Per(PerStack::CastSpeed),
    ),
    (
        "count_as_momentum_+_if_not_channelled",
        StackKind::Momentum,
        StackValue::Extra,
    ),
    (
        "critical_strike_chance_+%_per_righteous_charge",
        StackKind::Inspiration,
        StackValue::Per(PerStack::CritChance),
    ),
    (
        "elemental_damage_+%_final_per_righteous_charge",
        StackKind::Inspiration,
        StackValue::Per(PerStack::ElementalDamageMore),
    ),
    (
        "gain_x_rage_on_melee_hit",
        StackKind::Rage,
        StackValue::CanGain,
    ),
    (
        "maximum_intensify_stacks",
        StackKind::Intensity,
        StackValue::Maximum,
    ),
    ("maximum_rage", StackKind::Rage, StackValue::Maximum),
    (
        "nearby_enemies_are_intimidated_while_you_have_rage",
        StackKind::Rage,
        StackValue::Intimidate,
    ),
    (
        "support_momentum_attack_speed_+%_per_stack",
        StackKind::Momentum,
        StackValue::Per(PerStack::AttackSpeed),
    ),
    (
        "support_momentum_max_stacks",
        StackKind::Momentum,
        StackValue::Maximum,
    ),
    (
        "support_spell_boost_area_damage_+%_final_per_charge",
        StackKind::Intensity,
        StackValue::Per(PerStack::AreaDamageMore),
    ),
];

/// The stack modifier `stat` is, if any ([`MODIFIERS`]).
#[must_use]
pub fn modifier(stat: &str) -> Option<(StackKind, StackValue)> {
    MODIFIERS
        .binary_search_by(|(id, _, _)| (*id).cmp(stat))
        .ok()
        .map(|i| (MODIFIERS[i].1, MODIFIERS[i].2))
}

/// The config's rage input.
pub const RAGE_INPUT: &str = "multiplierRage";
/// The config's Inspiration charge input.
pub const INSPIRATION_INPUT: &str = "overrideInspirationCharges";
/// The config's Momentum input.
pub const MOMENTUM_INPUT: &str = "MomentumStacks";
/// The config's Intensity input.
pub const INTENSITY_INPUT: &str = "intensifyIntensity";
/// The config's non-instant spells input.
pub const SPELLS_INPUT: &str = "multiplierNonInstantSpellCastRecently";

/// The inputs this module reads.
pub const INPUTS: [&str; 5] = [
    RAGE_INPUT,
    INSPIRATION_INPUT,
    MOMENTUM_INPUT,
    INTENSITY_INPUT,
    SPELLS_INPUT,
];

/// Whether `input` is one this module reads.
#[must_use]
pub fn is_input(input: &str) -> bool {
    INPUTS.contains(&input)
}

/// A count option (`BuildModList`: the input, else the placeholder; a
/// count applies when not 0, a `countAllowZero` also when 0): `Ok(None)`
/// when it does not apply, `Err(())` for a value that is not a finite
/// number.
fn count(set: Option<&ConfigSet>, name: &str, allow_zero: bool) -> Result<Option<f64>, ()> {
    let Some(set) = set else { return Ok(None) };
    let read = |kind| match last(set, name, kind) {
        None => Ok(None),
        Some(ConfigValue::Number(n)) if n.value().0.is_finite() => {
            Ok(Some(n.value().0).filter(|v| allow_zero || *v != 0.0))
        }
        Some(_) => Err(()),
    };
    match read(ConfigInputKind::Input)? {
        Some(v) => Ok(Some(v)),
        None => read(ConfigInputKind::Placeholder),
    }
}

/// The `Base` sum of the unconditional effects on `Stack(kind, value)` so
/// far, and the first one's source. A flag counts when the sum is not 0:
/// rage's flags have a description that shows them only for a value of at
/// least 1. A flag whose description shows it for any value (the import
/// then assumes 0, as Convergence's, `poe1::buffs`) must be read by
/// presence instead.
fn sum(ingest: &Ingest<'_>, kind: StackKind, value: StackValue) -> (f64, Option<Source>) {
    let mut total = 0.0;
    let mut first = None;
    for e in ingest.effects() {
        if e.quantity == Quantity::Poe1(Poe1Quantity::Stack(kind, value))
            && matches!(e.op, Op::Base | Op::Flag)
            && e.conditions == Conditions::none()
        {
            total += e.value;
            first.get_or_insert_with(|| e.source.clone());
        }
    }
    (total, first)
}

/// PoB's multiplier: `floor(value + 0.0001)`.
fn multiplier(value: f64) -> f64 {
    (value + 0.0001).floor()
}

/// The rage stack the config sets (`Multiplier:RageStack`): the input when
/// rage can be gained, else 0. `Err` for an input of another type.
fn rage_stack(set: Option<&ConfigSet>, ingest: &Ingest<'_>) -> Result<f64, ()> {
    if sum(ingest, StackKind::Rage, StackValue::CanGain).0 == 0.0 {
        return Ok(0.0);
    }
    Ok(count(set, RAGE_INPUT, false)?.unwrap_or(0.0))
}

/// The source that makes the enemy Intimidated through rage, if any
/// (module docs): read by [`super::damage_taken`].
#[must_use]
pub fn intimidates(set: Option<&ConfigSet>, ingest: &Ingest<'_>) -> Option<Source> {
    let (flag, source) = sum(ingest, StackKind::Rage, StackValue::Intimidate);
    if flag == 0.0 {
        return None;
    }
    // A rage input of another type is a reason of [`stacks`].
    (rage_stack(set, ingest).ok()? >= 1.0)
        .then_some(source)
        .flatten()
}

/// Adds the effects of the stacks (module docs): after every other source.
pub fn stacks(set: Option<&ConfigSet>, data: &dyn CalcData, ingest: &mut Ingest<'_>) {
    let reason = |input: &str| UnsupportedReason::UnmodelledConfig {
        input: input.to_owned(),
    };
    let config = |input: &str| Source::Config {
        input: input.to_owned(),
    };
    let per = |ingest: &Ingest<'_>, kind: StackKind| -> Vec<Effect> {
        ingest
            .effects()
            .iter()
            .filter(|e| {
                matches!(
                    e.quantity,
                    Quantity::Poe1(Poe1Quantity::Stack(k, StackValue::Per(_))) if k == kind
                ) && e.op == Op::Base
                    && e.conditions == Conditions::none()
            })
            .cloned()
            .collect()
    };
    let constant = |ingest: &mut Ingest<'_>, stat: &str| -> Option<f64> {
        match data.character_constant(stat) {
            #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
            Some(v) => Some(v as f64),
            None => {
                ingest.indeterminate(IndeterminateReason::MissingData {
                    table: "character_constants",
                    key: stat.to_owned(),
                });
                None
            }
        }
    };

    // Rage.
    if sum(ingest, StackKind::Rage, StackValue::CanGain).0 != 0.0 {
        match rage_stack(set, ingest) {
            Err(()) => ingest.reason(reason(RAGE_INPUT)),
            Ok(stack) => {
                if let Some(base) = constant(ingest, "maximum_rage") {
                    let max = (base + sum(ingest, StackKind::Rage, StackValue::Maximum).0).floor();
                    let stacks = stack.min(max).max(0.0);
                    let effect = stacks.floor();
                    if effect != 0.0 {
                        ingest.effect(Effect {
                            quantity: Quantity::Damage,
                            op: Op::More,
                            value: effect,
                            conditions: Conditions::none().with_all(Flags::ATTACK),
                            source: config(RAGE_INPUT),
                        });
                    }
                }
            }
        }
    } else if sum(ingest, StackKind::Rage, StackValue::Intimidate).0 != 0.0 {
        // Without `CanGainRage` the config's stack does not apply; an input
        // of another type is still a reason.
        if count(set, RAGE_INPUT, false).is_err() {
            ingest.reason(reason(RAGE_INPUT));
        }
    }

    // Inspiration charges.
    let effects = per(ingest, StackKind::Inspiration);
    if !effects.is_empty() {
        match count(set, INSPIRATION_INPUT, true) {
            Err(()) => ingest.reason(reason(INSPIRATION_INPUT)),
            Ok(overridden) => {
                let n = match overridden {
                    Some(n) => Some(n),
                    None => constant(ingest, "maximum_righteous_charges").map(|b| {
                        (b + sum(ingest, StackKind::Inspiration, StackValue::Maximum).0).max(0.0)
                    }),
                };
                if let Some(n) = n {
                    apply(ingest, &effects, multiplier(n), None);
                }
            }
        }
    }

    // Momentum.
    let effects = per(ingest, StackKind::Momentum);
    let (max, _) = sum(ingest, StackKind::Momentum, StackValue::Maximum);
    if !effects.is_empty() && max > 0.0 {
        match count(set, MOMENTUM_INPUT, false) {
            Err(()) => ingest.reason(reason(MOMENTUM_INPUT)),
            Ok(configured) => {
                let (extra, _) = sum(ingest, StackKind::Momentum, StackValue::Extra);
                let n = configured.map_or(((max + extra) / 2.0).min(max), |c| c.min(max));
                apply(ingest, &effects, multiplier(n), None);
            }
        }
    }

    // Intensity.
    let effects = per(ingest, StackKind::Intensity);
    if !effects.is_empty() {
        match count(set, INTENSITY_INPUT, false) {
            Err(()) => ingest.reason(reason(INTENSITY_INPUT)),
            Ok(None) => {}
            Ok(Some(n)) => {
                let limit = 3.0 + sum(ingest, StackKind::Intensity, StackValue::Maximum).0;
                apply(ingest, &effects, multiplier(n).min(limit), None);
            }
        }
    }

    // Non-instant spells cast recently.
    let effects = per(ingest, StackKind::NonInstantSpells);
    if !effects.is_empty() {
        match count(set, SPELLS_INPUT, false) {
            Err(()) => ingest.reason(reason(SPELLS_INPUT)),
            Ok(None) => {}
            Ok(Some(n)) => apply(ingest, &effects, multiplier(n), None),
        }
    }
}

/// Each effect per stack as its target times `count`, with its source (or
/// `source` when given).
fn apply(ingest: &mut Ingest<'_>, effects: &[Effect], count: f64, source: Option<&Source>) {
    if count == 0.0 {
        return;
    }
    for e in effects {
        let Quantity::Poe1(Poe1Quantity::Stack(_, StackValue::Per(p))) = e.quantity else {
            continue;
        };
        let (quantity, op, all, any) = p.target();
        ingest.effect(Effect {
            quantity,
            op,
            value: e.value * count,
            conditions: Conditions::none().with_all(all).with_any(any),
            source: source.cloned().unwrap_or_else(|| e.source.clone()),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_modifiers_should_be_sorted_and_the_inputs_not_named_only() {
        assert!(MODIFIERS.windows(2).all(|w| w[0].0 < w[1].0));
        for input in INPUTS {
            assert!(is_input(input));
            assert!(!super::super::config::named_only(input), "{input}");
        }
        assert_eq!(
            modifier("maximum_rage"),
            Some((StackKind::Rage, StackValue::Maximum))
        );
    }

    #[test]
    fn a_multiplier_should_floor_as_pob() {
        assert_eq!(multiplier(2.5), 2.0);
        assert_eq!(multiplier(2.99995), 3.0);
        assert_eq!(multiplier(0.0), 0.0);
    }
}
