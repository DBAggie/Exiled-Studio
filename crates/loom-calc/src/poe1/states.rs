//! The named conditions the build settles (Phase 3 plan, step 8; P3-8f):
//! what every PoE1 query context knows.
//!
//! **PoB v2.67.2** (`Modules/CalcPerform.lua`, `doActorAttribsConditions`,
//! 135 onward) sets, among others:
//! - `UsingShield` when the off hand ("Weapon 2") holds a shield;
//! - `Using<flag>` for the main hand's weapon type
//!   (`data.weaponTypeInfo`, `Modules/Data.lua` 550: a staff or warstaff
//!   is `Staff`).
//!
//! **The engine** settles only the conditions a mapping row reads
//! ([`USING_STAFF`], [`USING_SHIELD`]), each from the hands of the weapon
//! set in use ([`collect::hands`]): a hand holding an item whose base is
//! not known leaves its condition unsettled, so an effect that needs it is
//! `unmodelled-condition` (a reason), never a guess. Any other condition
//! stays unsettled, as before P3-8f: `OffHandIsEmpty`, `Unarmed`,
//! `UsingMeleeWeapon`, `UsingOneHandedWeapon`, `UsingTwoHandedWeapon`,
//! `DualWielding`, the other weapon types, and Necromantic Aegis's
//! shield; an effect that needs one is a reason.
//!
//! Differences on purpose: PoB also sets `Using<flag>` from the off hand's
//! weapon type (`CalcPerform.lua` 187 to 190), but a staff cannot be held
//! in the off hand. A shield is an armour base with a chance to block
//! (only shields have one). The conditions are settled once per
//! calculation (`Ruleset::states`, P3-8g); every query context comes from
//! `Calc::context`.

use std::collections::BTreeMap;

use super::collect::{self, Held};
use crate::condition::ConditionKey;
use crate::input::CalcInput;

/// The main hand holds a staff or a warstaff (PoB's `UsingStaff`).
pub const USING_STAFF: &str = "UsingStaff";

/// The off hand holds a shield (PoB's `UsingShield`).
pub const USING_SHIELD: &str = "UsingShield";

/// The main hand holds a weapon (P3-8n): PoB gives an attack with it
/// `ModFlag.Weapon` (`CalcActiveSkill.lua` 209: the weapon's type is not
/// `None`), which "with Weapons" modifiers need. Not a PoB condition.
pub const MAIN_HAND_WEAPON: &str = "MainHandWeapon";

/// The item classes of PoB's `Staff` weapon type.
const STAFF_CLASSES: [&str; 2] = ["Staff", "Warstaff"];

/// The conditions the build settles (module docs).
#[must_use]
pub fn states(input: &CalcInput<'_>) -> BTreeMap<ConditionKey, bool> {
    let hands = collect::hands(input);
    let mut out = BTreeMap::new();
    let staff = match &hands.main_hand {
        Held::Weapon(class) => Some(STAFF_CLASSES.contains(&class.as_str())),
        Held::Empty | Held::Shield | Held::Other => Some(false),
        Held::Unknown => None,
    };
    let shield = match &hands.off_hand {
        Held::Shield => Some(true),
        Held::Empty | Held::Weapon(_) | Held::Other => Some(false),
        Held::Unknown => None,
    };
    let weapon = match &hands.main_hand {
        Held::Weapon(_) => Some(true),
        Held::Empty | Held::Shield | Held::Other => Some(false),
        Held::Unknown => None,
    };
    for (key, value) in [
        (USING_STAFF, staff),
        (USING_SHIELD, shield),
        (MAIN_HAND_WEAPON, weapon),
    ] {
        if let Some(v) = value {
            out.insert(ConditionKey::new(key), v);
        }
    }
    // The config's condition checks (P3-8n).
    super::conditions::settle(input, &mut out);
    out
}
