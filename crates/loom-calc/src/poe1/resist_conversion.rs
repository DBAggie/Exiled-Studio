//! Resistance conversion (P6-11ah): PoB v2.67.2's `calcs.resistances`
//! (`Modules/CalcDefence.lua` 513 to 556), before the resistances.
//!
//! What PoB does, for each pair of resistance types:
//! - **Maximum:** with a `<From>MaxResConvertTo<To>` BASE sum above 0
//!   ("Modifiers to Maximum Fire Resistance also apply to Maximum Cold and
//!   Lightning Resistances", 100), the `<From>ResistMax` BASE mods not from
//!   the character's base are added to `<To>ResistMax` at that rate.
//! - **Resistance:** with a `<From>ResConvertTo<To>` BASE sum ("Modifiers to
//!   Fire Resistance also apply to Cold and Lightning Resistances at #% of
//!   their Value"), the `<From>Resist` BASE mods not from the base (the
//!   resistance penalty), and every `<From>Resist` INC and MORE mod, are
//!   added to `<To>Resist` at that rate.
//!
//! Only mods named for the one type count: `ElementalResist` and
//! `ElementalResistMax` do not. The engine reads an elemental line as one
//! effect per element, so an effect of the source type is left out when
//! the same source gives the other two elements the same value under the
//! same conditions (an all-elemental line, Purity of Elements, the
//! bandit's 15). PoB tabulates the mods with no skill config: an effect
//! that needs query flags is left out.
//!
//! Only fire converts in the game's data: the rates' quantity names their
//! target, the source being fire. A rate that holds only under a condition
//! is a reason on every field.

use super::Poe1Quantity;
use crate::condition::Flags;
use crate::ingest::Ingest;
use crate::quantity::{DamageType, Op, Quantity};
use crate::source::Source;
use crate::state::UnsupportedReason;
use crate::store::{Conditions, Effect};

/// The resistance penalty's source (`collect::resistance_penalty`).
const PENALTY: Source = Source::Rule("resistance-penalty");

/// The elements in the order PoB lists them.
const ELEMENTS: [DamageType; 3] = [DamageType::Fire, DamageType::Cold, DamageType::Lightning];

/// Adds the converted effects (module docs) after every source of
/// resistance modifiers.
pub fn resist_conversion(ingest: &mut Ingest<'_>) {
    let mut added = Vec::new();
    for maximum in [true, false] {
        for to in [DamageType::Cold, DamageType::Lightning] {
            let rate_quantity = Quantity::Poe1(Poe1Quantity::ResistConvert(maximum, to));
            let mut rate = 0.0;
            for e in ingest.effects() {
                if e.quantity != rate_quantity || e.op != Op::Base {
                    continue;
                }
                if e.conditions != Conditions::none() {
                    added.push(Err(e.source.clone()));
                    continue;
                }
                rate += e.value;
            }
            if rate == 0.0 {
                continue;
            }
            let rate = rate / 100.0;
            let from = if maximum {
                Quantity::MaximumResistance(DamageType::Fire)
            } else {
                Quantity::Resistance(DamageType::Fire)
            };
            let target = if maximum {
                Quantity::MaximumResistance(to)
            } else {
                Quantity::Resistance(to)
            };
            let effects = ingest.effects();
            for e in effects {
                if e.quantity != from
                    || matches!(e.source, Source::Character)
                    || e.source == PENALTY
                    || e.conditions.all != Flags::NONE
                    || e.conditions.any != Flags::NONE
                {
                    continue;
                }
                let ops: &[Op] = if maximum {
                    &[Op::Base]
                } else {
                    &[Op::Base, Op::Increased, Op::More]
                };
                if !ops.contains(&e.op) || elemental(effects, e, maximum) {
                    continue;
                }
                added.push(Ok(Effect {
                    quantity: target,
                    op: e.op,
                    value: e.value * rate,
                    conditions: e.conditions.clone(),
                    source: e.source.clone(),
                }));
            }
        }
    }
    for a in added {
        match a {
            Ok(effect) => ingest.effect(effect),
            Err(source) => ingest.reason(UnsupportedReason::UnmodelledSkill {
                source,
                what: "resistance-conversion",
            }),
        }
    }
}

/// Whether `e`, a fire effect, is one of an elemental line's three: the
/// same source gives cold and lightning the same op, value, and
/// conditions.
fn elemental(effects: &[Effect], e: &Effect, maximum: bool) -> bool {
    ELEMENTS[1..].iter().all(|&t| {
        let q = if maximum {
            Quantity::MaximumResistance(t)
        } else {
            Quantity::Resistance(t)
        };
        effects.iter().any(|o| {
            o.quantity == q
                && o.op == e.op
                && o.value == e.value
                && o.conditions == e.conditions
                && o.source == e.source
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poe1::mapping::Poe1Mapping;

    fn effect(quantity: Quantity, op: Op, value: f64, source: Source) -> Effect {
        Effect {
            quantity,
            op,
            value,
            conditions: Conditions::none(),
            source,
        }
    }

    /// Half of a fire-only line converts to cold and lightning; an
    /// all-elemental line, the penalty, and the base maximum do not
    /// (PoB's `FireResist` alone, not `ElementalResist` or "Base").
    #[test]
    fn only_fire_lines_should_convert() {
        let mut ingest = Ingest::new(&Poe1Mapping);
        let fire = Source::Gem { group: 0, gem: 0 };
        let all = Source::Gem { group: 0, gem: 1 };
        let rate = Source::Gem { group: 0, gem: 2 };
        ingest.effect(effect(
            Quantity::Resistance(DamageType::Fire),
            Op::Base,
            40.0,
            fire,
        ));
        for t in ELEMENTS {
            ingest.effect(effect(Quantity::Resistance(t), Op::Base, 10.0, all.clone()));
        }
        ingest.effect(effect(
            Quantity::Resistance(DamageType::Fire),
            Op::Base,
            -60.0,
            PENALTY,
        ));
        ingest.effect(effect(
            Quantity::MaximumResistance(DamageType::Fire),
            Op::Base,
            75.0,
            Source::Character,
        ));
        for to in [DamageType::Cold, DamageType::Lightning] {
            ingest.effect(effect(
                Quantity::Poe1(Poe1Quantity::ResistConvert(false, to)),
                Op::Base,
                50.0,
                rate.clone(),
            ));
            ingest.effect(effect(
                Quantity::Poe1(Poe1Quantity::ResistConvert(true, to)),
                Op::Base,
                100.0,
                rate.clone(),
            ));
        }
        let before = ingest.effects().len();
        resist_conversion(&mut ingest);
        let added: Vec<_> = ingest.effects()[before..]
            .iter()
            .map(|e| (e.quantity, e.value))
            .collect();
        assert_eq!(
            added,
            [
                (Quantity::Resistance(DamageType::Cold), 20.0),
                (Quantity::Resistance(DamageType::Lightning), 20.0),
            ]
        );
    }
}
