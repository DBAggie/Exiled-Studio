//! An item's own values, as the game's tooltip shows them (B-2 item
//! evidence, E5): an armour piece's defences and a shield's chance to
//! block, a weapon's damage per type, attacks per second, and critical
//! strike chance.
//!
//! No calculation of its own: the build is collected as for a calculation
//! and the nodes the phases build for these values are built the same way
//! ([`phases::piece`], [`phases::shield_block`], [`hit::weapon_data`]), so
//! the card shows what the calculation uses. Only local lines act on these
//! quantities, so the caller passes a build holding the item alone (the
//! app: a level 1 build with the item in a slot that fits it).

use crate::engine::{self, Ruleset};
use crate::error::CalcError;
use crate::input::CalcInput;
use crate::quantity::{DamageType, Quantity};
use crate::state::{Reasons, UnsupportedReason};

use super::{ArmourSlot, Defence, PieceValue, Poe1Quantity, Poe1Rules, collect, hit, phases};

/// The values of the items in use in a build ([`item_values`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemValues {
    /// Each armour slot's defences above 0, slots in PoB's order, each
    /// slot's in [`Defence::ALL`] order.
    pub defences: Vec<(ArmourSlot, Defence, f64)>,
    /// The off hand's shield's chance to block, percent.
    pub block: Option<f64>,
    /// The main hand's weapon.
    pub weapon: Option<WeaponValues>,
    /// What the values may not account for: the queries' reasons, the
    /// item's lines not resolved, an unmodelled item, missing data.
    pub reasons: Reasons,
}

/// A weapon's values ([`ItemValues::weapon`]).
#[derive(Debug, Clone, PartialEq)]
pub struct WeaponValues {
    /// Damage per type, `(type, min, max)`, in PoB's order (physical,
    /// lightning, cold, fire, chaos), for each type the weapon deals.
    pub damage: Vec<(DamageType, f64, f64)>,
    /// Attacks per second.
    pub attack_rate: f64,
    /// Critical strike chance, percent.
    pub crit_chance: f64,
}

/// The values of the items `input`'s build has in use (module docs). A
/// PoE2 build has none.
///
/// # Errors
/// [`CalcError::NoDataVersion`] or [`CalcError::DataVersionMismatch`];
/// the collect and graph errors a calculation has.
pub fn item_values(input: &CalcInput<'_>) -> Result<ItemValues, CalcError> {
    input.check_data_version()?;
    let rules = Poe1Rules;
    if rules.game() != input.build.game() {
        return Ok(ItemValues::default());
    }
    let mut calc = engine::collected(input, &rules)?;
    let mut out = ItemValues::default();
    let mut reasons = Reasons::new();
    for slot in ArmourSlot::ALL {
        if !phases::has(
            &calc,
            Quantity::Poe1(Poe1Quantity::Piece(slot, PieceValue::Equipped)),
        ) {
            continue;
        }
        for defence in Defence::ALL {
            let value = phases::piece(&mut calc, slot, defence, &mut reasons)?;
            if value.value > 0.0 {
                out.defences.push((slot, defence, value.value));
            }
        }
    }
    out.block = phases::shield_block(&mut calc, &mut reasons)?.map(|b| b.value);
    let hands = collect::hands(input);
    let held = hands
        .main
        .map(|(id, _)| (id, super::Hand::Main))
        .or_else(|| hands.off.map(|(id, _)| (id, super::Hand::Off)));
    if let Some((id, hand)) = held {
        let weapon = hit::weapon_data(&mut calc, id, hand, &mut reasons)?;
        out.weapon = Some(WeaponValues {
            damage: hit::TYPES
                .iter()
                .zip(weapon.damage)
                .filter_map(|(t, range)| range.map(|[min, max]| (*t, min.value, max.value)))
                .collect(),
            attack_rate: weapon.rate.value,
            crit_chance: weapon.crit.value,
        });
    }
    // Of collecting's reasons (the build holds the item alone, so each is
    // about it): a line not resolved, which may be a local one, and an
    // unmodelled item or missing data. Not an unmapped stat: every stat
    // that acts on these values is mapped (`PieceMapping`, the weapon's
    // rows), so an unmapped one acts on something else (socketed gems, a
    // global stat).
    let global = calc.global();
    reasons.partial.extend(
        global
            .partial
            .iter()
            .filter(|r| !matches!(r, UnsupportedReason::UnmappedStat { .. }))
            .cloned(),
    );
    reasons
        .indeterminate
        .extend(global.indeterminate.iter().cloned());
    out.reasons = reasons;
    Ok(out)
}
