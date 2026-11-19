//! Repeats (P6-11aj): PoB v2.67.2's `output.Repeats` and its repeat
//! modifiers (`Modules/CalcOffence.lua` 774 to 870), for the main skill's
//! hit, before its damage.
//!
//! What PoB does:
//! - **The count:** `Repeats = 1 + RepeatCount` BASE in the skill's config
//!   (the main hand's weapon flags for an attack), when the skill is an
//!   attack or a spell and none of `Instant`, `Channel`, `Triggered`,
//!   `Retaliation`, or `NonRepeatable` (nor `CannotRepeat`, whose stat is
//!   unmapped). Multistrike's count needs a melee weapon or no weapon
//!   (`ModFlagOr`), Spell Echo's a cast.
//! - **The config's `repeatMode`** (`ConfigOptions.lua` 940: `AVERAGE` by
//!   default, every new config set has it, `ConfigTab.lua` 1237): with more
//!   than one repeat and a mode other than `NONE`:
//!   - per-repeat damage (`RepeatPerRepeatDamage` MORE, Greater Spell
//!     Echo's): `Damage` MORE `value x (Repeats - 1) / 2`, per mod;
//!   - the final repeat's double damage chance (`RepeatFinalDoubleDamageChance`
//!     BASE, Awakened Spell Echo's): `DoubleDamageChance` BASE `value /
//!     Repeats`, per mod;
//!   - each repeat's own damage (`RepeatOneDamage` to `RepeatThreeDamage`
//!     MORE, Multistrike's), for the repeats the count reaches: one
//!     `Damage` MORE of their sum over `Repeats`, with the last mod's
//!     source and tags.
//! - **The rate:** each repeat is a use: the cooldown's and the server
//!   tick's caps are times `Repeats` (2359, 2367), and the costs per
//!   second divide by it (5803 to 5819).
//!
//! The derived effects go into the actor's store as PoB's `NewMod` adds
//! them to the skill's mod list (the damage as
//! [`Poe1Quantity::RepeatAverageDamage`], which the hit multiplies in
//! apart from the other `Damage` MORE, as PoB's results show), and [`Poe1Quantity::Repeats`] records the
//! count for the rate and the costs. Not modelled, each a reason: the
//! `FINAL` and `FINAL_DPS` modes (`unmodelled-config`), a final repeat's
//! own damage or area (no stat maps to them), and a trap's or a mine's
//! repeats (their skills are `Partial` already).

use std::collections::BTreeSet;

use super::Poe1Quantity;
use crate::condition::{Applies, QueryContext};
use crate::engine::Calc;
use crate::quantity::{Op, Quantity};
use crate::source::Source;
use crate::state::{Reasons, UnsupportedReason};
use crate::store::{Conditions, Effect};

/// The config list of the repeat mode.
pub const MODE_INPUT: &str = "repeatMode";

/// The source of the recorded count.
const COUNT_SOURCE: Source = Source::Rule("repeats");

/// Skill types that cannot repeat (`CalcOffence.lua` 775).
const NOT_REPEATABLE: [&str; 5] = [
    "Instant",
    "Channel",
    "Triggered",
    "Retaliation",
    "NonRepeatable",
];

/// The skill's repeat count as the hit recorded it: 1 when nothing did.
#[must_use]
pub fn count(calc: &Calc<'_>) -> f64 {
    let q = Quantity::Poe1(Poe1Quantity::Repeats);
    calc.store()
        .effects()
        .iter()
        .find(|e| e.quantity == q)
        .map_or(1.0, |e| e.value)
}

/// The effects of `quantity` and `op` that apply in `cx`; a condition the
/// engine cannot settle is a reason, its effect left out.
fn applying(
    calc: &Calc<'_>,
    quantity: Quantity,
    op: Op,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Vec<Effect> {
    let mut out = Vec::new();
    for e in calc.store().effects() {
        if e.quantity != quantity || e.op != op {
            continue;
        }
        match e.conditions.applies(cx) {
            Applies::Yes => out.push(e.clone()),
            Applies::No => {}
            Applies::Unknown(keys) => {
                for condition in keys {
                    reasons
                        .partial
                        .insert(UnsupportedReason::UnmodelledCondition {
                            condition,
                            quantity,
                            source: e.source.clone(),
                        });
                }
            }
        }
    }
    out
}

/// Settles the skill's repeats (module docs) once per actor: adds the
/// derived effects and the recorded count, the repeat reasons into
/// `reasons`.
pub fn apply(
    calc: &mut Calc<'_>,
    types: &BTreeSet<String>,
    cx: &QueryContext,
    reasons: &mut Reasons,
) {
    let recorded = Quantity::Poe1(Poe1Quantity::Repeats);
    if calc
        .store()
        .effects()
        .iter()
        .any(|e| e.quantity == recorded)
    {
        return;
    }
    let repeatable = (types.contains("Attack") || types.contains("Spell"))
        && !NOT_REPEATABLE.iter().any(|t| types.contains(*t));
    let extra: f64 = if repeatable {
        applying(
            calc,
            Quantity::Poe1(Poe1Quantity::RepeatCount),
            Op::Base,
            cx,
            reasons,
        )
        .iter()
        .map(|e| e.value)
        .sum()
    } else {
        0.0
    };
    let repeats = 1.0 + extra;
    let mut added = Vec::new();
    if repeats > 1.0 {
        let set = super::collect::active_config(calc.input().build);
        let mode = set.and_then(|s| {
            super::config::last_input(s, MODE_INPUT, loom_domain::ConfigInputKind::Input)
        });
        match mode {
            None => average(calc, repeats, cx, reasons, &mut added),
            Some(loom_domain::ConfigValue::String(m)) if m == "AVERAGE" => {
                average(calc, repeats, cx, reasons, &mut added);
            }
            Some(loom_domain::ConfigValue::String(m)) if m == "NONE" => {}
            Some(_) => {
                reasons.partial.insert(UnsupportedReason::UnmodelledConfig {
                    input: MODE_INPUT.to_owned(),
                });
            }
        }
    }
    added.push(Effect {
        quantity: recorded,
        op: Op::Base,
        value: repeats,
        conditions: Conditions::none(),
        source: COUNT_SOURCE,
    });
    for e in added {
        calc.add_effect(e);
    }
}

/// The `AVERAGE` mode's effects (module docs) into `added`.
fn average(
    calc: &Calc<'_>,
    repeats: f64,
    cx: &QueryContext,
    reasons: &mut Reasons,
    added: &mut Vec<Effect>,
) {
    let per_repeat = Quantity::Poe1(Poe1Quantity::RepeatPerRepeatDamage);
    for e in applying(calc, per_repeat, Op::More, cx, reasons) {
        added.push(Effect {
            quantity: Quantity::Poe1(Poe1Quantity::RepeatAverageDamage),
            op: Op::More,
            value: e.value * (repeats - 1.0) / 2.0,
            conditions: e.conditions,
            source: e.source,
        });
    }
    let final_double = Quantity::Poe1(Poe1Quantity::RepeatFinalDoubleDamageChance);
    for e in applying(calc, final_double, Op::Base, cx, reasons) {
        added.push(Effect {
            quantity: Quantity::Poe1(Poe1Quantity::DoubleDamageChance),
            op: Op::Base,
            value: e.value / repeats,
            conditions: e.conditions,
            source: e.source,
        });
    }
    // Each repeat's own damage: the first to the third, as the count
    // reaches them (`{2, "One"}`, `{3, "Two"}`, `{4, "Three"}`).
    let mut total = 0.0;
    let mut last: Option<Effect> = None;
    for n in 1..=3_u8 {
        if f64::from(n) + 1.0 > repeats {
            break;
        }
        for e in applying(
            calc,
            Quantity::Poe1(Poe1Quantity::RepeatDamage(n)),
            Op::More,
            cx,
            reasons,
        ) {
            total += e.value;
            last = Some(e);
        }
    }
    if let Some(last) = last {
        added.push(Effect {
            quantity: Quantity::Poe1(Poe1Quantity::RepeatAverageDamage),
            op: Op::More,
            value: (total / repeats + 100.0) - 100.0,
            conditions: last.conditions,
            source: last.source,
        });
    }
}
