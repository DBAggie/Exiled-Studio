//! The chance to evade the enemy's attacks (Phase 6 P6-10b, part 11):
//! PoB v2.67.2's `EvadeChance`, `MeleeEvadeChance`, and
//! `ProjectileEvadeChance` (`Modules/CalcDefence.lua` 1049 to 1096).
//!
//! What PoB does:
//! - `MeleeEvasion` and `ProjectileEvasion` are `max(round(evasion x
//!   calcLib.mod(<Kind>Evasion)), 0)`;
//! - with `CannotEvade` (Unwavering Stance) every chance is 0; with
//!   `AlwaysEvade` every chance is 100;
//! - otherwise `enemyAccuracy = round(calcLib.val(enemyDB, "Accuracy"))`:
//!   the monster accuracy at the enemy's level (`CalcSetup.lua` 575) times
//!   the enemy's accuracy modifiers (Blind's 20% less, `CalcPerform.lua`
//!   831);
//! - `<Kind>EvadeChance = max(0, min(95, (100 - (hitChance(<kind>
//!   evasion, enemyAccuracy) - EvadeChance BASE) x enemy hit chance mod) x
//!   calcLib.mod(EvadeChance, <Kind>EvadeChance)))`, `hitChance` PoB's
//!   `calcs.hitChance` ([`super::hit::pob_hit_chance`]);
//! - `EvadeChance` is the melee one when both are equal, else `100 -
//!   (hitChance(evasion, enemyAccuracy) - EvadeChance BASE) x hit chance
//!   mod`, unclamped.
//!
//! The engine maps no stat of `MeleeEvasion`, `ProjectileEvasion`,
//! `EvadeChance`, or the enemy's hit chance yet: each such stat is an
//! unmapped one, so a reason, and their values are PoB's defaults (1, 0).

use super::phases::{self, Valued};
use super::{Poe1Field, Poe1Quantity, enemy};
use crate::condition::Flags;
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::{Node, Term};
use crate::quantity::{Op, Quantity};
use crate::result::Field;
use crate::source::Source;
use crate::state::IndeterminateReason;

/// PoB's `data.misc.EvadeChanceCap`.
pub const EVADE_CHANCE_CAP: f64 = 95.0;

/// The phase (module docs).
///
/// # Errors
/// A graph error.
pub fn phase(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let input = *calc.input();
    let Some(evasion) = calc.field(Field::Evasion).and_then(|f| {
        Some(Valued {
            node: f.node?,
            value: f.value?,
        })
    }) else {
        return Ok(());
    };
    let mut reasons = phases::marker_reasons(
        calc,
        |_| None,
        |q| {
            matches!(
                q,
                Quantity::Poe1(Poe1Quantity::EnemyAccuracy | Poe1Quantity::CannotEvade)
            )
        },
    );
    let cx = calc.context(Flags::NONE);
    let cannot = {
        let (store, graph) = calc.parts();
        let q = store.flag(graph, Quantity::Poe1(Poe1Quantity::CannotEvade), &cx)?;
        reasons.extend(&q.reasons);
        q.value > 0.0
    };
    let fields = [
        Poe1Field::EvadeChance,
        Poe1Field::MeleeEvadeChance,
        Poe1Field::ProjectileEvadeChance,
    ];
    if cannot {
        for f in fields {
            let zero = phases::constant(calc, "cannot-evade", 0.0)?;
            calc.set_derived(
                Field::Poe1(f),
                phases::query(zero, reasons.clone()),
                &[Field::Evasion],
            );
        }
        return Ok(());
    }
    let foe = enemy::enemy(input.build, input.data);
    #[expect(
        clippy::cast_precision_loss,
        reason = "accuracy values are far below 2^53"
    )]
    let table = match input.data.monster_level(foe.level) {
        Some(m) => m.accuracy as f64,
        None => {
            reasons
                .indeterminate
                .insert(IndeterminateReason::MissingData {
                    table: "monster_levels",
                    key: foe.level.to_string(),
                });
            return Ok(());
        }
    };
    // The enemy's base accuracy as a contribution of the enemy.
    let node = calc.graph_mut().push(
        Node::Contribution {
            source: Source::Enemy,
            quantity: Quantity::Poe1(Poe1Quantity::EnemyAccuracy),
            op: Op::Base,
            value: table,
        },
        table,
    )?;
    let base = Valued { node, value: table };
    let (inc, more) = super::hit::modifiers_of(
        calc,
        Quantity::Poe1(Poe1Quantity::EnemyAccuracy),
        &cx,
        &mut reasons,
    )?;
    let raw = phases::product(calc, &[base, inc, more], Term::Step("enemy-accuracy"))?;
    let accuracy = phases::round(calc, raw, Term::Step("enemy-accuracy-rounded"))?;
    // The melee and projectile evasion: `max(round(evasion x 1), 0)`.
    let rounded = phases::round(calc, evasion, Term::Step("kind-evasion"))?;
    let zero = phases::constant(calc, "no-negative-evasion", 0.0)?;
    let kind_evasion = phases::pair(calc, rounded, zero, Term::Step("kind-evasion"), false)?;
    let hit = super::hit::pob_hit_chance(calc, kind_evasion, accuracy, Term::Step("enemy-hit"))?;
    // `100 - (hit - 0) x 1`, at most 95, at least 0.
    let hundred = phases::constant(calc, "percent", 100.0)?;
    let evade = super::hit::minus(calc, hundred, hit, Term::Step("evade"))?;
    let cap = phases::constant(calc, "evade-chance-cap", EVADE_CHANCE_CAP)?;
    let capped = phases::pair(calc, evade, cap, Term::Step("evade-capped"), true)?;
    let value = phases::pair(calc, capped, zero, Term::Result, false)?;
    // Both kinds equal, so `EvadeChance` is the melee one.
    for f in fields {
        calc.set_derived(
            Field::Poe1(f),
            phases::query(value, reasons.clone()),
            &[Field::Evasion],
        );
    }
    Ok(())
}
