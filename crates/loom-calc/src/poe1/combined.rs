//! The main skill's combined damage per second (Phase 6 P6-12, part 1):
//! PoB v2.67.2's `CombinedDPS` (`Modules/CalcOffence.lua` 5982 to 6139).
//!
//! What PoB does, for a skill that does not show its average damage:
//! - `CombinedDPS = TotalDPS + ImpaleDPS + TotalDotDPS`, `TotalDotDPS =
//!   min(TotalDot + TotalPoisonDPS + IgniteDPS + BleedDPS (+ ground,
//!   corrupting blood, decay), DotDpsCap)`, the cap `35791394` (`Data.lua`
//!   255);
//! - times the cull multiplier `100 / (100 - CullPercent)` (3217 to 3221:
//!   Culling Strike's 10, [`Poe1Quantity::CullingStrike`]) and the
//!   reservation multiplier `100 / (100 - LifeReservationPercent)` of the
//!   enemy (1 here: its stats stay reasons).
//!
//! The engine reads the fields the earlier phases computed: the hit's DPS
//! (a dependency: a skill whose hit is not computed, a damage over time
//! skill among them, leaves this field unsupported), and each of impale,
//! damage over time, ignite, bleeding, and poison that the build has (each
//! a dependency, so an unsupported one leaves this field unsupported and a
//! partial one makes it partial). Not modelled: mirages (P6-8b), ground
//! damage over time and corrupting blood (the damage over time field is
//! then unsupported), decay, critical culling, and skills that show their
//! average damage (their stat stays a reason).

use super::phases::{self, Valued};
use super::{Poe1Field, Poe1Quantity};
use crate::condition::Flags;
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::{Node, Term};
use crate::quantity::Quantity;
use crate::result::Field;
use crate::state::{Reasons, UnsupportedReason};

/// PoB's `data.misc.DotDpsCap`: `(2^31 - 1) / 60`.
pub const DOT_DPS_CAP: f64 = 35_791_394.0;

/// Culling Strike's percent (PoB's `CullPercent` MAX 10).
pub const CULL_PERCENT: f64 = 10.0;

/// The combined phase (module docs).
///
/// # Errors
/// A graph error.
pub fn phase(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let input = *calc.input();
    let levels = super::gem_levels::GemLevels::from_effects(calc.store().effects());
    let Ok(skill) = super::skill::main_skill(input.build, input.data, &levels) else {
        return Ok(());
    };
    let field = Field::Poe1(Poe1Field::CombinedDps);
    let value_of = |calc: &Calc<'_>, f: Field| {
        calc.field(f).and_then(|r| {
            Some(Valued {
                node: r.node?,
                value: r.value?,
            })
        })
    };
    let Some(hit) = value_of(calc, Field::HitDps) else {
        calc.set_unsupported(
            field,
            UnsupportedReason::DependsOnNotComputed {
                field: Field::HitDps,
            },
        );
        return Ok(());
    };
    let mut deps = vec![Field::HitDps];
    let mut terms = vec![hit];
    if calc.field(Field::Poe1(Poe1Field::ImpaleDps)).is_some() {
        deps.push(Field::Poe1(Poe1Field::ImpaleDps));
        if let Some(v) = value_of(calc, Field::Poe1(Poe1Field::ImpaleDps)) {
            terms.push(v);
        }
    }
    let mut dots = Vec::new();
    for f in [
        Field::DotDps,
        Field::PoisonDps,
        Field::IgniteDps,
        Field::BleedDps,
    ] {
        if calc.field(f).is_some() {
            deps.push(f);
            if let Some(v) = value_of(calc, f) {
                dots.push(v);
            }
        }
    }
    if !dots.is_empty() {
        let sum = phases::total(calc, &dots, Term::Step("dot-sum"))?;
        let cap = phases::constant(calc, "dot-dps-cap", DOT_DPS_CAP)?;
        terms.push(phases::pair(
            calc,
            sum,
            cap,
            Term::Step("dot-capped"),
            true,
        )?);
    }
    let mut combined = phases::total(calc, &terms, Term::Step("combined"))?;
    // Culling Strike in the skill's context.
    let mut flags = skill.keyword_flags();
    if skill.types.contains("Attack") {
        flags = flags | Flags::ATTACK;
    }
    if skill.types.contains("Spell") {
        flags = flags | Flags::SPELL;
    }
    let mut reasons = Reasons::new();
    let cx = calc.context(flags);
    let (store, graph) = calc.parts();
    let cull = store.flag(graph, Quantity::Poe1(Poe1Quantity::CullingStrike), &cx)?;
    reasons.extend(&cull.reasons);
    if cull.value > 0.0 {
        let value = 100.0 / (100.0 - CULL_PERCENT);
        let multiplier = phases::constant(calc, "cull-multiplier", value)?;
        let node = calc.graph_mut().push(
            Node::Any {
                term: Term::Step("culling-strike"),
                of: vec![cull.node],
            },
            1.0,
        )?;
        let on = Valued { node, value: 1.0 };
        combined = phases::product(calc, &[combined, multiplier, on], Term::Step("culled"))?;
    }
    let result = phases::total(calc, &[combined], Term::Result)?;
    calc.set_derived(field, phases::query(result, reasons), &deps);
    Ok(())
}
