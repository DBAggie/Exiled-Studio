//! The main skill's hit when it is a self-cast spell (Phase 3 plan, step
//! 9b) or an attack with the weapon in the main hand (step 10): its average
//! damage against the enemy, cast or attack rate, chance to hit (attacks),
//! critical strike chance and multiplier, and hit DPS, as PoB v2.67.2
//! computes them (`Modules/CalcOffence.lua`, `calcs.offence`), with the
//! same order of operations and rounding points. Its steps, each cited
//! below:
//!
//! 1. **Base damage** per damage type (`Calculate base hit damage`): the
//!    source's (a spell's own `skillData.ColdMin`, the gem's resolved level
//!    value; an attack's weapon, step 8) times the skill's base multiplier,
//!    plus added damage times the skill's damage effectiveness:
//!    `base x multiplier + added x effectiveness`.
//! 2. **Damage** per type (`calcDamage`): `round(base x (1 + inc / 100) x
//!    more + converted)`, the increases and more of `Damage` and the type's
//!    own (`ColdDamage`, `ElementalDamage`); a melee hit's physical damage
//!    also has strength's `floor(Str / 5)`% increased (`Modules/
//!    CalcPerform.lua`: `PhysicalDamage` INC `strDmgBonus`, `ModFlag.Melee`).
//!    Damage converted from an
//!    earlier type (PoB's order: physical, lightning, cold, fire, chaos) is
//!    that type's damage computed with both types' modifiers, times the
//!    share converted, rounded (`calcDamage` with `convDst`); a type with
//!    no base of its own is its converted damage alone. Only the skill's
//!    own conversion is modelled (`Skill<From>DamageConvertTo<To>`: above
//!    100% in total it is scaled to 100%); a type keeps `1 - min(total /
//!    100, 1)` of its own damage. Every more multiplier follows PoB's rule:
//!    each mod name's product rounded to two decimals, per list (the
//!    skill's gems, the character).
//! 3. **Each pass** (non-crit, then crit with the multiplier): per type
//!    `min / 2 + max / 2` (no luck) of the damage times the share kept,
//!    times the enemy's effect: `1 - (resist - penetration) / 100`, the
//!    resistance capped at its maximum (75, or the configured resistance
//!    above it, at most 90) and at least -200 (`calcResistForType`); for
//!    physical damage `1 - reduction / 100`, the reduction its armour
//!    gives against the pass's average hit, `armour / (armour + hit x 5)
//!    x 100` (`calcs.armourReductionF`), at most the monster template's
//!    `maximum_physical_damage_reduction_%` (75). The effect is then
//!    multiplied by the enemy's damage taken, `(1 + inc / 100) x more`
//!    (`effMult`), when the config gives it any ([`super::damage_taken`]:
//!    shock, the Uber boss, delirium). Summed over the types in PoB's
//!    order.
//! 4. **Crit chance**: `min((base + added) x (1 + inc / 100) x more, 100)`,
//!    at least 0, then times the accuracy hit chance / 100 (100 for
//!    spells). The base is the skill's for a spell, the weapon's for an
//!    attack. **Crit multiplier**: `1 + max(0, round(sum / 100, 2))`, the
//!    sum from the character's base (the template's 150, less 100) and the
//!    stats.
//! 5. **Average hit** `hit x (1 - crit / 100) + crit hit x crit / 100`;
//!    **average damage** its product with the hit chance / 100.
//! 6. **Rate**: a spell's `1 / (cast time / round((1 + inc / 100) x more,
//!    2))`; an attack's the same with `1 / (weapon rate x (1 + attack speed
//!    multiplier / 100))` for the cast time; at most the server tick rate
//!    (`1 / 0.033`).
//! 7. **Hit DPS** `average damage x rate x 1 x 1` (PoB's DPS and quantity
//!    multipliers, which no mapped stat changes).
//! 8. **The weapon** (`Classes/Item.lua`, the weapon branch): physical
//!    damage `round((base + local flat) x (1 + local inc / 100) x (1 +
//!    quality / 100))`, elemental `round(local flat x (1 + local inc /
//!    100))`, chaos the local flat; a type counts only when both bounds are
//!    above 0. Attack rate, the game's model (Q11, confirmed in game
//!    2026-11-12; a `game-truth` divergence from PoB): `1000 / floor(attack
//!    time x 100 / (100 + local attack speed))`, the attack time in integer
//!    milliseconds floored after the local increases. PoB instead computes
//!    `round(round(1000 / attack time, 2) x (1 + local attack speed / 100),
//!    2)`. Where the skill's attack speed multiplier and global increases
//!    enter (step 6) stays PoB's (open question Q12).
//!    Crit `round((base + local) x (1 +
//!    local inc / 100), 2)`.
//! 9. **Chance to hit** (attacks; `Calculate hit chance`): accuracy
//!    `max(0, floor(base x (1 + inc / 100) x more))`, the base the level's
//!    (`accuracy_rating_per_level x (level - 1)`, `Modules/CalcSetup.lua`),
//!    2 per dexterity (`data.misc.AccuracyPerDexBase`), and the stats;
//!    `max(min(round(accuracy / (accuracy + (evasion / 5) ^ 0.9) x 125),
//!    100), 5)` against the enemy's evasion rounded (`calcs.hitChance`;
//!    the evasion is `calcLib.val`: the enemy model's base times the
//!    enemy's evasion INC and MORE, only Blind's so far, P3-12g).
//!    The enemy blocks nothing modelled, so the hit chance is the accuracy
//!    hit chance.
//!
//! Every factor PoB applies that is 1 or 0 without a stat the mapping does
//! not know (other conversion and gain, luck, double damage, action speed,
//! damage taken of one type or by projectiles, traps, and mines, the
//! enemy's block, the hit chance modifier) is left out: a stat that
//! would change it is unmapped and makes the fields `Partial`. The default
//! enemy's other modifiers reach no mapped stat
//! (`poe1::enemy` module docs).
//!
//! Repeats (P6-11aj) are [`super::repeats`]'s: settled at the start of
//! each hit, their averaged damage read with the `Damage` MORE, their
//! count in the rate's caps.
//!
//! **Which skills** (`spell_skill`, `attack_skill`): a spell and a hit
//! (PoB's `skillFlags.hit`: a `Damage` or `Projectile` type), cast by the
//! player, a totem, a trap, or a mine: not an attack, triggered, a minion,
//! or brand ([`NOT_SELF_CAST`]; channelled skills since P6-5, totems since
//! P6-6d, `Partial` through [`TOTEM_REASON`]; traps and mines since P6-6e,
//! `Partial` through [`TRAP_MINE_REASON`], their attacks and DoT not
//! computed), with a cast time (a cooldown caps the rate since P6-7c3b,
//! [`cooldown_limit`]; a trap's cooldown does not, P6-11hp; a mine's alone
//! is not modelled); or
//! an attack
//! that is none of those but an attack, without a cooldown, with a weapon
//! in the main hand of a class the skill can use and no weapon in the off
//! hand (dual wielding is not modelled); in an enabled group. Every other
//! main skill leaves the hit fields not computed. A spell's area flag is
//! the game's `is_area_damage` stat (PoB's hand-written `area` flag; equal
//! for Ice Nova). An attack's melee and area flags are its skill part's
//! ([`SKILL_PARTS`], PoB's hand-written parts; the gem's `skillPart`,
//! default 1, at most the number of parts; 0 leaves the fields not
//! computed, where PoB would index a part that does not exist). PoB also
//! disables a skill whose supports exclude the weapon (`getWeaponFlags`
//! over each support's `weaponTypes`); the bundle does not carry supports'
//! weapon restrictions yet, and no support with one is mapped (its stats
//! are unmapped, so the fields are `Partial`): a support with a weapon
//! restriction must bring the data with its mapping.

use std::collections::BTreeMap;

use loom_domain::{Build, ItemId, SocketGroup};

use super::collect;
use super::enemy::{self, Enemy, EnemyProblem};
use super::gem_levels::GemLevels;
use super::keystones;
use super::phases::{self, Valued};
use super::skill::{self, MainSkill};
use super::{Hand, Poe1Quantity, WeaponValue};
use crate::condition::{Applies, ConditionKey, Flags, QueryContext};
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::{Node, Term};
use crate::input::{CalcData, CalcInput, StatSetLevel};
use crate::quantity::{Bound, DamageType, Op, Quantity};
use crate::result::Field;
use crate::source::Source;
use crate::state::{IndeterminateReason, Reasons, UnsupportedReason};
use crate::store::Effect;

/// Skill types that make a spell something other than a plain self-cast
/// hit: PoB computes their rate or damage another way (attacks with
/// weapons, triggers, totems, traps, mines, minions, brands' activations).
/// Totems' skills are plain hits since P6-6d: PoB computes a totem's hit
/// as the skill's, per totem, with the totem's action speed (`1 +
/// TotemActionSpeed INC / 100`, `CalcOffence.lua` 2346 to 2351) in place
/// of the player's (no totem action speed stat is mapped: 1).
/// Channelled skills are plain hits since P6-5: PoB computes them as any
/// skill, without the server tick cap on their rate (`CalcOffence.lua`
/// 2366); their stages and release are skill data and tagged stat map
/// entries, which leave a skill unvetted ([`pob_vetted`]).
pub const NOT_SELF_CAST: [&str; 5] = ["Attack", "Triggered", "Minion", "CreatesMinion", "Brand"];

/// Skill types that make an attack something other than a plain attack
/// with the weapon in hand ([`NOT_SELF_CAST`] but `Attack`).
pub const NOT_PLAIN_ATTACK: [&str; 6] = [
    "Triggered",
    "Trapped",
    "RemoteMined",
    "Minion",
    "CreatesMinion",
    "Brand",
];

/// A part of a skill as PoB's skill data writes it (`parts`), with the
/// flags it sets (`skillFlags.melee`, `skillFlags.area`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillPart {
    /// PoB's name of the part.
    pub name: &'static str,
    /// The part's hits are melee.
    pub melee: bool,
    /// The part's hits are area damage.
    pub area: bool,
    /// The part's hits are projectiles (P6-6b).
    pub projectile: bool,
}

/// The reason a totem's skill keeps where PoB reads totem data the engine
/// does not (P6-6d review; narrowed in open items S1). Since P6-6d the
/// engine reads the totem keyword (`Flags::KEYWORD_TOTEM`), the totem
/// conditions (`conditions::summoned_totem`, `skill_used_by_you`,
/// `main_skill_recently`), and the totem limit (`stacks::totems`); the
/// totem's action speed comes only from unmapped stats (reasons). What
/// stays:
/// - every field of a skill in [`TOTEM_DATA_SKILLS`];
/// - a totem's bleed, whose stacks PoB multiplies by the active totems
///   (`CalcOffence.lua` 4327 to 4330), which the engine's bleed does not;
/// - a totem's damage over time (`poe1::dot`), which PoB may stack per
///   totem (`DotCanStackAsTotems`, 5784).
///
/// A totem's leech is PoB's leech to the player only (3528 to 3530), which
/// no mapped stat gives: [`super::leech`] sets no leech field for it.
pub const TOTEM_REASON: &str = "totem-data";

/// Totem skills whose hit reads totem data the engine does not: Siege
/// Ballista of Trarthus' `totemFireOnce` and ring `dpsMultiplier`
/// (`act_dex.lua` 14127 to 14133; `CalcOffence.lua` 2352 to 2355).
pub const TOTEM_DATA_SKILLS: [&str; 1] = ["SiegeBallistaAltY"];

/// The reason a trap's or a mine's spell kept (P6-6e) while the engine did
/// not read its throw count, its throwing speed, or its keyword mods.
/// Since P6-11fa none is left: the speed (P6-11cy), the keywords
/// (`Flags::KEYWORD_TRAP` and `KEYWORD_MINE`), and the counts are read, and
/// a trap or mine with a cooldown is not computed (no skill of PoB's data
/// has a `trapCooldown`). Kept as the name of that former reason.
pub const TRAP_MINE_REASON: &str = "trap-mine-data";

/// The config's mines per throw (`ConfigOptions.lua` 1575:
/// `MineThrowCount` OVERRIDE in combat; P6-11fa).
pub const MINES_PER_THROW_INPUT: &str = "minesPerThrow";
/// The config's traps per throw (1587: `TrapThrowCount` OVERRIDE).
pub const TRAPS_PER_THROW_INPUT: &str = "trapsPerThrow";

/// PoB's base trap throwing time and mine laying time, seconds
/// (`CalcSetup.lua` 55, 56).
const TRAP_THROWING_TIME: f64 = 0.6;
/// See [`TRAP_THROWING_TIME`].
const MINE_LAYING_TIME: f64 = 0.3;

/// `SkillData` keys of PoB's base mods that only label or size what PoB
/// shows (the area's radius and its labels), read into no slice field
/// (P6-6b).
pub const DISPLAY_SKILL_DATA: [&str; 5] = [
    "radius",
    "radiusLabel",
    "radiusSecondary",
    "radiusSecondaryLabel",
    "radiusTertiaryLabel",
];

/// The fields of a PoB part the engine reads (P6-6b): its name and flags.
/// `chaining` (Static Strike's parts; agent list) sets PoB's
/// `skillFlags.chaining`, read for the chain count (`CalcOffence.lua` 1032
/// to 1046), which reaches no slice field (`number_of_chains` and the
/// per-chain lines are no-effect rows or guards), and for Pact of Beidat,
/// a spell's.
const PART_FIELDS: [&str; 5] = ["area", "chaining", "melee", "name", "projectile"];

/// The main skill's part from PoB's skill data (P6-6b; the generated
/// [`super::pob_skills`], v2.67.2's `Data/Skills/*.lua`): the base flags
/// `area`, `melee`, `projectile` (`baseFlags`), each overridden by the part's
/// own field, the part `m_min(#parts, skillPart or 1)` (`wanted`, 1-based).
/// `None` when PoB has no data for the skill.
#[must_use]
pub fn pob_part(id: &str, wanted: usize) -> Option<(SkillPart, usize)> {
    let pob = super::pob_skill::skill(id)?;
    let mut part = SkillPart {
        name: "",
        melee: pob.flag("melee"),
        area: pob.flag("area"),
        projectile: pob.flag("projectile"),
    };
    if pob.parts.is_empty() {
        return Some((part, 0));
    }
    let index = wanted.min(pob.parts.len()).max(1);
    let p = pob.parts[index - 1];
    part.name = p.name();
    let truthy = |k: &str| p.get(k).map(super::pob_skill::Value::truthy);
    part.melee = truthy("melee").unwrap_or(part.melee);
    part.area = truthy("area").unwrap_or(part.area);
    part.projectile = truthy("projectile").unwrap_or(part.projectile);
    Some((part, index))
}

/// An attack's melee and projectile flags after its main-hand weapon of
/// class `class` (`CalcActiveSkill.lua` 302 to 308; P6-6b review): a melee
/// weapon clears the projectile flag, a ranged one (bow, wand: the
/// classes PoB's `weaponTypeInfo` marks not melee) the melee flag.
#[must_use]
pub fn weapon_flags(class: &str, melee: bool, projectile: bool) -> (bool, bool) {
    // Kept apart from [`weapon_type_flags`]: this one decides the part's
    // melee and projectile flags, as PoB's `weaponTypeInfo.melee`.
    let ranged = matches!(class, "Bow" | "Wand");
    if !ranged && melee {
        (true, false)
    } else if ranged && projectile {
        (false, true)
    } else {
        (melee, projectile)
    }
}

/// The weapon flags of an attack with a weapon of item class `class`
/// (P6-11b; PoB's `getWeaponFlags`, `CalcActiveSkill.lua` 192 to 223, with
/// `data.weaponTypeInfo`, `Modules/Data.lua` 550): its type, `Weapon`,
/// one- or two-handed, and melee or ranged. `None` for a class PoB has no
/// weapon type of (the attack is not computed). A weapon that counts as
/// all one-handed types (Varunastra) is not modelled: its line is an
/// unmapped stat (a reason).
#[must_use]
pub fn weapon_type_flags(class: &str) -> Option<Flags> {
    let (kind, one_hand, melee) = match class {
        "Bow" => (Flags::BOW, false, false),
        "Claw" => (Flags::CLAW, true, true),
        "Dagger" | "Rune Dagger" => (Flags::DAGGER, true, true),
        "Staff" | "Warstaff" => (Flags::STAFF, false, true),
        "Wand" => (Flags::WAND, true, false),
        "One Hand Axe" => (Flags::AXE, true, true),
        "Two Hand Axe" => (Flags::AXE, false, true),
        "One Hand Mace" | "Sceptre" => (Flags::MACE, true, true),
        "Two Hand Mace" => (Flags::MACE, false, true),
        "One Hand Sword" | "Thrusting One Hand Sword" => (Flags::SWORD, true, true),
        "Two Hand Sword" => (Flags::SWORD, false, true),
        "FishingRod" => (Flags::FISHING, false, true),
        _ => return None,
    };
    let hands = if one_hand {
        Flags::WEAPON_1H
    } else {
        Flags::WEAPON_2H
    };
    let range = if melee {
        Flags::WEAPON_MELEE
    } else {
        Flags::WEAPON_RANGED
    };
    Some(kind | Flags::WEAPON | hands | range)
}

/// Whether the engine reads PoB's hand-written data of the skill that
/// changes its hit (P6-6b): no skill functions, no stat of its stat map
/// read under a tag ([`super::pob_skill::PobSkill::own_stats`], P6-6b
/// review), no base mod but the display `SkillData`
/// ([`DISPLAY_SKILL_DATA`], untagged), and, for part `index` (1-based; 0
/// without parts), no field but a name and flags ([`PART_FIELDS`]). Else
/// the hit is `Partial` (`unmodelled-skill`, `pob-skill-data`): PoB computes
/// it from data the engine does not read. The skill sweep is the evidence
/// (`docs/phase-6/skill-data.md`, "P6-6b").
///
/// Three lists name what the engine reads of that data (open items, S1):
/// [`READ_OWN_STATS`], [`READ_PART_BASE_MODS`], and [`READ_FUNCTIONS`]
/// (the last only through [`pob_vetted_with`], which knows the level's
/// cooldown; here no function counts as read).
#[must_use]
pub fn pob_vetted(id: &str, index: usize) -> bool {
    pob_vetted_with(id, index, false)
}

/// Stats of a skill's own stat map that PoB reads under a tag
/// ([`super::pob_skill::PobSkill::own_stats`]) and the engine reads with
/// that tag (open items, S1):
/// - Shield Crush's added damage per 15 of the shield's armour and evasion
///   (`act_str.lua` 9856; `SkillStatMap.lua` 1675 to 1697, `PerStat`
///   `ArmourOnWeapon` and `EvasionOnWeapon` div 15): `hit::shield_added`
///   (P6-11hd);
/// - Shield Crush's damage by distance (`MORE` `Hit` with `MeleeProximity`
///   ramp `{1, 0}`, `act_str.lua` 9875 to 9877):
///   `collect::melee_proximity` (P6-11ht);
/// - Static Strike's less beam damage (`Damage` MORE in part 2,
///   `act_str.lua` 10679 to 10681): [`static_strike_beams`] (agent list);
/// - Ground Slam's more damage to close targets (`Damage` MORE `Hit` with
///   `Condition:AtCloseRange`, `act_str.lua` 2086 to 2088): the mapping's
///   state row under the config's `conditionAtCloseRange` (open items, S1);
/// - Lacerate's stance stats (the agent list, 2026-11-27): the mapping's
///   state rows under the settled stance and the config's `changedStance`.
pub const READ_OWN_STATS: [(&str, &str); 13] = [
    // The agent list (2026-11-27): Elemental Hit of the Spectrum's more
    // damage per the enemy's elemental ailments (`stacks::
    // enemy_elemental_ailments`).
    (
        "ElementalHitAltX",
        "elemental_hit_damage_+%_final_per_enemy_elemental_ailment",
    ),
    ("GroundSlam", "groundslam_damage_to_close_targets_+%_final"),
    // Perforate's Blood Stance damage (the mapping's state row) and its
    // Duality's extra spears ([`perforate_dps_multiplier`]; the agent list,
    // 2026-11-27).
    ("Perforate", "blood_spears_damage_+%_final_in_blood_stance"),
    (
        "PerforateAltX",
        "blood_spears_additional_number_of_spears_if_changed_stance_recently",
    ),
    (
        "PerforateAltX",
        "blood_spears_damage_+%_final_in_blood_stance",
    ),
    // The agent list (2026-11-27): Kinetic Blast of Clustering's added
    // physical damage per maximum mana (`mana_added`).
    (
        "KineticBlastAltX",
        "added_physical_damage_to_attacks_equal_to_%_maximum_mana",
    ),
    (
        "Lacerate",
        "active_skill_bleeding_damage_+%_final_in_blood_stance",
    ),
    ("Lacerate", "attack_speed_+%_if_changed_stance_recently"),
    (
        "Lacerate",
        "chance_to_bleed_on_hit_%_chance_in_blood_stance",
    ),
    ("StaticStrike", BEAM_DAMAGE_STAT),
    (
        "ShieldCrush",
        "off_hand_maximum_added_physical_damage_per_15_shield_armour_and_evasion_rating",
    ),
    (
        "ShieldCrush",
        "off_hand_minimum_added_physical_damage_per_15_shield_armour_and_evasion_rating",
    ),
    ("ShieldCrush", "shield_crush_damage_+%_final_from_distance"),
];

/// Base mods tagged with one part (`{ type = "SkillPart", skillPart = n }`)
/// the engine reads: (skill, `SkillData` key, part). Since the agent list
/// (2026-11-27) each is PoB's `dpsMultiplier` of that part
/// ([`part_dps_multiplier`]), which PoB multiplies into `TotalDPS`, every
/// damaging ailment's stacks, the leech hit rate, and the impale DPS
/// (`CalcOffence.lua` 3712, 4322, 4603, 4908, 4024, 6035):
/// - Shield Crush's and its transfigured gem's 2 in part 2, "2 Overlapping
///   Waves" (`act_str.lua` 9887);
/// - Shock Nova's and its transfigured gem's 2 in part 1, "Ring + Nova";
/// - Lacerate's and its two transfigured gems' and Eviscerate's 2 in part
///   2, both slashes.
pub const READ_PART_BASE_MODS: [(&str, &str, usize); 11] = [
    ("Eviscerate", "dpsMultiplier", 2),
    // Perforate's spike overlap ([`perforate_dps_multiplier`]); its part
    // index is 0 (no parts), where the base mod's 1 is that function's
    // starting value.
    ("Perforate", "dpsMultiplier", 0),
    ("PerforateAltX", "dpsMultiplier", 0),
    ("PerforateAltY", "dpsMultiplier", 0),
    ("Lacerate", "dpsMultiplier", 2),
    ("LacerateAltX", "dpsMultiplier", 2),
    ("LacerateAltY", "dpsMultiplier", 2),
    ("ShieldCrush", "dpsMultiplier", 2),
    ("ShieldCrushAltX", "dpsMultiplier", 2),
    ("ShockNova", "dpsMultiplier", 1),
    ("ShockNovaAltX", "dpsMultiplier", 1),
];

/// PoB's `dpsMultiplier` of part `index` of skill `id` (1-based), from
/// [`READ_PART_BASE_MODS`]; 1 otherwise.
#[must_use]
pub fn part_dps_multiplier(id: &str, index: usize) -> f64 {
    if !READ_PART_BASE_MODS
        .iter()
        .any(|&(s, k, part)| s == id && k == "dpsMultiplier" && part == index)
    {
        return 1.0;
    }
    super::pob_skill::skill(id)
        .and_then(|p| {
            p.base_mods
                .iter()
                .find(|m| m.tagged && m.key == Some("dpsMultiplier"))
        })
        .and_then(|m| match m.value {
            Some(super::pob_skill::Value::Num(v)) => Some(v),
            _ => None,
        })
        .unwrap_or(1.0)
}

/// Skill functions that change nothing while the main skill's level has no
/// cooldown (open items, S1). The engine maps no added cooldown (each such
/// stat is a reason, [`cooldown_limit`]), so without the level's cooldown
/// PoB's `output.Cooldown` is nil:
/// - Vortex's `preDamageFunc` (`act_int.lua` 20542 to 20544) sets
///   `hitTimeOverride = output.Cooldown`, which `CalcOffence.lua` 2403
///   reads only when set. Vortex's levels have no cooldown.
pub const READ_FUNCTIONS: [(&str, &str); 1] = [("Vortex", "preDamageFunc")];

/// Skill functions the engine reads in every part, with or without a
/// cooldown (agent list): Static Strike's `preDamageFunc` (`act_str.lua`
/// 10663 to 10667) sets the hit time of part 2 only, which
/// [`static_strike_beams`] reads; in part 1 it does nothing.
///
/// Perforate's (the agent list, 2026-11-27; `act_str.lua`): `dpsMultiplier
/// = max(dpsMultiplier or 1, 1)`, after its spike overlap
/// ([`perforate_dps_multiplier`]).
pub const READ_PART_FUNCTIONS: [(&str, &str); 4] = [
    ("Perforate", "preDamageFunc"),
    ("PerforateAltX", "preDamageFunc"),
    ("PerforateAltY", "preDamageFunc"),
    ("StaticStrike", "preDamageFunc"),
];

/// Perforate and its transfigured gems (the agent list, 2026-11-27): its
/// base mod `skill("dpsMultiplier", 1, { Multiplier PerforateSpikeOverlap,
/// limitVar <its spears> }, { Condition BloodStance })` (Perforate of
/// Bloodshed's without the condition): `(skill, Blood Stance only)`.
pub const PERFORATE: [(&str, bool); 3] = [
    ("Perforate", true),
    ("PerforateAltX", true),
    ("PerforateAltY", false),
];

/// The config's overlapping spikes (`ConfigOptions.lua` 555:
/// `Multiplier:PerforateSpikeOverlap` for Perforate and its transfigured
/// gems).
pub const PERFORATE_OVERLAP_INPUT: &str = "perforateSpikeOverlap";

/// Perforate's spears (its `blood_spears_base_number_of_spears`, PoB's
/// `Multiplier:<skill>MaxSpikes`).
const PERFORATE_SPEARS_STAT: &str = "blood_spears_base_number_of_spears";
/// Perforate of Duality's extra spears after a stance change (its
/// `blood_spears_additional_number_of_spears_if_changed_stance_recently`).
const PERFORATE_EXTRA_SPEARS_STAT: &str =
    "blood_spears_additional_number_of_spears_if_changed_stance_recently";

/// Perforate's `dpsMultiplier` ([`PERFORATE`]): `max(min(overlap, spears),
/// 1)` while its condition holds, the overlap the config's count (input,
/// else placeholder), the spears its own stats (Duality's extra with the
/// config's `changedStance`); 1 otherwise and for any other skill. A count
/// of another type is a reason.
fn perforate_dps_multiplier(
    input: &CalcInput<'_>,
    skill: &MainSkill<'_>,
    reasons: &mut Reasons,
) -> f64 {
    let id = skill.effect().id.as_str();
    let Some(&(_, blood_only)) = PERFORATE.iter().find(|(s, _)| *s == id) else {
        return 1.0;
    };
    let set = super::collect::active_config(input.build);
    let read = |name: &str, kind| set.and_then(|s| super::config::last_input(s, name, kind));
    if blood_only
        && matches!(
            read(super::gems::STANCE_INPUT, loom_domain::ConfigInputKind::Input),
            Some(loom_domain::ConfigValue::String(s)) if s == "SAND"
        )
    {
        return 1.0;
    }
    let overlap =
        match read(PERFORATE_OVERLAP_INPUT, loom_domain::ConfigInputKind::Input).or_else(|| {
            read(
                PERFORATE_OVERLAP_INPUT,
                loom_domain::ConfigInputKind::Placeholder,
            )
        }) {
            None => 0.0,
            Some(loom_domain::ConfigValue::Number(n)) => n.value().0,
            Some(_) => {
                reasons.partial.insert(UnsupportedReason::UnmodelledConfig {
                    input: PERFORATE_OVERLAP_INPUT.to_owned(),
                });
                return 1.0;
            }
        };
    let gem = skill.active_gem;
    let stats = skill::instance(skill.effect(), gem.level, gem.quality, input.data).stats;
    let stat = |name: &str| {
        #[expect(clippy::cast_precision_loss, reason = "a small stat value")]
        let v = stats
            .iter()
            .filter(|(s, _)| s == name)
            .map(|(_, v)| *v as f64)
            .sum::<f64>();
        v
    };
    let mut spears = stat(PERFORATE_SPEARS_STAT);
    if matches!(
        read("changedStance", loom_domain::ConfigInputKind::Input),
        Some(loom_domain::ConfigValue::Boolean(true))
    ) {
        spears += stat(PERFORATE_EXTRA_SPEARS_STAT);
    }
    crate::math::max(crate::math::min(overlap, spears), 1.0)
}

/// Skill functions that do nothing in one part (open items, S1): (skill,
/// function, part). Spark's `preDamageFunc` (`act_int.lua` 16721 to 16727)
/// multiplies the DPS by the hits per duration in part 2, "Maximum Hits",
/// only; in part 1, "1 Hit", it changes nothing. Part 2 keeps
/// `pob-skill-data`.
pub const NOOP_PART_FUNCTIONS: [(&str, &str, usize); 1] = [("Spark", "preDamageFunc", 1)];

/// Skill functions and own stats that reach no slice field (open items, S1):
/// Cyclone's and Vaal Cyclone's `initialFunc` adds `Multiplier:
/// AdditionalMeleeRange` (`act_dex.lua` 4701 to 4714, 4936 to 4945), which
/// only their `cyclone_area_of_effect_+%_per_additional_melee_range` reads
/// (`AreaOfEffect` INC per that multiplier), and `AreaOfEffect` feeds only
/// radius outputs (the mapping's `area` rows, P6-11dl). Cyclone of Tumult's
/// stage stats are not among them.
pub const NO_SLICE_FUNCTIONS: [(&str, &str); 2] =
    [("Cyclone", "initialFunc"), ("VaalCyclone", "initialFunc")];
/// See [`NO_SLICE_FUNCTIONS`].
///
/// Shock Nova's `shock_nova_ring_chance_to_shock_+%` (the agent list,
/// 2026-11-27): a chance to shock, which reaches no slice field (PoB never
/// shocks the enemy from a chance; the engine's shock is the config's and
/// the guaranteed ones).
pub const NO_SLICE_OWN_STATS: [(&str, &str); 6] = [
    (
        "Cyclone",
        "cyclone_area_of_effect_+%_per_additional_melee_range",
    ),
    // Elemental Hit of the Spectrum's explosion radius (`skill("radius")` in
    // part 2): radius outputs alone (the agent list, 2026-11-27).
    (
        "ElementalHitAltX",
        "active_skill_base_area_of_effect_radius",
    ),
    // Perforate's Sand Stance area: radius outputs alone.
    ("Perforate", "skill_area_of_effect_+%_final_in_sand_stance"),
    (
        "PerforateAltX",
        "skill_area_of_effect_+%_final_in_sand_stance",
    ),
    ("ShockNova", "shock_nova_ring_chance_to_shock_+%"),
    (
        "VaalCyclone",
        "cyclone_area_of_effect_+%_per_additional_melee_range",
    ),
];

/// The `SkillData` key PoB clears when a trigger sets the skill's rate
/// (`CalcOffence.lua` 2223 to 2227: `skillData.showAverage = false`), and
/// reads otherwise in many outputs (the combined DPS, leech, poison and
/// bleed stacks, impale; 333, 3926, 4014, 4767, 5535, 5983, 6131). Read only
/// for a triggered skill ([`pob_vetted_triggered`]; open items, S1).
pub const SHOW_AVERAGE_KEY: &str = "showAverage";

/// Static Strike's zap period, milliseconds (PoB's `repeatFrequency`, `div =
/// 1000`, `act_str.lua` 10682 to 10685).
pub const ZAP_FREQUENCY_STAT: &str = "static_strike_base_zap_frequency_ms";
/// Static Strike's zap speed, percent (PoB's `StaticStrikeFrequency` INC,
/// 10686 to 10688; its quality's).
pub const ZAP_SPEED_STAT: &str = "static_strike_zap_speed_+%";
/// Static Strike's less beam damage, percent (`Damage` MORE in part 2).
pub const BEAM_DAMAGE_STAT: &str = "static_strike_beam_damage_+%_final";

/// Stat map entries of any skill that only mark it triggered, which the
/// engine reads as the skill's trigger kind ([`super::triggers::kind`];
/// open items, agent list): `spell_uncastable_if_triggerable` and
/// `skill_has_trigger_from_unique_item` set `skillData.triggered` and
/// `triggeredByUnique` (`SkillStatMap.lua` 191 to 203). PoB reads them as
/// a triggered skill: its rate from the trigger (`CalcOffence.lua` 2205 to
/// 2227), no "attacked or cast a spell recently" from it
/// (`CalcPerform.lua` 240, [`super::conditions::main_skill_recently`]),
/// never a trigger's source (`CalcTriggers.lua` 41). The engine computes
/// such a skill through [`super::triggers`] alike.
pub const TRIGGER_OWN_STATS: [&str; 2] = [
    "skill_has_trigger_from_unique_item",
    "spell_uncastable_if_triggerable",
];

/// Untagged `FLAG` base mods that reach no slice field outside a guard
/// (open items, agent list):
/// - `CannotSplit` (Molten Burst's): read only for the split count
///   (`CalcOffence.lua` 1069), which no slice field holds;
/// - `OneShotProj` (Rain of Arrows'): read only with
///   `SequentialProjectiles` (769), whose one mapped source, Barrage's
///   `projectiles_barrage`, guards the projectile count
///   (`mapping::NO_EFFECT_DEPENDS_ON`).
/// - `DotCanStack` (Toxic Rain's; the agent list, 2026-11-27): read only
///   for the damage over time and the mirages' and full DPS's count of it
///   (`CalcOffence.lua` 5737, 6117; `Calcs.lua` 271), never the hit; the
///   DoT vets itself ([`super::dot::pob_dot_vetted`]).
pub const READ_BASE_FLAGS: [&str; 3] = ["CannotSplit", "DotCanStack", "OneShotProj"];

/// Base mods the engine reads by their name, untagged or not: (skill, mod
/// name). Elemental Hit of the Spectrum's
/// `Multiplier:ElementalHitAilmentOnEnemy` 1 per enemy ailment
/// (`ActorCondition` tags): [`super::stacks::enemy_elemental_ailments`]
/// (the agent list, 2026-11-27).
pub const READ_BASE_MODS: [(&str, &str); 1] =
    [("ElementalHitAltX", "Multiplier:ElementalHitAilmentOnEnemy")];

/// Skill functions that do nothing unless a config input sets the skill
/// data they read (the agent list, 2026-11-27): (skill, function, input).
/// Toxic Rain's `preDamageFunc` (`act_dex.lua`, `ToxicRain` and its
/// transfigured gems) sets `dpsMultiplier = min(podOverlapMultiplier or
/// 1, ProjectileCount)`, 1 without the config's `toxicRainPodOverlap`
/// (`ConfigOptions.lua` 700 to 702). Vetted; the attack keeps
/// `pob-skill-data` when the input is set ([`config_noop_set`]).
pub const CONFIG_NOOP_FUNCTIONS: [(&str, &str, &str); 3] = [
    ("ToxicRain", "preDamageFunc", "toxicRainPodOverlap"),
    ("ToxicRainAltX", "preDamageFunc", "toxicRainPodOverlap"),
    ("ToxicRainAltY", "preDamageFunc", "toxicRainPodOverlap"),
];

/// Whether the build sets the input of one of `id`'s
/// [`CONFIG_NOOP_FUNCTIONS`].
fn config_noop_set(input: &CalcInput<'_>, id: &str) -> bool {
    let Some(set) = super::collect::active_config(input.build) else {
        return false;
    };
    CONFIG_NOOP_FUNCTIONS.iter().any(|(s, _, name)| {
        *s == id
            && super::config::last_input(set, name, loom_domain::ConfigInputKind::Input).is_some()
    })
}

/// [`pob_vetted`], where `no_cooldown` says the main skill's level has no
/// cooldown: only then a function of [`READ_FUNCTIONS`] counts as read.
#[must_use]
pub fn pob_vetted_with(id: &str, index: usize, no_cooldown: bool) -> bool {
    vetted(id, index, no_cooldown, false)
}

/// [`pob_vetted_with`] for a skill a trigger sets the rate of: PoB then
/// clears its own `showAverage` ([`SHOW_AVERAGE_KEY`]), so that base mod
/// counts as read (open items, S1; Icicle Burst, which Cameria's Avarice
/// triggers).
///
/// Only PoB's `triggerRate` branch clears it (`CalcOffence.lua` 2223 to
/// 2227); its `triggerTime` branch just above (2214 to 2220) does not. The
/// engine reaches `triggerTime` only for a skill Cast on Death triggers
/// (`CalcPerform.lua` 1447), which the spell path excludes, and for Cast
/// while Channelling's channelled source, which is never the triggered main
/// skill; every other ported trigger gives a rate (review of S1).
#[must_use]
pub fn pob_vetted_triggered(id: &str, index: usize, no_cooldown: bool) -> bool {
    vetted(id, index, no_cooldown, true)
}

/// The vetting ([`pob_vetted_with`]); `triggered`: [`pob_vetted_triggered`].
fn vetted(id: &str, index: usize, no_cooldown: bool, triggered: bool) -> bool {
    let Some(pob) = super::pob_skill::skill(id) else {
        return false;
    };
    let function_read = |f: &&str| {
        (no_cooldown && READ_FUNCTIONS.contains(&(id, *f)))
            || READ_PART_FUNCTIONS.contains(&(id, *f))
            || NOOP_PART_FUNCTIONS.contains(&(id, *f, index))
            || NO_SLICE_FUNCTIONS.contains(&(id, *f))
            || CONFIG_NOOP_FUNCTIONS
                .iter()
                .any(|(s, func, _)| *s == id && func == f)
    };
    if !pob.functions.iter().all(function_read) {
        return false;
    }
    if !pob.own_stats.iter().all(|s| {
        READ_OWN_STATS.contains(&(id, *s))
            || TRIGGER_OWN_STATS.contains(s)
            || NO_SLICE_OWN_STATS.contains(&(id, *s))
    }) {
        return false;
    }
    // A channelled release skill: PoB sets its hit time outside its data
    // (Snipe's `hitTimeMultiplier`, `CalcTriggers.lua` 1405 to 1418), which
    // the engine does not read (P6-5 review).
    if pob.flag("channelRelease") {
        return false;
    }
    // Display data, or a DoT's flags (`dotIsArea` and kin), which PoB reads
    // only in the DoT's config (`CalcOffence.lua` 5657 to 5669) and the DoT
    // vets itself (`dot::DOT_SKILL_DATA_KEYS`; open items, S1).
    // A radius is display data tagged or not (the agent list, 2026-11-27:
    // Perforate's radius per stance, tagged with the stance's condition).
    let display = |m: &super::pob_skill::PobBaseMod| {
        m.name == "SkillData"
            && (!m.tagged || m.key.is_some_and(|k| DISPLAY_SKILL_DATA.contains(&k)))
            && m.key.is_some_and(|k| {
                DISPLAY_SKILL_DATA.contains(&k)
                    || super::dot::DOT_SKILL_DATA_KEYS.contains(&k)
                    || (triggered && k == SHOW_AVERAGE_KEY)
            })
    };
    // A part's own base mod the engine reads ([`READ_PART_BASE_MODS`]).
    let other_part = |m: &super::pob_skill::PobBaseMod| {
        m.name == "SkillData"
            && m.tagged
            && m.key.is_some_and(|k| {
                READ_PART_BASE_MODS
                    .iter()
                    .any(|&(s, key, _)| s == id && key == k)
            })
    };
    // A flag no slice field reads ([`READ_BASE_FLAGS`]).
    let read_flag = |m: &super::pob_skill::PobBaseMod| {
        m.kind == "FLAG" && !m.tagged && READ_BASE_FLAGS.contains(&m.name)
    };
    let read_mod = |m: &super::pob_skill::PobBaseMod| READ_BASE_MODS.contains(&(id, m.name));
    if !pob
        .base_mods
        .iter()
        .all(|m| display(m) || other_part(m) || read_flag(m) || read_mod(m))
    {
        return false;
    }
    match index.checked_sub(1).and_then(|i| pob.parts.get(i)) {
        Some(part) => part.values.iter().all(|(k, _)| PART_FIELDS.contains(k)),
        None => true,
    }
}

/// "Hits with both weapons" while dual wielding (PoB's
/// `doubleHitsWhenDualWielding`): read from the skill's gems by
/// [`attack_skill`], so the mapping gives it no effect.
pub const DOUBLE_HITS_STAT: &str = "skill_double_hits_when_dual_wielding";

/// PoB's `doubleHitsWhenDualWielding` for `skill`: the skill's data
/// (Swordstorm's) or the stat [`DOUBLE_HITS_STAT`] of the skill or a
/// support (`SkillStatMap.lua` 291).
pub(super) fn double_hits(skill: &MainSkill<'_>, data: &dyn CalcData) -> bool {
    super::pob_skill::skill(&skill.effect().id)
        .and_then(|p| p.skill_data("doubleHitsWhenDualWielding"))
        .is_some_and(super::pob_skill::Value::truthy)
        || skill::instances(skill, data).iter().any(|i| {
            i.stats
                .iter()
                .any(|(s, v)| s == DOUBLE_HITS_STAT && *v != 0)
        })
}

/// PoB's `skillFlag = "arrow"` (`SkillStatMap.lua` 1341): the skill's
/// keyword `KeywordFlag.Arrow` (`CalcActiveSkill.lua` 412; P6-11c), from the
/// active gem's own stats only (PoB reads `skillFlag` entries of the active
/// effect's stat map, 341 to 346; a support's flags come from `addFlags`;
/// P6-11c review).
pub const ARROW_STAT: &str = "skill_can_fire_arrows";

/// PoB's damage types in its conversion order (`dmgTypeList`), the order
/// it sums each pass in.
pub(super) const TYPES: [DamageType; 5] = [
    DamageType::Physical,
    DamageType::Lightning,
    DamageType::Cold,
    DamageType::Fire,
    DamageType::Chaos,
];

/// PoB's `data.misc.MaxResistCap`.
const MAX_RESIST_CAP: f64 = 90.0;
/// PoB's `data.misc.ResistFloor`.
const RESIST_FLOOR: f64 = -200.0;
/// PoB's `CritChanceCap` BASE 100 (`Modules/CalcSetup.lua`).
const CRIT_CHANCE_CAP: f64 = 100.0;
/// A spell's chance to hit: PoB's `AccuracyHitChance` is 100 for a
/// spell, and nothing modelled lets the enemy block it.
const SPELL_HIT_CHANCE: f64 = 100.0;
/// PoB's `data.misc.ServerTickTime`; the rate cap is `1 / 0.033` times
/// the repeat count ([`super::repeats`]).
const SERVER_TICK_TIME: f64 = 0.033;
/// PoB's `data.misc.AccuracyPerDexBase`: accuracy rating per dexterity.
const ACCURACY_PER_DEXTERITY: f64 = 2.0;

/// "Gain Accuracy Rating equal to twice your Strength" (P6-11gm).
const ACCURACY_PER_STRENGTH: f64 = 2.0;

/// The enemy's base maximum resistance (PoB's `data.misc.EnemyMaxResist`,
/// the monster template's).
const ENEMY_MAX_RESIST_STAT: &str = "base_maximum_all_resistances_%";
/// The enemy's largest physical damage reduction (PoB's
/// `data.misc.EnemyPhysicalDamageReductionCap`, the monster template's).
const ENEMY_MAX_REDUCTION_STAT: &str = "maximum_physical_damage_reduction_%";
/// The character's base critical strike multiplier (PoB's
/// `CritMultiplier` BASE `base_critical_strike_multiplier - 100`).
const CRIT_MULTIPLIER_STAT: &str = "base_critical_strike_multiplier";
/// The character's accuracy rating per level (PoB's `Accuracy` BASE
/// `accuracy_rating_per_level` per level above 1).
const ACCURACY_PER_LEVEL_STAT: &str = "accuracy_rating_per_level";

/// The main skill as a self-cast spell hit (module docs).
#[derive(Debug, Clone)]
pub struct SpellSkill<'d> {
    /// The main skill.
    pub skill: MainSkill<'d>,
    /// The main group's index among the active set's groups (0-based, as
    /// [`Source::Gem`] counts).
    pub group: usize,
    /// The active effect's stat set level row (crit, effectiveness).
    pub level: &'d StatSetLevel,
    /// Cast time, milliseconds.
    pub cast_time: i64,
    /// The level's cooldown, milliseconds (0: none; P6-7c3b).
    pub cooldown: i64,
    /// The level's stored uses (0: none).
    pub stored_uses: i64,
    /// Whether its hits are area damage (PoB's flags and part, else
    /// `is_area_damage`).
    pub area: bool,
    /// Whether its hits are projectiles (PoB's flags and part; P6-6b).
    pub projectile: bool,
    /// Whether its hits are melee (PoB's flags and part; P6-6c review).
    pub melee: bool,
    /// Whether the engine reads all of PoB's data of the skill
    /// ([`pob_vetted`]).
    pub vetted: bool,
    /// Whether it does when a trigger sets its rate
    /// ([`pob_vetted_triggered`]: its own `showAverage` read).
    pub vetted_triggered: bool,
    /// Its part's `dpsMultiplier` ([`part_dps_multiplier`]).
    pub dps_multiplier: f64,
}

impl SpellSkill<'_> {
    /// The active gem's source.
    #[must_use]
    pub const fn source(&self) -> Source {
        Source::Gem {
            group: self.group,
            gem: self.skill.active_gem.gem,
        }
    }
}

/// The main skill as an attack with the weapon in the main hand (module
/// docs).
#[derive(Debug, Clone)]
pub struct AttackSkill<'d> {
    /// The main skill.
    pub skill: MainSkill<'d>,
    /// The main group's index among the active set's groups (0-based).
    pub group: usize,
    /// The active effect's stat set level row (crit, effectiveness).
    pub level: &'d StatSetLevel,
    /// The level's attack speed multiplier, percent.
    pub attack_speed_multiplier: i64,
    /// The weapon in the main hand.
    pub weapon: ItemId,
    /// The main hand's weapon flags ([`weapon_type_flags`], P6-11b).
    pub weapon_flags: Flags,
    /// The weapon in the off hand when the skill attacks with it too
    /// (Phase 6, dual wielding: PoB's `weapon2Attack`).
    pub off_weapon: Option<ItemId>,
    /// The off hand's weapon flags when it attacks (P6-11b).
    pub off_weapon_flags: Flags,
    /// The skill hits with both weapons at once (PoB's skill data
    /// `doubleHitsWhenDualWielding`); else it alternates.
    pub double_hits: bool,
    /// The skill fires arrows ([`ARROW_STAT`]).
    pub arrow: bool,
    /// The skill part computed.
    pub part: SkillPart,
    /// The part's index in PoB's data, 1-based; 0 without PoB's parts.
    pub part_index: usize,
    /// Whether the engine reads all of PoB's data of the skill
    /// ([`pob_vetted`]); without PoB's data, the part's flags are the
    /// game's (`Melee` type, `is_area_damage`).
    pub known_parts: bool,
    /// A shield attack (P6-11he; PoB's `shieldAttack`): the level's attack
    /// time in milliseconds; `weapon` is then the shield, the attack's only
    /// hand the off hand.
    pub shield: Option<i64>,
}

impl AttackSkill<'_> {
    /// The active gem's source.
    #[must_use]
    pub const fn source(&self) -> Source {
        Source::Gem {
            group: self.group,
            gem: self.skill.active_gem.gem,
        }
    }
}

/// The main skill's hit the engine computes.
#[derive(Debug, Clone)]
pub enum HitSkill<'d> {
    /// A self-cast spell.
    Spell(SpellSkill<'d>),
    /// An attack with the weapon in the main hand.
    Attack(AttackSkill<'d>),
}

impl<'d> HitSkill<'d> {
    /// The main skill.
    #[must_use]
    pub const fn skill(&self) -> &MainSkill<'d> {
        match self {
            Self::Spell(s) => &s.skill,
            Self::Attack(a) => &a.skill,
        }
    }

    /// The main group's index (0-based).
    #[must_use]
    pub const fn group(&self) -> usize {
        match self {
            Self::Spell(s) => s.group,
            Self::Attack(a) => a.group,
        }
    }
}

/// The build's main skill when the engine computes its hit as a self-cast
/// spell (module docs), else `None`.
#[must_use]
pub fn spell_skill<'d>(
    build: &Build,
    data: &'d dyn CalcData,
    levels: &GemLevels,
) -> Option<SpellSkill<'d>> {
    let skill = skill::main_skill(build, data, levels).ok()?;
    let group = skill.group.checked_sub(1)?;
    if !skill::active_groups(build).ok()?.get(group)?.enabled {
        return None;
    }
    let has = |t: &str| skill.types.contains(t);
    // A triggered spell is computed when the engine ports its trigger
    // (P6-8a, [`super::triggers`]): its rate is the trigger's.
    let triggered = super::triggers::computable(&skill);
    if !has("Spell")
        || !(has("Damage") || has("Projectile"))
        || NOT_SELF_CAST
            .iter()
            .any(|t| has(t) && !(*t == "Triggered" && triggered))
    {
        return None;
    }
    let effect = skill.effect();
    let gem = skill.active_gem;
    let level_row = skill::effect_level(effect, gem.level)?;
    // A cooldown caps the rate (P6-7c3b, [`cast_rate`]). A trap's is PoB's
    // `TrapCooldown` (P6-11hp; `CalcOffence.lua` 1259 to 1267), which caps
    // nothing: the rate is the throwing rate. A mine's that is not a trap
    // is PoB's `output.Cooldown` (1286), which would cap the laying rate:
    // not modelled.
    if effect.cast_time <= 0 || (level_row.cooldown != 0 && has("RemoteMined") && !has("Trapped")) {
        return None;
    }
    let level = skill::stat_set_level(effect, gem.level, data)?;
    // PoB's flags and part (P6-6b), else the game's area stat.
    let wanted = skill::active_groups(build)
        .ok()
        .and_then(|groups| groups.get(group).cloned())
        .and_then(|g| {
            g.gems
                .typed()
                .nth(gem.gem)
                .and_then(|x| skill::attribute(&x.attributes, "skillPart"))
                .and_then(|v| v.parse::<usize>().ok())
        })
        .unwrap_or(1);
    if wanted == 0 && super::pob_skill::skill(&effect.id).is_some_and(|p| !p.parts.is_empty()) {
        return None;
    }
    let (part, part_index, known) = match pob_part(&effect.id, wanted) {
        Some((p, i)) => (p, i, true),
        None => (
            SkillPart {
                name: "",
                melee: false,
                area: is_area_damage(&skill, data),
                projectile: false,
            },
            0,
            false,
        ),
    };
    Some(SpellSkill {
        skill,
        group,
        level,
        cast_time: effect.cast_time,
        cooldown: level_row.cooldown,
        stored_uses: level_row.stored_uses,
        area: part.area,
        projectile: part.projectile,
        melee: part.melee,
        vetted: known && pob_vetted_with(&effect.id, part_index, level_row.cooldown == 0),
        vetted_triggered: known
            && pob_vetted_triggered(&effect.id, part_index, level_row.cooldown == 0),
        dps_multiplier: part_dps_multiplier(&effect.id, part_index),
    })
}

/// Whether the main skill's own stats make its hits area damage.
fn is_area_damage(skill: &MainSkill<'_>, data: &dyn CalcData) -> bool {
    let gem = skill.active_gem;
    skill::instance(skill.effect(), gem.level, gem.quality, data)
        .stats
        .iter()
        .any(|(s, v)| s == "is_area_damage" && *v != 0)
}

/// The build's main skill when the engine computes its hit as an attack
/// with the weapon in the main hand (module docs), else `None`.
#[must_use]
pub fn attack_skill<'d>(input: &CalcInput<'d>, levels: &GemLevels) -> Option<AttackSkill<'d>> {
    let (build, data) = (input.build, input.data);
    let skill = skill::main_skill(build, data, levels).ok()?;
    let group = skill.group.checked_sub(1)?;
    let groups = skill::active_groups(build).ok()?;
    let main_group = groups.get(group)?;
    if !main_group.enabled {
        return None;
    }
    let has = |t: &str| skill.types.contains(t);
    // A triggered attack is computed when the engine ports its trigger
    // (open items S3): its rate is the trigger's ([`attack_hit`]).
    let triggered = super::triggers::computable(&skill);
    if !has("Attack")
        || NOT_PLAIN_ATTACK
            .iter()
            .any(|t| has(t) && !(*t == "Triggered" && triggered))
    {
        return None;
    }
    let effect = skill.effect();
    let gem = skill.active_gem;
    let level_row = skill::effect_level(effect, gem.level)?;
    // A cooldown caps a used attack's rate (not modelled); a triggered
    // attack's rate is its trigger's, which reads the cooldown (open items
    // S3, part 7: the skills items grant and trigger).
    let ported = matches!(
        super::triggers::kind(&skill),
        super::triggers::Kind::Ported(..)
    );
    if level_row.cooldown != 0 && !ported {
        return None;
    }
    let level = skill::stat_set_level(effect, gem.level, data)?;
    // A shield attack (P6-11he; PoB's `shieldAttack` flag,
    // `CalcActiveSkill.lua` 269 to 279): the skill needs a shield and
    // attacks with the off hand only, its weapon flags 0, its weapon the
    // skill's own values ([`shield_weapon_data`]); the main hand's weapon
    // does not decide between melee and projectile.
    if super::pob_skill::skill(&effect.id).is_some_and(|p| p.flags.contains(&"shieldAttack")) {
        let shield = collect::shield(input)?;
        let (part, part_index, known_parts) = attack_part(&skill, main_group, data)?;
        let arrow = skill::instance(effect, gem.level, gem.quality, data)
            .stats
            .iter()
            .any(|(s, v)| s == ARROW_STAT && *v != 0);
        return Some(AttackSkill {
            skill,
            group,
            level,
            attack_speed_multiplier: level_row.attack_speed_multiplier,
            weapon: shield,
            weapon_flags: Flags::NONE,
            off_weapon: None,
            off_weapon_flags: Flags::NONE,
            double_hits: false,
            arrow,
            part,
            part_index,
            known_parts,
            shield: Some(level_row.attack_time),
        });
    }
    let hands = collect::hands(input);
    let (weapon, base) = hands.main.clone()?;
    // PoB disables a skill its weapon cannot use (`weaponTypes`,
    // [`MainSkill::can_use`]).
    let usable = |class: &str| skill.can_use(class);
    if !usable(&base.class) {
        return None;
    }
    // The off hand (Phase 6, `CalcActiveSkill.lua` 318 to 335): a weapon
    // the skill can use attacks too; one it cannot use disables the skill
    // (PoB: "Off Hand weapon is not usable with this skill"), so the attack
    // is not computed. An item there whose base is not known leaves it not
    // computed too. The skill types that require dual wielding or two
    // different weapon types, and supports' weapon types, are not checked:
    // only vetted attacks are `Complete` (`VETTED_ATTACKS`), and none has
    // them.
    let main_flags = weapon_type_flags(&base.class)?;
    let (off_weapon, off_weapon_flags) = match hands.off {
        Some((id, off)) if usable(&off.class) => (Some(id), weapon_type_flags(&off.class)?),
        Some(_) => return None,
        None if hands.off_hand_weapon => return None,
        None => (None, Flags::NONE),
    };
    let double_hits = double_hits(&skill, data);
    let arrow = skill::instance(effect, gem.level, gem.quality, data)
        .stats
        .iter()
        .any(|(s, v)| s == ARROW_STAT && *v != 0);
    let (mut part, part_index, known_parts) = attack_part(&skill, main_group, data)?;
    // The main-hand weapon decides between melee and projectile
    // (`CalcActiveSkill.lua` 302 to 308): a melee weapon clears the
    // projectile flag, a ranged one (bow, wand: not melee in PoB's
    // `weaponTypeInfo`) the melee flag (P6-6b review).
    (part.melee, part.projectile) = weapon_flags(&base.class, part.melee, part.projectile);
    Some(AttackSkill {
        skill,
        group,
        level,
        attack_speed_multiplier: level_row.attack_speed_multiplier,
        weapon,
        weapon_flags: main_flags,
        off_weapon,
        off_weapon_flags,
        double_hits,
        arrow,
        part,
        part_index,
        known_parts,
        shield: None,
    })
}

/// The attack's part (PoB's `skillPart`, P6-6b), its index (1-based, 0
/// without PoB's parts), and whether PoB's data of it is read in full;
/// `None` for part 0 of a skill with parts.
fn attack_part(
    skill: &MainSkill<'_>,
    main_group: &SocketGroup,
    data: &dyn CalcData,
) -> Option<(SkillPart, usize, bool)> {
    let effect = skill.effect();
    let gem = skill.active_gem;
    let has = |t: &str| skill.types.contains(t);
    // PoB: `m_min(#parts, skillPart or 1)`; an empty or missing attribute
    // is 1 (P6-6b: PoB's parts and flags from its skill data).
    let wanted = main_group
        .gems
        .typed()
        .nth(gem.gem)
        .and_then(|g| skill::attribute(&g.attributes, "skillPart"))
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1);
    // Part 0: PoB would index a part that does not exist.
    if wanted == 0 && super::pob_skill::skill(&effect.id).is_some_and(|p| !p.parts.is_empty()) {
        return None;
    }
    Some(match pob_part(&effect.id, wanted) {
        Some((p, index)) => (p, index, pob_vetted(&effect.id, index)),
        None => (
            SkillPart {
                name: "",
                melee: has("Melee"),
                area: is_area_damage(skill, data),
                projectile: false,
            },
            0,
            false,
        ),
    })
}

/// The main skill's hit the engine computes: a self-cast spell, else an
/// attack (module docs), its gems at the levels `levels` gives them
/// (P3-8i).
#[must_use]
pub fn hit_skill<'d>(input: &CalcInput<'d>, levels: &GemLevels) -> Option<HitSkill<'d>> {
    spell_skill(input.build, input.data, levels)
        .map(HitSkill::Spell)
        .or_else(|| attack_skill(input, levels).map(HitSkill::Attack))
}

/// A leaf with the value of the skill (or the character) itself.
fn leaf(
    calc: &mut Calc<'_>,
    source: Source,
    quantity: Quantity,
    value: f64,
) -> Result<Valued, CalcError> {
    let node = calc.graph_mut().push(
        Node::Contribution {
            source,
            quantity,
            op: Op::Base,
            value,
        },
        value,
    )?;
    Ok(Valued { node, value })
}

/// `a / b` with `term`.
fn quotient(calc: &mut Calc<'_>, a: Valued, b: Valued, term: Term) -> Result<Valued, CalcError> {
    let value = a.value / b.value;
    let node = calc.graph_mut().push(
        Node::Quotient {
            term,
            a: a.node,
            b: b.node,
        },
        value,
    )?;
    Ok(Valued { node, value })
}

/// `a - b` with `term`.
fn difference(calc: &mut Calc<'_>, a: Valued, b: Valued, term: Term) -> Result<Valued, CalcError> {
    let value = a.value - b.value;
    let node = calc.graph_mut().push(
        Node::Difference {
            term,
            a: a.node,
            b: b.node,
        },
        value,
    )?;
    Ok(Valued { node, value })
}

/// `base ^ exponent` with `term`.
fn power(
    calc: &mut Calc<'_>,
    base: Valued,
    exponent: Valued,
    term: Term,
) -> Result<Valued, CalcError> {
    let value = crate::math::pow(base.value, exponent.value);
    let node = calc.graph_mut().push(
        Node::Power {
            term,
            base: base.node,
            exponent: exponent.node,
        },
        value,
    )?;
    Ok(Valued { node, value })
}

/// `floor(x)` with `term`.
fn floor(calc: &mut Calc<'_>, x: Valued, term: Term) -> Result<Valued, CalcError> {
    let value = crate::math::floor(x.value);
    let node = calc
        .graph_mut()
        .push(Node::Floor { term, of: x.node }, value)?;
    Ok(Valued { node, value })
}

/// `x / 100` with `term`.
fn percent(calc: &mut Calc<'_>, x: Valued, term: Term) -> Result<Valued, CalcError> {
    let hundred = phases::constant(calc, "hundred", 100.0)?;
    quotient(calc, x, hundred, term)
}

/// PoB's `round(x, 2)`: `floor(x x 100 + 0.5) / 100`.
fn round_2(calc: &mut Calc<'_>, x: Valued, term: Term) -> Result<Valued, CalcError> {
    let hundred = phases::constant(calc, "hundred", 100.0)?;
    let scaled = phases::product(calc, &[x, hundred], Term::Step("round-scaled"))?;
    let rounded = phases::round(calc, scaled, Term::Step("round-whole"))?;
    quotient(calc, rounded, hundred, term)
}

/// The sum of `op` effects on `quantity` in `cx`, with `term`.
fn sum_in(
    calc: &mut Calc<'_>,
    quantity: Quantity,
    op: Op,
    cx: &QueryContext,
    term: Term,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let (store, graph) = calc.parts();
    let q = store.sum(graph, quantity, op, cx, term)?;
    reasons.extend(&q.reasons);
    Ok(Valued {
        node: q.node,
        value: q.value,
    })
}

/// The flag on `quantity` in `cx` when one applies (P3-8m): its node,
/// value 1. A flag whose sum is 0 is off: every flag read here has a
/// description shown only for a value of at least 1 (P3-8n: a flag shown
/// for any value is read by presence, as Convergence's in `poe1::buffs`). Queried only when the store has an effect on `quantity`, so a
/// hit without one keeps its graph.
fn flag_on(
    calc: &mut Calc<'_>,
    quantity: Quantity,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    if !phases::has(calc, quantity) {
        return Ok(None);
    }
    let (store, graph) = calc.parts();
    let q = store.flag(graph, quantity, cx)?;
    reasons.extend(&q.reasons);
    Ok((q.value != 0.0).then_some(Valued {
        node: q.node,
        value: q.value,
    }))
}

/// A field's value set by a flag (P3-8m): the flag times `value`, so the
/// value's node reads the flag's sources.
fn by_flag(
    calc: &mut Calc<'_>,
    flag: Valued,
    name: &'static str,
    value: f64,
) -> Result<Valued, CalcError> {
    let constant = phases::constant(calc, name, value)?;
    phases::product(calc, &[flag, constant], Term::Result)
}

/// `1 + inc / 100` of `quantity` in `cx`.
fn increased(
    calc: &mut Calc<'_>,
    quantity: Quantity,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let (store, graph) = calc.parts();
    let inc = store.increased(graph, quantity, cx)?;
    reasons.extend(&inc.reasons);
    Ok(Valued {
        node: inc.node,
        value: inc.value,
    })
}

/// `1 + inc / 100` and the more product ([`more`]) of `quantity` in `cx`.
fn modifiers(
    calc: &mut Calc<'_>,
    quantity: Quantity,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<(Valued, Valued), CalcError> {
    let inc = increased(calc, quantity, cx, reasons)?;
    let more = more(calc, quantity, cx, reasons)?;
    Ok((inc, more))
}

/// The PoB mod name an effect on `Damage` stands for, as its position in
/// the order PoB reads the names of a hit (`damageStatsForTypes`:
/// `Damage`, then `PhysicalDamage`, `LightningDamage`, `ColdDamage`,
/// `FireDamage`, `ElementalDamage`, `ChaosDamage`); 0 for other
/// quantities, which have one name.
fn name_order(e: &Effect) -> usize {
    if e.quantity != Quantity::Damage {
        return 0;
    }
    if e.conditions.any == Flags::ELEMENTAL {
        return 5;
    }
    let order = [
        (Flags::PHYSICAL, 1),
        (Flags::LIGHTNING, 2),
        (Flags::COLD, 3),
        (Flags::FIRE, 4),
        (Flags::CHAOS, 6),
    ];
    order
        .iter()
        .find(|(f, _)| e.conditions.all.contains(*f))
        .map_or(0, |(_, n)| *n)
}

/// PoB's more multiplier (`ModList:MoreInternal`, "The more multipliers for
/// each mod are computed to the nearest percent then applied"): per list
/// (the skill's own mods, its gems'; then the character's, the parent
/// list) and per mod name, the product of `1 + value / 100` rounded to two
/// decimals (`round(modResult, 2)`); the rounded factors multiplied in name
/// order, the skill's list times the character's. Effects of one name and
/// list multiply in the store's canonical order (PoB: the list's), the same
/// bits for one effect and after the rounding in practice.
fn more(
    calc: &mut Calc<'_>,
    quantity: Quantity,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    // (skill list?, name order) -> effects.
    let mut groups: BTreeMap<(bool, usize), Vec<Effect>> = BTreeMap::new();
    // The repeats' averaged damage (P6-11aj): its own group, after the
    // names.
    let repeat = Quantity::Poe1(Poe1Quantity::RepeatAverageDamage);
    for e in calc.store().effects() {
        let derived = quantity == Quantity::Damage && e.quantity == repeat;
        if (e.quantity != quantity && !derived) || e.op != Op::More {
            continue;
        }
        match e.conditions.applies(cx) {
            Applies::Yes => {}
            Applies::No => continue,
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
                continue;
            }
        }
        // The skill's own list: its active gem and the supports that apply
        // (`Calc::set_skill_sources`); another gem's buff is the
        // character's (PoB's `modDB`; close-out).
        let skill = calc.is_skill_source(&e.source);
        let order = if derived { 7 } else { name_order(e) };
        groups.entry((!skill, order)).or_default().push(e.clone());
    }
    let mut lists: [Vec<Valued>; 2] = [Vec::new(), Vec::new()];
    for ((character, _), mut effects) in groups {
        effects.sort_by(|a, b| {
            a.source
                .cmp(&b.source)
                .then_with(|| a.value.total_cmp(&b.value))
        });
        let mut factors = Vec::new();
        for e in effects {
            let node = calc.graph_mut().push(
                Node::Contribution {
                    source: e.source,
                    quantity: e.quantity,
                    op: e.op,
                    value: e.value,
                },
                e.value,
            )?;
            factors.push(phases::factor(
                calc,
                Valued {
                    node,
                    value: e.value,
                },
                Term::Step("more-factor"),
            )?);
        }
        let product = phases::product(calc, &factors, Term::Step("more-name"))?;
        let rounded = round_2(calc, product, Term::Step("more-name-rounded"))?;
        lists[usize::from(character)].push(rounded);
    }
    let [skill, character] = lists;
    let skill = phases::product(calc, &skill, Term::Step("more-skill"))?;
    let character = phases::product(calc, &character, Term::Step("more-character"))?;
    phases::product(calc, &[skill, character], Term::More)
}

/// The hit phase (module docs). A main skill whose hit the engine does
/// not compute leaves the hit fields not computed.
///
/// # Errors
/// A graph error.
pub fn hit(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let input = *calc.input();
    // The gem level modifiers as `collect` read them: every effect is in
    // the store by now.
    let levels = GemLevels::from_effects(calc.store().effects());
    // Righteous Fire's threshold (`poe1::gems`, close-out): unreserved life
    // at least 2 and mana at least 1, PoB's `StatThreshold` tags; unknown
    // (a reason through the rows' condition) when either is not computed.
    let life = calc.field(Field::LifeUnreserved).and_then(|f| f.value);
    let mana = calc.field(Field::Mana).and_then(|f| f.value);
    if let (Some(life), Some(mana)) = (life, mana) {
        calc.settle_state(
            ConditionKey::new(super::gems::STAT_THRESHOLD),
            life >= 2.0 && mana >= 1.0,
        );
    }
    if let Some(life) = life {
        calc.settle_state(ConditionKey::new(super::gems::LIFE_THRESHOLD), life >= 2.0);
    }
    let skill = hit_skill(&input, &levels);
    if let Some(h) = &skill {
        let sources = std::iter::once(&h.skill().active_gem)
            .chain(&h.skill().supports)
            .map(|r| Source::Gem {
                group: h.group(),
                gem: r.gem,
            })
            .collect();
        calc.set_skill_sources(sources);
    }
    match skill {
        None => Ok(()),
        Some(HitSkill::Spell(spell)) => spell_hit(calc, &spell),
        Some(HitSkill::Attack(attack)) => attack_hit(calc, &attack),
    }
}

/// A self-cast spell's hit.
pub(super) fn spell_hit(calc: &mut Calc<'_>, spell: &SpellSkill<'_>) -> Result<(), CalcError> {
    let input = *calc.input();
    let enemy = enemy::enemy(input.build, input.data);
    // PoB's spell flags (P6-11hp; `CalcActiveSkill.lua` 353 to 359, 427):
    // `ModFlag.Spell` with the base flag `spell`, the spell keyword for a
    // Spell type without the base flag `cast` (Bear Trap has neither: spell
    // modifiers do not reach its damage). A skill without PoB's data keeps
    // both.
    let pob = super::pob_skill::skill(&spell.skill.effect().id);
    let mut flags = Flags::CAST | Flags::HIT | spell.skill.keyword_flags();
    if pob.is_none_or(|p| p.flag("spell")) {
        flags = flags | Flags::SPELL;
    }
    if pob.is_none_or(|p| !p.flag("cast")) {
        flags = flags | Flags::KEYWORD_SPELL;
    }
    if spell.area {
        flags = flags | Flags::AREA;
    }
    // Melee, else projectile (`CalcActiveSkill.lua` 361 to 366).
    if spell.melee {
        flags = flags | Flags::MELEE;
    } else if spell.projectile {
        flags = flags | Flags::PROJECTILE;
    }
    // Arcanist Brand's support part adds the `brand` skill flag, so PoB's
    // `KeywordFlag.Brand` (`act_int.lua` 631, `CalcActiveSkill.lua` 409; open
    // items S3). The engine's `SKILL_BRAND` stands for the keyword and the
    // `SkillType.Brand` tag; the one tag row (cast speed with brand skills)
    // reaches only the cast rate, which a triggered spell does not read.
    if super::triggers::brand_triggered(&spell.skill) {
        flags = flags | Flags::SKILL_BRAND;
    }
    // The trigger before the damage (P6-8a): whether PoB turns the skill's
    // `TriggeredDamage` into `Damage` is settled first. Its cooldown
    // recovery reads the skill's context (open items S3).
    let trigger = super::triggers::rate(calc, &spell.skill, &spell.source(), flags);
    let triggered_damage = matches!(trigger, super::triggers::Outcome::Rate(_, _, true));
    calc.settle_state(
        ConditionKey::new(super::triggers::TRIGGERED_DAMAGE),
        triggered_damage,
    );
    // Arcanist Brand's more damage against the branded enemy (open items
    // S3): with the triggered damage and the config's check. A check of
    // another type leaves it unsettled (the state row's reason).
    if let Some(targeting) = super::triggers::targeting_branded_enemy(&input) {
        calc.settle_state(
            ConditionKey::new(super::triggers::TRIGGERED_DAMAGE_BRANDED),
            triggered_damage && targeting,
        );
    }
    // PoB's `showAverage` from a gem's base mods (Spellslinger's support,
    // open items S3; `CalcOffence.lua` 333), unless a trigger sets the
    // rate (2227): the combined DPS's base is then the average damage
    // (`combined::combine`).
    // Cast on Death keeps it too (open items S3): its support's stat sets
    // it, and its stand-in time is not a trigger rate (`CalcOffence.lua`
    // 2214).
    let on_death = super::triggers::on_death(&spell.skill);
    calc.settle_state(
        ConditionKey::new(SHOW_AVERAGE),
        on_death
            || (matches!(trigger, super::triggers::Outcome::SelfCast)
                && base_mod_show_average(&spell.skill)),
    );
    let cx = calc.context(flags);
    // Spellslinger's share of the wand's base damage (open items S3).
    let mut wand_reasons = Reasons::new();
    wand_damage_to_spells(calc, spell, &mut wand_reasons)?;
    // A spell PoB's oracle has not checked: PoB computes it from its own
    // skill data too (parts, cast time overrides, skill functions, flags),
    // which the game data does not carry.
    let mut unvetted = wand_reasons;
    // The repeats, before the damage and the rate (P6-11aj).
    super::repeats::apply(calc, &spell.skill.types, &cx, &mut unvetted);
    // A trigger's rate clears the skill's own `showAverage` (open items,
    // S1); Cast on Death's keeps it.
    let read = spell.vetted
        || (spell.vetted_triggered
            && !on_death
            && matches!(trigger, super::triggers::Outcome::Rate(..)));
    if !read {
        unvetted.partial.insert(UnsupportedReason::UnmodelledSkill {
            source: spell.source(),
            what: "pob-skill-data",
        });
    }
    // PoB's totem flag is the skill's `baseFlags.totem`
    // (`CalcActiveSkill.lua` 415 to 444). A minion's spell with the
    // `SummonsTotem` type but no such flag (the Flame Golem's Immolate) is
    // cast by the minion (P6-7c3); the character's keeps the type's reason.
    let totem = if calc.actor() == crate::engine::Actor::Minion {
        super::pob_skill::skill(&spell.skill.effect().id).is_none_or(|p| p.flag("totem"))
    } else {
        spell.skill.types.contains("SummonsTotem")
            && TOTEM_DATA_SKILLS.contains(&spell.skill.effect().id.as_str())
    };
    if totem {
        unvetted.partial.insert(UnsupportedReason::UnmodelledSkill {
            source: spell.source(),
            what: TOTEM_REASON,
        });
    }

    let skill_base = skill_crit_base(calc, spell.source(), spell.level)?;
    let (crit, mut crit_reasons) = crit_chance(calc, skill_base, None, &cx)?;
    crit_reasons.extend(&unvetted);
    calc.set(Field::CritChance, phases::query(crit, crit_reasons));
    let (multiplier, mut multiplier_reasons) = crit_multiplier(calc, input.data, &cx)?;
    multiplier_reasons.extend(&unvetted);
    calc.set(
        Field::CritMultiplier,
        phases::query(multiplier, multiplier_reasons),
    );

    let mut reasons = unvetted.clone();
    let shape = HitShape {
        source: spell.source(),
        level: spell.level,
        flags,
        damage: DamageSource::Skill,
        melee_physical: None,
        crit_recently: crit.value != 0.0,
        exert_uptime: None,
    };
    let (hit, crit_hit, sources) =
        passes(calc, &shape, &enemy, input.data, multiplier, &mut reasons)?;
    let average = average_damage(calc, hit, crit_hit, crit, None)?;
    calc.set_derived(
        Field::AverageDamage,
        phases::query(average, reasons),
        &[Field::CritChance, Field::CritMultiplier],
    );

    // A trigger's rate replaces the cast rate (P6-8a; `CalcOffence.lua`
    // 2223 to 2227); without a source PoB reports the skill as self-cast.
    let (speed, mut speed_reasons) = match trigger {
        super::triggers::Outcome::SelfCast => cast_rate(calc, spell, &cx)?,
        super::triggers::Outcome::Rate(value, reasons, _) => (
            phases::constant(calc, "skill-trigger-rate", value)?,
            reasons,
        ),
        super::triggers::Outcome::Unsupported(reason) => {
            calc.set_unsupported(Field::Speed, reason);
            return Ok(());
        }
    };
    speed_reasons.extend(&unvetted);
    calc.set(Field::Speed, phases::query(speed, speed_reasons));
    hit_dps(calc, average, speed, spell.dps_multiplier)?;
    let spell_hit_chance = phases::constant(calc, "spell-hit-chance", SPELL_HIT_CHANCE)?;
    let ailment_rate = rate_with(calc, speed, spell.dps_multiplier)?;
    let ailment_hit = super::ailment::AilmentHit {
        flags,
        sources: &sources,
        crit,
        hit_chance: spell_hit_chance,
        rate: ailment_rate,
        skill: super::pob_skill::skill(&spell.skill.effect().id).map(|p| (p, spell.source())),
    };
    set_ailments(calc, &ailment_hit, &enemy, input.data, &unvetted)?;
    // Cast on Death (open items S3): PoB counts an ignite's stacks without
    // the rate (`CalcOffence.lua` 4909) and a poison or a bleed by its
    // average (`showAverage`, 4767, 6032), which the ailments do not model:
    // each damaging ailment the hit computed is not computed.
    if on_death {
        let fields = super::ailment::Ailment::ALL
            .iter()
            .map(|a| a.field())
            .chain([Field::Poe1(super::Poe1Field::BurningGroundDps)]);
        for field in fields {
            if calc.field(field).is_some() {
                calc.set_unsupported(
                    field,
                    UnsupportedReason::UnmodelledSkill {
                        source: spell.source(),
                        what: "cast-on-death-ailment",
                    },
                );
            }
        }
    }
    Ok(())
}

/// The state of PoB's `showAverage` a gem's base mods set on a self-cast
/// spell ([`spell_hit`]; read by `combined::combine`).
pub(super) const SHOW_AVERAGE: &str = "ShowAverage";

/// Whether PoB's hand-written base mods of the skill's active gem or of a
/// support set `skillData.showAverage` (an untagged `SkillData` mod; not a
/// trap's or a mine's, which `combined` reads by its stat and cooldown).
fn base_mod_show_average(skill: &MainSkill<'_>) -> bool {
    if [skill::TRAP_TYPE, skill::MINE_TYPE]
        .iter()
        .any(|t| skill.types.contains(*t))
    {
        return false;
    }
    std::iter::once(skill.effect())
        .chain(skill.supports.iter().map(|s| s.effect))
        .filter_map(|e| super::pob_skill::skill(&e.id))
        .flat_map(|p| p.base_mods.iter())
        .any(|m| {
            m.name == "SkillData"
                && m.key == Some("showAverage")
                && !m.tagged
                && m.value == Some(super::pob_skill::Value::Bool(true))
        })
}

/// The stat of Spellslinger's support part that gives a spell a share of
/// the wand's base damage (PoB's `gainPercentBaseWandDamageToSpells`).
pub(super) const WAND_DAMAGE_STAT: &str = "gain_%_of_base_wand_damage_as_added_spell_damage";

/// Spellslinger's added spell damage (open items S3; `CalcOffence.lua` 879
/// to 897): when a support of the spell has [`WAND_DAMAGE_STAT`], each
/// damage type's minimum and maximum of the wand (its weapon data, after
/// local modifiers) times the stat over 100, floored, added as `<Type>Min`
/// and `<Type>Max` BASE with `ModFlag.Spell`; with a wand in each hand, the
/// mean of the two before the floor. Without a wand it adds nothing.
fn wand_damage_to_spells(
    calc: &mut Calc<'_>,
    spell: &SpellSkill<'_>,
    reasons: &mut Reasons,
) -> Result<(), CalcError> {
    let input = *calc.input();
    let Some((gem, percent)) = spell.skill.supports.iter().find_map(|s| {
        skill::instance(s.effect, s.level, s.quality, input.data)
            .stats
            .iter()
            .find(|(id, _)| id == WAND_DAMAGE_STAT)
            .map(|(_, v)| (s.gem, *v))
    }) else {
        return Ok(());
    };
    #[expect(clippy::cast_precision_loss, reason = "a percent, exact as f64")]
    let mult = percent as f64 / 100.0;
    let hands = collect::hands(&input);
    let wand = |held: &collect::Held| matches!(held, collect::Held::Weapon(c) if c == "Wand");
    let mut wands = Vec::new();
    for (held, item, hand) in [
        (&hands.main_hand, hands.main.as_ref(), Hand::Main),
        (&hands.off_hand, hands.off.as_ref(), Hand::Off),
    ] {
        if let (true, Some((id, _))) = (wand(held), item) {
            wands.push(weapon_data(calc, *id, hand, reasons)?);
        }
    }
    if wands.is_empty() {
        return Ok(());
    }
    #[expect(clippy::cast_precision_loss, reason = "one or two wands")]
    let count = wands.len() as f64;
    let source = Source::Gem {
        group: spell.group,
        gem,
    };
    for (i, t) in TYPES.iter().enumerate() {
        for (b, bound) in Bound::ALL.into_iter().enumerate() {
            let sum: f64 = wands
                .iter()
                .filter_map(|w| w.damage.get(i).copied().flatten())
                .map(|range| range[b].value)
                .sum();
            let value = crate::math::floor(sum / count * mult);
            if value != 0.0 {
                calc.add_effect(Effect {
                    quantity: Quantity::AddedDamage(*t, bound),
                    op: Op::Base,
                    value,
                    conditions: crate::condition::Conditions::none().with_all(Flags::SPELL),
                    source: source.clone(),
                });
            }
        }
    }
    Ok(())
}

/// The main skill's ignite from one hit pass (Phase 6, [`super::ailment`]):
/// the field only when the hit ignites, with the hit's skill reasons; it
/// reads the crit chance and rate.
fn set_ailments(
    calc: &mut Calc<'_>,
    hit: &super::ailment::AilmentHit<'_>,
    enemy: &Enemy,
    data: &dyn CalcData,
    unvetted: &Reasons,
) -> Result<(), CalcError> {
    // The hit's leech (P6-10b part 9): the character's own. A totem's,
    // trap's, or mine's hit leeches only by PoB's `DamageLifeLeechToPlayer`
    // (`CalcOffence.lua` 3528 to 3530), which no mapped stat gives: no
    // leech field (open items, S1).
    let placed = hit
        .flags
        .intersects(Flags::KEYWORD_TOTEM | Flags::KEYWORD_TRAP | Flags::KEYWORD_MINE);
    if calc.actor() == crate::engine::Actor::Player && !placed {
        super::leech::rates(calc, hit, unvetted)?;
    }
    for ailment in super::ailment::Ailment::ALL {
        if let Some(super::ailment::Damaging {
            dps,
            mut reasons,
            ground,
            ..
        }) = super::ailment::damaging(calc, hit, enemy, data, ailment)?
        {
            reasons.extend(unvetted);
            // A totem's bleed stacks per active totem ([`TOTEM_REASON`]).
            if ailment == super::ailment::Ailment::Bleed
                && hit.flags.contains(Flags::KEYWORD_TOTEM)
                && let Some((_, source)) = &hit.skill
            {
                reasons.partial.insert(UnsupportedReason::UnmodelledSkill {
                    source: source.clone(),
                    what: TOTEM_REASON,
                });
            }
            // The stacks read the hit chance too (attacks; a spell's is
            // the constant 100, not a field).
            let deps: &[Field] = if hit.flags.contains(Flags::ATTACK) {
                &[Field::CritChance, Field::Speed, Field::HitChance]
            } else {
                &[Field::CritChance, Field::Speed]
            };
            // The ignite's burning ground (P6-11ga).
            if let Some(ground) = ground {
                calc.set_derived(
                    Field::Poe1(super::Poe1Field::BurningGroundDps),
                    phases::query(ground, reasons.clone()),
                    deps,
                );
            }
            calc.set_derived(ailment.field(), phases::query(dps, reasons), deps);
        }
    }
    // Impale (P6-2d): every impale's DPS, an attack's or a spell's hit.
    if let Some((dps, mut reasons)) = super::ailment::impale(calc, hit, enemy, data)? {
        reasons.extend(unvetted);
        let deps: &[Field] = if hit.flags.contains(Flags::ATTACK) {
            &[Field::CritChance, Field::Speed, Field::HitChance]
        } else {
            &[Field::CritChance, Field::Speed]
        };
        calc.set_derived(
            Field::Poe1(super::Poe1Field::ImpaleDps),
            phases::query(dps, reasons),
            deps,
        );
    }
    Ok(())
}

/// `output.AverageHit = totalHitAvg x (1 - CritChance / 100) +
/// totalCritAvg x CritChance / 100`; `output.AverageDamage = AverageHit x
/// HitChance / 100` (a spell's hit chance is the constant 100).
fn average_damage(
    calc: &mut Calc<'_>,
    hit: Valued,
    crit_hit: Valued,
    crit: Valued,
    hit_chance: Option<Valued>,
) -> Result<Valued, CalcError> {
    let crit_share = percent(calc, crit, Term::Step("crit-share"))?;
    let one = phases::constant(calc, "one", 1.0)?;
    let non_crit_share = difference(calc, one, crit_share, Term::Step("non-crit-share"))?;
    let from_hits = phases::product(calc, &[hit, non_crit_share], Term::Step("from-non-crits"))?;
    let crit_scaled = phases::product(calc, &[crit_hit, crit], Term::Step("crit-scaled"))?;
    let from_crits = percent(calc, crit_scaled, Term::Step("from-crits"))?;
    let average_hit = phases::total(calc, &[from_hits, from_crits], Term::Step("average-hit"))?;
    let hit_chance = match hit_chance {
        Some(h) => h,
        None => phases::constant(calc, "spell-hit-chance", SPELL_HIT_CHANCE)?,
    };
    let scaled = phases::product(
        calc,
        &[average_hit, hit_chance],
        Term::Step("hit-chance-scaled"),
    )?;
    percent(calc, scaled, Term::Result)
}

/// Static Strike's beams (agent list; `act_str.lua` 10663 to 10689): in
/// part 2, "Beams", PoB's `preDamageFunc` sets `hitTimeOverride =
/// repeatFrequency / (1 + StaticStrikeFrequency INC / 100)`, the period the
/// skill's [`ZAP_FREQUENCY_STAT`] over 1000 and the INC its
/// [`ZAP_SPEED_STAT`] (no other line gives it). `HitSpeed = 1 / HitTime`
/// (`CalcOffence.lua` 2403 to 2405, 2474 to 2476: no tick cap) is the rate of
/// the DPS (3712), the ailments (4325, 4607, 4914), leech (3211), and impale
/// (6045); `Speed` stays the attack rate. The part's
/// [`BEAM_DAMAGE_STAT`] is `Damage` MORE, added here. `None` outside part 2,
/// where none of the three is read; a period not above 0 is a reason.
fn static_strike_beams(
    calc: &mut Calc<'_>,
    attack: &AttackSkill<'_>,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    if attack.skill.effect().id != "StaticStrike" || attack.part_index != 2 {
        return Ok(None);
    }
    let data = calc.input().data;
    let (mut period, mut zap_speed, mut more) = (0_i64, 0_i64, 0_i64);
    for instance in skill::instances(&attack.skill, data) {
        for (stat, value) in &instance.stats {
            match stat.as_str() {
                ZAP_FREQUENCY_STAT => period += *value,
                ZAP_SPEED_STAT => zap_speed += *value,
                BEAM_DAMAGE_STAT => more += *value,
                _ => {}
            }
        }
    }
    #[expect(clippy::cast_precision_loss, reason = "gem stat values, exact as f64")]
    let (period, zap_speed, more) = (period as f64, zap_speed as f64, more as f64);
    if more != 0.0 {
        calc.add_effect(Effect {
            quantity: Quantity::Damage,
            op: Op::More,
            value: more,
            conditions: crate::condition::Conditions::none(),
            source: attack.source(),
        });
    }
    let factor = 1.0 + zap_speed / 100.0;
    if period <= 0.0 || factor <= 0.0 {
        reasons.partial.insert(UnsupportedReason::UnmodelledSkill {
            source: attack.source(),
            what: "static-strike-zap",
        });
        return Ok(None);
    }
    let period = phases::constant(calc, "static-strike-zap-period", period / 1000.0)?;
    let factor = phases::constant(calc, "static-strike-zap-speed", factor)?;
    let time = quotient(calc, period, factor, Term::Step("hit-time"))?;
    let one = phases::constant(calc, "one", 1.0)?;
    quotient(calc, one, time, Term::Result).map(Some)
}

/// The hit's rate: the beams' ([`static_strike_beams`]), set as
/// [`Field::HitSpeed`] with the skill's reasons, else the attack's `speed`
/// (PoB's `HitSpeed or Speed`).
fn hit_rate(
    calc: &mut Calc<'_>,
    beams: Option<Valued>,
    reasons: &Reasons,
    speed: Valued,
) -> Valued {
    match beams {
        Some(rate) => {
            calc.set(Field::HitSpeed, phases::query(rate, reasons.clone()));
            rate
        }
        None => speed,
    }
}

/// `output.TotalDPS = AverageDamage x (HitSpeed or Speed) x dpsMultiplier
/// x quantityMultiplier`; both multipliers are 1.
fn hit_dps(calc: &mut Calc<'_>, average: Valued, speed: Valued, dps: f64) -> Result<(), CalcError> {
    let rate = phases::total(calc, &[speed], Term::Step("rate"))?;
    let dps_multiplier = if dps == 1.0 {
        phases::product(calc, &[], Term::Step("dps-multiplier"))?
    } else {
        let part = phases::constant(calc, "part-dps-multiplier", dps)?;
        phases::product(calc, &[part], Term::Step("dps-multiplier"))?
    };
    let quantity_multiplier = phases::product(calc, &[], Term::Step("quantity-multiplier"))?;
    let dps = phases::product(
        calc,
        &[average, rate, dps_multiplier, quantity_multiplier],
        Term::Result,
    )?;
    calc.set_derived(
        Field::HitDps,
        phases::query(dps, Reasons::new()),
        &[Field::AverageDamage, Field::Speed],
    );
    Ok(())
}

/// The hit rate the ailments' stacks, the leech, and the impale read:
/// `rate x dpsMultiplier` ([`READ_PART_BASE_MODS`]).
fn rate_with(calc: &mut Calc<'_>, rate: Valued, dps: f64) -> Result<Valued, CalcError> {
    if dps == 1.0 {
        return Ok(rate);
    }
    let part = phases::constant(calc, "part-dps-multiplier", dps)?;
    phases::product(
        calc,
        &[rate, part],
        Term::Step("hit-rate-with-dps-multiplier"),
    )
}

/// One hand's attack pass (module docs; Phase 6: PoB's `passList`, one
/// pass per hand): its chance to hit, crit chance and multiplier, rate, and
/// average damage, each with its reasons.
struct HandHit {
    hit_chance: (Valued, Reasons),
    crit: (Valued, Reasons),
    multiplier: (Valued, Reasons),
    speed: (Valued, Reasons),
    average: (Valued, Reasons),
    /// The pass's bases and conversion, for its ailments.
    sources: HitSources,
}

/// One hand's weapon and rate, before its damage (P6-11c review: PoB
/// combines the hands' speeds before either hand's damage, `CalcOffence.lua`
/// 2444, which Intimidating Cry's uptime then reads).
struct HandRate {
    hand: Hand,
    flags: Flags,
    weapon: WeaponData,
    /// The pass's reasons with the weapon's.
    unvetted: Reasons,
    speed: (Valued, Reasons),
}

/// One hand's weapon values and rate (module docs, step 8) with the
/// weapon `item` in `hand`.
fn hand_rate(
    calc: &mut Calc<'_>,
    attack: &AttackSkill<'_>,
    hand: Hand,
    item: ItemId,
    flags: Flags,
    unvetted: &Reasons,
) -> Result<HandRate, CalcError> {
    let cx = calc.context(flags);
    let mut unvetted = unvetted.clone();
    let mut weapon_reasons = Reasons::new();
    let weapon = match attack.shield {
        Some(ms) => shield_weapon_data(calc, attack, ms, &mut weapon_reasons)?,
        None => weapon_data(calc, item, hand, &mut weapon_reasons)?,
    };
    unvetted.extend(&weapon_reasons);
    let (speed, mut speed_reasons) = attack_rate(calc, attack, weapon.rate, hand, &cx)?;
    speed_reasons.extend(&unvetted);
    Ok(HandRate {
        hand,
        flags,
        weapon,
        unvetted,
        speed: (speed, speed_reasons),
    })
}

/// One hand's pass (module docs, steps 4 to 9) after its rate:
/// `exert_uptime` is Intimidating Cry's uptime ([`exerted_attacks`]).
fn hand_hit(
    calc: &mut Calc<'_>,
    attack: &AttackSkill<'_>,
    rate: HandRate,
    exert_uptime: Option<Valued>,
    enemy: &Enemy,
) -> Result<HandHit, CalcError> {
    let input = *calc.input();
    let HandRate {
        hand,
        flags,
        weapon,
        unvetted,
        speed,
    } = rate;
    let cx = calc.context(flags);

    // Chance to hit (step 9).
    let mut hit_reasons = unvetted.clone();
    // `CannotBeEvaded`: `AccuracyHitChance = 100` (`CalcOffence.lua` 2162;
    // Resolute Technique, P3-8m): neither accuracy nor the enemy's evasion
    // is read.
    let cannot_be_evaded = Quantity::Poe1(Poe1Quantity::CannotBeEvaded);
    let hit_chance = match flag_on(calc, cannot_be_evaded, &cx, &mut hit_reasons)? {
        Some(flag) => by_flag(calc, flag, "hits-cannot-be-evaded", 100.0)?,
        None => {
            enemy_reasons(enemy, true, &mut hit_reasons);
            let accuracy = accuracy(calc, &cx, hand, &mut hit_reasons)?;
            accuracy_hit_chance(calc, enemy, accuracy, &cx, &mut hit_reasons)?
        }
    };

    let (crit, mut crit_reasons) = crit_chance(calc, weapon.crit, Some(hit_chance), &cx)?;
    crit_reasons.extend(&unvetted);
    let (multiplier, mut multiplier_reasons) = crit_multiplier(calc, input.data, &cx)?;
    multiplier_reasons.extend(&unvetted);

    let mut reasons = unvetted.clone();
    // `strDmgBonus = m_floor(Str / 5)` (no mapped stat changes the ratio or
    // adds dexterity and intelligence to it).
    let melee_physical = phases::bonus(
        calc,
        Field::Strength,
        0.2,
        "strength-per-melee-damage",
        "strength-bonus",
    )?;
    let shape = HitShape {
        source: attack.source(),
        level: attack.level,
        flags,
        damage: DamageSource::Weapon(&weapon.damage),
        melee_physical,
        crit_recently: crit.value != 0.0,
        exert_uptime,
    };
    let (hit, crit_hit, sources) =
        passes(calc, &shape, enemy, input.data, multiplier, &mut reasons)?;
    let average = average_damage(calc, hit, crit_hit, crit, Some(hit_chance))?;
    Ok(HandHit {
        hit_chance: (hit_chance, hit_reasons),
        crit: (crit, crit_reasons),
        multiplier: (multiplier, multiplier_reasons),
        speed,
        average: (average, reasons),
        sources,
    })
}

/// An attack's hit (module docs): with the weapon in the main hand, and
/// with the off hand's too when dual wielding (Phase 6).
pub(super) fn attack_hit(calc: &mut Calc<'_>, attack: &AttackSkill<'_>) -> Result<(), CalcError> {
    let input = *calc.input();
    let enemy = enemy::enemy(input.build, input.data);
    let mut flags =
        Flags::ATTACK | Flags::KEYWORD_ATTACK | Flags::HIT | attack.skill.keyword_flags();
    // Melee, else projectile (`CalcActiveSkill.lua` 361 to 366; the
    // weapon already cleared the other, [`attack_skill`]).
    if attack.part.melee {
        flags = flags | Flags::MELEE;
    } else if attack.part.projectile {
        flags = flags | Flags::PROJECTILE;
    }
    if attack.part.area {
        flags = flags | Flags::AREA;
    }
    // `KeywordFlag.Bow` for an attack whose main hand holds a bow
    // (`CalcActiveSkill.lua` 406: both hands' configs).
    if attack.weapon_flags.contains(Flags::BOW) {
        flags = flags | Flags::KEYWORD_BOW;
    }
    if attack.arrow {
        flags = flags | Flags::KEYWORD_ARROW;
    }
    // Each hand's config adds its weapon's flags (`weapon1Cfg`,
    // `weapon2Cfg`, `CalcActiveSkill.lua` 470 to 479; P6-11b).
    // PoB's `OffHandAttack` condition (P6-11hg): the off hand's context,
    // and a shield attack's only one.
    let off_flags = flags | attack.off_weapon_flags | Flags::OFF_HAND_ATTACK;
    let flags = if attack.shield.is_some() {
        flags | attack.weapon_flags | Flags::OFF_HAND_ATTACK
    } else {
        flags | attack.weapon_flags
    };
    let mut unvetted = Reasons::new();
    // The repeats in the skill's config, the main hand's flags (P6-11aj).
    let skill_cx = calc.context(flags);
    super::repeats::apply(calc, &attack.skill.types, &skill_cx, &mut unvetted);
    let id = attack.skill.effect().id.as_str();
    // A totem's attack ([`TOTEM_REASON`]): the character's keeps the reason
    // only for [`TOTEM_DATA_SKILLS`]; a minion's as before.
    if attack.skill.types.contains("SummonsTotem")
        && (calc.actor() == crate::engine::Actor::Minion || TOTEM_DATA_SKILLS.contains(&id))
    {
        unvetted.partial.insert(UnsupportedReason::UnmodelledSkill {
            source: attack.source(),
            what: TOTEM_REASON,
        });
    }
    // PoB takes a main-hand attack's base crit from the weapon alone
    // (`source.CritChance`); a level's own attack crit chance replaces the
    // off hand's weapon crit (`skillData.CritChance`), which is not
    // modelled: a reason for either hand.
    // A shield attack's crit is the level's (P6-11he, [`shield_weapon_data`]).
    if attack.level.attack_crit_chance != 0 && attack.shield.is_none() {
        unvetted.partial.insert(UnsupportedReason::UnmodelledSkill {
            source: attack.source(),
            what: "skill-crit-chance",
        });
    }
    // Manaforged Arrows' more damage per mana spent (open items S3; PoB's
    // `Damage` MORE with `PerStat` `ManaCost`, its value times `floor(cost
    // + 0.0001)`): a `Damage` MORE effect per term, from the cost field,
    // which the cost phase sets before the hit.
    let per_cost = Quantity::Poe1(Poe1Quantity::DamageMorePerManaCost);
    if phases::has(calc, per_cost) {
        let terms = super::repeats::applying(calc, per_cost, Op::Base, &skill_cx, &mut unvetted);
        let cost = calc
            .field(Field::Poe1(super::Poe1Field::ManaCost))
            .and_then(|f| f.value.map(|v| (v, f.state.clone())));
        match cost {
            Some((cost, state)) if super::triggers::carry(&state, &mut unvetted) => {
                let steps = crate::math::floor(cost + 0.0001);
                for e in terms {
                    calc.add_effect(Effect {
                        quantity: Quantity::Damage,
                        op: Op::More,
                        value: e.value * steps,
                        conditions: e.conditions.clone(),
                        source: e.source.clone(),
                    });
                }
            }
            _ => {
                unvetted.partial.insert(UnsupportedReason::UnmodelledSkill {
                    source: attack.source(),
                    what: "damage-per-mana-cost",
                });
            }
        }
    }
    // Static Strike's beams ([`static_strike_beams`]): their less damage
    // before either hand's damage; their hit rate is the DPS's and the
    // ailments' rate, the attack rate staying `Speed`.
    let beam_rate = static_strike_beams(calc, attack, &mut unvetted)?;
    // A trigger's rate is every pass's speed (open items S3;
    // `CalcOffence.lua` 2223 to 2227, `output.Speed =
    // skillData.triggerRate`); without a source PoB reports the skill as
    // used by hand. A rate that cannot be computed leaves the attack's
    // fields not computed.
    let trigger_speed = match super::triggers::rate(calc, &attack.skill, &attack.source(), flags) {
        super::triggers::Outcome::SelfCast => None,
        super::triggers::Outcome::Rate(value, reasons, triggered_damage) => {
            calc.settle_state(
                ConditionKey::new(super::triggers::TRIGGERED_DAMAGE),
                triggered_damage,
            );
            Some((value, reasons))
        }
        super::triggers::Outcome::Unsupported(reason) => {
            calc.set_unsupported(Field::Speed, reason);
            return Ok(());
        }
    };
    // PoB's data of the skill, read in full ([`pob_vetted`]); a trigger's
    // rate clears its `showAverage` ([`pob_vetted_triggered`]).
    let read = (attack.known_parts
        || (trigger_speed.is_some() && pob_vetted_triggered(id, attack.part_index, false)))
        && !config_noop_set(calc.input(), id);
    if !read {
        unvetted.partial.insert(UnsupportedReason::UnmodelledSkill {
            source: attack.source(),
            what: "pob-skill-data",
        });
    }
    let triggered = |calc: &mut Calc<'_>, rate: &mut HandRate| -> Result<(), CalcError> {
        if let Some((value, reasons)) = &trigger_speed {
            let speed = phases::constant(calc, "skill-trigger-rate", *value)?;
            let mut reasons = reasons.clone();
            reasons.extend(&rate.unvetted);
            rate.speed = (speed, reasons);
        }
        Ok(())
    };
    // A shield attack hits with the off hand only (P6-11he).
    let hand = if attack.shield.is_some() {
        Hand::Off
    } else {
        Hand::Main
    };
    let mut main_rate = hand_rate(calc, attack, hand, attack.weapon, flags, &unvetted)?;
    triggered(calc, &mut main_rate)?;
    let Some(off_item) = attack.off_weapon else {
        // The rate came first, as PoB (`CalcOffence.lua`: speed before
        // damage): the exerted attacks read the global speed.
        let exert = exerted_attacks(calc, attack, flags, main_rate.speed.0, &mut unvetted)?;
        main_rate.unvetted.extend(&unvetted);
        let main = hand_hit(calc, attack, main_rate, exert, &enemy)?;
        let HandHit {
            hit_chance,
            crit,
            multiplier,
            speed,
            average,
            sources,
        } = main;
        calc.set_derived(
            Field::HitChance,
            phases::query(hit_chance.0, hit_chance.1),
            &[Field::Dexterity],
        );
        calc.set_derived(
            Field::CritChance,
            phases::query(crit.0, crit.1),
            &[Field::HitChance],
        );
        calc.set(
            Field::CritMultiplier,
            phases::query(multiplier.0, multiplier.1),
        );
        calc.set_derived(
            Field::AverageDamage,
            phases::query(average.0, average.1),
            &[
                Field::CritChance,
                Field::CritMultiplier,
                Field::HitChance,
                Field::Strength,
            ],
        );
        calc.set(Field::Speed, phases::query(speed.0, speed.1));
        let rate = hit_rate(calc, beam_rate, &unvetted, speed.0);
        let dps = part_dps_multiplier(id, attack.part_index)
            * perforate_dps_multiplier(&input, &attack.skill, &mut unvetted);
        hit_dps(calc, average.0, rate, dps)?;
        let ailment_rate = rate_with(calc, rate, dps)?;
        let ailment_hit = super::ailment::AilmentHit {
            flags,
            sources: &sources,
            crit: crit.0,
            hit_chance: hit_chance.0,
            rate: ailment_rate,
            skill: super::pob_skill::skill(id).map(|p| (p, attack.source())),
        };
        return set_ailments(calc, &ailment_hit, &enemy, input.data, &unvetted);
    };
    // A part's `dpsMultiplier` ([`READ_PART_BASE_MODS`]) is read only by the
    // single-hand path: a dual wielder's is a reason on every field of the
    // pass (the agent list, 2026-11-27).
    if part_dps_multiplier(id, attack.part_index) != 1.0 || PERFORATE.iter().any(|(s, _)| *s == id)
    {
        unvetted.partial.insert(UnsupportedReason::UnmodelledSkill {
            source: attack.source(),
            what: "dual-wield-dps-multiplier",
        });
    }
    let mut off_rate = hand_rate(calc, attack, Hand::Off, off_item, off_flags, &unvetted)?;
    triggered(calc, &mut off_rate)?;
    let speed = combined_speed(calc, main_rate.speed.0, off_rate.speed.0)?;
    let exert = exerted_attacks(calc, attack, flags, speed, &mut unvetted)?;
    main_rate.unvetted.extend(&unvetted);
    off_rate.unvetted.extend(&unvetted);
    let main = hand_hit(calc, attack, main_rate, exert, &enemy)?;
    let off = hand_hit(calc, attack, off_rate, exert, &enemy)?;
    // The hit rate after the hands' speeds are combined (`CalcOffence.lua`
    // 2474 to 2476): the beams' when set, else the combined speed.
    let rate = hit_rate(calc, beam_rate, &unvetted, speed);
    dual_wield(calc, attack, &main, &off, speed, rate)?;
    let speed = rate;
    // A dual wielder's damaging ailments (open items S7): each hand's pass
    // with its own flags (`band(cfg.flags, ModFlag.WeaponMask)`, the hand's
    // config; P6-11b review), its stacks at the hands' combined speed (PoB's
    // `globalOutput.Speed`, 4323, 4607), then PoB's `combineStat`
    // (5528 to 5545). Burning ground stays not computed: PoB does not
    // combine it.
    let mut hand_reasons = unvetted.clone();
    hand_reasons.extend(&main.speed.1);
    hand_reasons.extend(&off.speed.1);
    for ailment in super::ailment::Ailment::ALL {
        let mut each = Vec::with_capacity(2);
        for (hand, flags) in [(&main, flags), (&off, off_flags)] {
            let ailment_hit = super::ailment::AilmentHit {
                flags,
                sources: &hand.sources,
                crit: hand.crit.0,
                hit_chance: hand.hit_chance.0,
                rate: speed,
                skill: super::pob_skill::skill(id).map(|p| (p, attack.source())),
            };
            each.push(super::ailment::damaging(
                calc,
                &ailment_hit,
                &enemy,
                input.data,
                ailment,
            )?);
        }
        let mut each = each.into_iter();
        let main_ailment = each.next().flatten();
        let off_ailment = each.next().flatten();
        if main_ailment.is_none() && off_ailment.is_none() {
            continue;
        }
        if ailment == super::ailment::Ailment::Ignite
            && (main_ailment.as_ref().is_some_and(|a| a.ground.is_some())
                || off_ailment.as_ref().is_some_and(|a| a.ground.is_some()))
        {
            calc.set_unsupported(
                Field::Poe1(super::Poe1Field::BurningGroundDps),
                UnsupportedReason::NotComputed,
            );
        }
        let mut reasons = hand_reasons.clone();
        for a in [&main_ailment, &off_ailment].into_iter().flatten() {
            reasons.extend(&a.reasons);
        }
        let value = combine_ailment(
            calc,
            ailment,
            main_ailment.as_ref(),
            off_ailment.as_ref(),
            attack.double_hits,
        )?;
        calc.set_derived(
            ailment.field(),
            phases::query(value, reasons),
            &[Field::CritChance, Field::Speed, Field::HitChance],
        );
    }
    // A dual wielder's leech, each hand's combined (`combineStat` `DPS`,
    // 3877 to 3891; open items S7).
    if calc.actor() == crate::engine::Actor::Player {
        let [main_hit, off_hit] =
            [(&main, flags), (&off, off_flags)].map(|(hand, flags)| super::ailment::AilmentHit {
                flags,
                sources: &hand.sources,
                crit: hand.crit.0,
                hit_chance: hand.hit_chance.0,
                rate: speed,
                skill: None,
            });
        super::leech::rates_dual(
            calc,
            [&main_hit, &off_hit],
            speed,
            attack.double_hits,
            &unvetted,
        )?;
    }
    // A dual wielder's impale (open items S7, parts 4 and 9;
    // `CalcOffence.lua` 5569 to 5571, 6027 to 6047): each hand's parts,
    // then [`combine_impale`]. When only one hand impales, the other hand's
    // stored hit is still averaged in (`impaleStoredHitAvg` is every pass's).
    let mut parts = Vec::with_capacity(2);
    let mut hits = Vec::with_capacity(2);
    for (hand, flags) in [(&main, flags), (&off, off_flags)] {
        let ailment_hit = super::ailment::AilmentHit {
            flags,
            sources: &hand.sources,
            crit: hand.crit.0,
            hit_chance: hand.hit_chance.0,
            rate: speed,
            skill: None,
        };
        parts.push(super::ailment::impale_parts(
            calc,
            &ailment_hit,
            &enemy,
            input.data,
        )?);
        hits.push(ailment_hit);
    }
    if parts.iter().all(Option::is_none) {
        return Ok(());
    }
    let mut hands = Vec::with_capacity(2);
    let mut reasons = hand_reasons.clone();
    for ((part, hit), hand) in parts.iter().zip(&hits).zip([&main, &off]) {
        let stored = match part {
            Some(p) => {
                reasons.extend(&p.reasons);
                p.stored_hit
            }
            None => match super::ailment::impale_stored_hit(calc, hit)? {
                Some(s) => s,
                None => phases::constant(calc, "impale-stored-hit-none", 0.0)?,
            },
        };
        hands.push((part.as_ref(), stored, hand.hit_chance.0));
    }
    let [m, o] = [hands[0], hands[1]];
    let value = combine_impale(calc, [m, o], speed, attack.double_hits)?;
    if value.value > 0.0 {
        calc.set_derived(
            Field::Poe1(super::Poe1Field::ImpaleDps),
            phases::query(value, reasons),
            &[Field::CritChance, Field::Speed, Field::HitChance],
        );
    }
    Ok(())
}

/// One hand for [`combine_impale`]: its parts when it impales, its stored
/// hit (`impaleStoredHitAvg`, every pass's), and its chance to hit.
type ImpaleHand<'a> = (Option<&'a super::ailment::ImpaleParts>, Valued, Valued);

/// A dual wielder's impale DPS from each hand's parts (`CalcOffence.lua`
/// 6027 to 6047; open items S7), times the combined `speed`:
/// - a skill that hits with both weapons: `sum over hands of stored hit x
///   (modifier - 1) x hit chance / 100`;
/// - otherwise PoB's combined values: the stored hit and the hit chance
///   averaged (`DPS` halved, `AVERAGE`), and `ImpaleModifier` by `CHANCE`:
///   each hand's weighted by `ImpaleChance x HitChance` over both; then
///   `stored x (modifier - 1) x hit chance / 100`.
///
/// When only one hand impales (part 9), the other's `ImpaleModifier` is
/// nil: the summed branch's term for it is 0 (`(nil or 1) - 1`), and
/// `CHANCE` takes the impaling hand's modifier alone (2061); the stored
/// hit and the hit chance are still both hands' averages.
fn combine_impale(
    calc: &mut Calc<'_>,
    hands: [ImpaleHand<'_>; 2],
    speed: Valued,
    double: bool,
) -> Result<Valued, CalcError> {
    let one = phases::constant(calc, "one", 1.0)?;
    let two = phases::constant(calc, "two", 2.0)?;
    let per_hit = if double {
        let mut terms = Vec::with_capacity(2);
        for (parts, stored, hit_chance) in hands {
            let Some(parts) = parts else { continue };
            let share = percent_of(calc, hit_chance, Term::Step("hit-share"))?;
            terms.push(phases::product(
                calc,
                &[stored, parts.gain, share],
                Term::Step("impale-per-hit"),
            )?);
        }
        phases::total(calc, &terms, Term::Step("both-hands"))?
    } else {
        let [(m, ms, mh), (o, os, oh)] = hands;
        let stored = phases::total(calc, &[ms, os], Term::Step("both-hands"))?;
        let stored = quotient(calc, stored, two, Term::Step("impale-stored-hit-combined"))?;
        let hit_sum = phases::total(calc, &[mh, oh], Term::Step("both-hands"))?;
        let hit_chance = quotient(calc, hit_sum, two, Term::Step("hit-chance-combined"))?;
        let (m, o) = match (m, o) {
            (Some(m), Some(o)) => (m, o),
            (Some(one_hand), None) | (None, Some(one_hand)) => {
                let share = percent_of(calc, hit_chance, Term::Step("hit-share"))?;
                let per_hit = phases::product(
                    calc,
                    &[stored, one_hand.gain, share],
                    Term::Step("impale-per-hit"),
                )?;
                return phases::product(calc, &[per_hit, speed], Term::Result);
            }
            (None, None) => return phases::constant(calc, "impale-none", 0.0),
        };
        let main_weight = phases::product(calc, &[m.chance, mh], Term::Step("impale-main-chance"))?;
        let off_weight = phases::product(calc, &[o.chance, oh], Term::Step("impale-off-chance"))?;
        let weights = phases::total(
            calc,
            &[main_weight, off_weight],
            Term::Step("impale-chances"),
        )?;
        let main_portion = quotient(
            calc,
            main_weight,
            weights,
            Term::Step("impale-main-portion"),
        )?;
        let off_portion = quotient(calc, off_weight, weights, Term::Step("impale-off-portion"))?;
        let from_main = phases::product(
            calc,
            &[m.modifier, main_portion],
            Term::Step("impale-main-part"),
        )?;
        let from_off = phases::product(
            calc,
            &[o.modifier, off_portion],
            Term::Step("impale-off-part"),
        )?;
        let modifier = phases::total(calc, &[from_main, from_off], Term::Step("impale-modifier"))?;
        let gain = minus(calc, modifier, one, Term::Step("impale-gain"))?;
        let share = percent_of(calc, hit_chance, Term::Step("hit-share"))?;
        phases::product(calc, &[stored, gain, share], Term::Step("impale-per-hit"))?
    };
    phases::product(calc, &[per_hit, speed], Term::Result)
}

/// PoB's `combineStat` of a dual wielder's ailment (`CalcOffence.lua` 2002
/// to 2104, 5528 to 5545; open items S7):
/// - bleeding and ignite (`CHANCE_AILMENT`): one hand's alone when only it
///   inflicts it; otherwise `max x r + min x (1 - r)`, `r = min(1,
///   <Ailment>Stacks / <Ailment>StacksMax)` of the shared output. For
///   bleeding that is the stacks of the last pass that bled (the off
///   hand's, 4476) over the maximum of 1 the engine models; ignite's stacks
///   are never written (1), so `r` is 1 and the larger hand's ignite
///   counts.
/// - poison (`TotalPoisonDPS`, `DPS`): `(main + off) / 2`, a hand without it
///   0, the sum for a skill that hits with both weapons.
fn combine_ailment(
    calc: &mut Calc<'_>,
    ailment: super::ailment::Ailment,
    main: Option<&super::ailment::Damaging>,
    off: Option<&super::ailment::Damaging>,
    double: bool,
) -> Result<Valued, CalcError> {
    use super::ailment::Ailment;
    if ailment == Ailment::Poison {
        let zero = phases::constant(calc, "zero", 0.0)?;
        let terms = [main, off].map(|a| a.map_or(zero, |a| a.dps));
        let sum = phases::total(calc, &terms, Term::Step("both-hands"))?;
        if double {
            return Ok(sum);
        }
        let two = phases::constant(calc, "two", 2.0)?;
        return quotient(calc, sum, two, Term::Result);
    }
    let (m, o) = match (main, off) {
        (Some(m), Some(o)) => (m, o),
        (Some(a), None) | (None, Some(a)) => return Ok(a.dps),
        (None, None) => return phases::constant(calc, "zero", 0.0),
    };
    let larger = phases::pair(calc, m.dps, o.dps, Term::Step("larger-hand"), false)?;
    if ailment == Ailment::Ignite {
        return Ok(larger);
    }
    let smaller = phases::pair(calc, m.dps, o.dps, Term::Step("smaller-hand"), true)?;
    let one = phases::constant(calc, "one", 1.0)?;
    let share = phases::pair(calc, o.stacks, one, Term::Step("larger-hand-share"), true)?;
    let rest = minus(calc, one, share, Term::Step("smaller-hand-share"))?;
    let from_larger = phases::product(calc, &[larger, share], Term::Step("larger-hand-part"))?;
    let from_smaller = phases::product(calc, &[smaller, rest], Term::Step("smaller-hand-part"))?;
    phases::total(calc, &[from_larger, from_smaller], Term::Result)
}

/// The hands' combined speed (PoB's `combineStat("Speed",
/// "HARMONICMEAN")`, `CalcOffence.lua` 2444): `2 / (1 / main + 1 / off)`, 0
/// when either is 0.
fn combined_speed(calc: &mut Calc<'_>, main: Valued, off: Valued) -> Result<Valued, CalcError> {
    if main.value == 0.0 || off.value == 0.0 {
        return phases::constant(calc, "no-speed", 0.0);
    }
    let two = phases::constant(calc, "two", 2.0)?;
    let one = phases::constant(calc, "one", 1.0)?;
    let a = quotient(calc, one, main, Term::Step("main-hand-time"))?;
    let b = quotient(calc, one, off, Term::Step("off-hand-time"))?;
    let sum = phases::total(calc, &[a, b], Term::Step("both-hands-time"))?;
    quotient(calc, two, sum, Term::Result)
}

/// Both hands' passes combined (Phase 6; PoB's `combineStat`,
/// `CalcOffence.lua` 2003 to 2108, 2441 to 2445, 3869 to 3875): chance to
/// hit, crit chance, and crit multiplier `AVERAGE`; speed the harmonic mean
/// (0 when a hand's is 0); average damage `DPS` (the hands' sum, halved
/// unless the skill hits with both at once); hit DPS the combined average
/// times the combined speed (each hand's `TotalDPS` reads the combined
/// speed, so `DPS` of them is that product).
fn dual_wield(
    calc: &mut Calc<'_>,
    attack: &AttackSkill<'_>,
    main: &HandHit,
    off: &HandHit,
    speed: Valued,
    rate: Valued,
) -> Result<(), CalcError> {
    let two = phases::constant(calc, "two", 2.0)?;
    let average_of = |calc: &mut Calc<'_>, a: Valued, b: Valued| -> Result<Valued, CalcError> {
        let sum = phases::total(calc, &[a, b], Term::Step("both-hands"))?;
        quotient(calc, sum, two, Term::Result)
    };
    let joined = |a: &Reasons, b: &Reasons| {
        let mut r = a.clone();
        r.extend(b);
        r
    };
    let hit_chance = average_of(calc, main.hit_chance.0, off.hit_chance.0)?;
    calc.set_derived(
        Field::HitChance,
        phases::query(hit_chance, joined(&main.hit_chance.1, &off.hit_chance.1)),
        &[Field::Dexterity],
    );
    let crit = average_of(calc, main.crit.0, off.crit.0)?;
    calc.set_derived(
        Field::CritChance,
        phases::query(crit, joined(&main.crit.1, &off.crit.1)),
        &[Field::HitChance],
    );
    let multiplier = average_of(calc, main.multiplier.0, off.multiplier.0)?;
    calc.set(
        Field::CritMultiplier,
        phases::query(multiplier, joined(&main.multiplier.1, &off.multiplier.1)),
    );
    calc.set(
        Field::Speed,
        phases::query(speed, joined(&main.speed.1, &off.speed.1)),
    );
    // `DPS`: `(main + off)`, halved unless the skill hits with both.
    let dps_mode = |calc: &mut Calc<'_>, a: Valued, b: Valued| -> Result<Valued, CalcError> {
        let sum = phases::total(calc, &[a, b], Term::Step("both-hands"))?;
        if attack.double_hits {
            Ok(sum)
        } else {
            quotient(calc, sum, two, Term::Result)
        }
    };
    let average = dps_mode(calc, main.average.0, off.average.0)?;
    calc.set_derived(
        Field::AverageDamage,
        phases::query(average, joined(&main.average.1, &off.average.1)),
        &[
            Field::CritChance,
            Field::CritMultiplier,
            Field::HitChance,
            Field::Strength,
        ],
    );
    // Each hand's `TotalDPS = AverageDamage x globalOutput.Speed x 1 x 1`
    // (`CalcOffence.lua` 3712: the combined speed, set before the passes'
    // damage), then `DPS`: the combined average times the combined speed
    // (P6-11b; P6-2a used each hand's own speed, which agreed only when
    // the hands' speeds did). With a hit rate (`rate`, PoB's `HitSpeed`),
    // that rate (agent list: Static Strike's beams).
    hit_dps(calc, average, rate, 1.0)
}

/// The main skill's exerted attacks (`CalcOffence.lua` 2539 to 2802;
/// close-out for Intimidating Cry, the agent list for the others). For a
/// skill PoB lets warcries exert (not `NeverExertable`, triggered,
/// channelled, used by another thing, a retaliation, or a totem's), each
/// modelled cry ([`super::gems::ExertCry`]) whose type the skill has gives
/// its uptime, percent: `min(100, min((exerts / speed) / (cooldown + cast
/// time), 1) x 100 x stored uses)`, `exerts` PoB's `Num<Cry>Exerts`,
/// `floor((base + extra) x more)` with [`Poe1Quantity::ExtraExertedAttacks`]
/// (`CalcPerform.lua` 2316 to 2320), the cast time `1 / min(cast rate x
/// action speed, tick rate)` (`calcWarcryCastTime`). The exerted attacks'
/// uptime is the largest of them, the warcries' the sum
/// (`GlobalWarcryUptimeRatio`); with the first above 0 the skill gains, as
/// PoB's `NewMod` (2783 to 2787), `Damage` INC and MORE of the summed
/// [`Poe1Quantity::ExertDamage`] times that uptime over 100, and `Damage`
/// MORE of the summed [`Poe1Quantity::ExertAverageDamage`] times the sum
/// over 100. Returns Intimidating Cry's uptime, which the double damage
/// reads (3200).
fn exerted_attacks(
    calc: &mut Calc<'_>,
    attack: &AttackSkill<'_>,
    flags: Flags,
    speed: Valued,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    use super::gems::{ExertCry, WarcryValue};
    let types = &attack.skill.types;
    let excluded = [
        "NeverExertable",
        "Triggered",
        "Channel",
        "OtherThingUsesSkill",
        "Retaliation",
        "SummonsTotem",
    ];
    if excluded.iter().any(|t| types.contains(*t)) {
        return Ok(None);
    }
    let cries: Vec<ExertCry> = ExertCry::ALL
        .into_iter()
        .filter(|c| {
            types.contains(c.skill_type())
                && phases::has(
                    calc,
                    Quantity::Poe1(Poe1Quantity::ExertingCry(*c, WarcryValue::Exerts)),
                )
        })
        .collect();
    if cries.is_empty() {
        return Ok(None);
    }
    let cx = calc.context(Flags::NONE);
    let extra = Quantity::Poe1(Poe1Quantity::ExtraExertedAttacks);
    let extra_base = sum_in(
        calc,
        extra,
        Op::Base,
        &cx,
        Term::Step("extra-exerts"),
        reasons,
    )?;
    let extra_more = more(calc, extra, &cx, reasons)?;
    let one = phases::constant(calc, "one", 1.0)?;
    let hundred = phases::constant(calc, "hundred", 100.0)?;
    let cap = phases::constant(calc, "server-tick-rate", 1.0 / SERVER_TICK_TIME)?;
    let mut intimidating = None;
    let mut uptimes = Vec::new();
    for cry in cries {
        let q = |v| Quantity::Poe1(Poe1Quantity::ExertingCry(cry, v));
        let term = |calc: &mut Calc<'_>, v: WarcryValue, reasons: &mut Reasons| {
            sum_in(calc, q(v), Op::Base, &cx, Term::Step(v.name()), reasons)
        };
        let base = term(calc, WarcryValue::Exerts, reasons)?;
        let cooldown = term(calc, WarcryValue::Cooldown, reasons)?;
        let cast_rate = term(calc, WarcryValue::CastRate, reasons)?;
        let stored = term(calc, WarcryValue::StoredUses, reasons)?;
        let exerts = phases::total(calc, &[base, extra_base], Term::Step("exerts-with-extra"))?;
        let exerts = phases::product(calc, &[exerts, extra_more], Term::Step("exerts-more"))?;
        let exerts = floor(calc, exerts, Term::Step("exerts"))?;
        let cast_rate = action_speed(calc, cast_rate, reasons)?;
        let cast_rate = phases::pair(calc, cast_rate, cap, Term::Step("warcry-cast-rate"), true)?;
        let cast_time = quotient(calc, one, cast_rate, Term::Step("warcry-cast-time"))?;
        let per_attack = quotient(calc, exerts, speed, Term::Step("exerts-per-attack-rate"))?;
        let window = phases::total(calc, &[cooldown, cast_time], Term::Step("warcry-window"))?;
        let ratio = quotient(calc, per_attack, window, Term::Step("exert-ratio"))?;
        let ratio = phases::pair(calc, ratio, one, Term::Step("exert-ratio-capped"), true)?;
        let base = phases::product(calc, &[ratio, hundred], Term::Step("exert-uptime-base"))?;
        let uptime = phases::product(calc, &[base, stored], Term::Step("exert-uptime-stored"))?;
        let uptime = phases::pair(calc, uptime, hundred, Term::Step("exert-uptime"), true)?;
        if cry == ExertCry::Intimidating {
            intimidating = Some(uptime);
        }
        uptimes.push(uptime);
    }
    let zero = phases::constant(calc, "zero", 0.0)?;
    let mut exerted = zero;
    for u in &uptimes {
        exerted = phases::pair(calc, exerted, *u, Term::Step("exerted-uptime"), false)?;
    }
    if exerted.value > 0.0 {
        let global = phases::total(calc, &uptimes, Term::Step("warcry-uptime"))?;
        let hand = calc.context(flags);
        let damage = Quantity::Poe1(Poe1Quantity::ExertDamage);
        let average = Quantity::Poe1(Poe1Quantity::ExertAverageDamage);
        let inc = sum_in(
            calc,
            damage,
            Op::Increased,
            &hand,
            Term::Step("exert-inc"),
            reasons,
        )?;
        let more = sum_in(
            calc,
            damage,
            Op::More,
            &hand,
            Term::Step("exert-more"),
            reasons,
        )?;
        let avg = sum_in(
            calc,
            average,
            Op::More,
            &hand,
            Term::Step("exert-average"),
            reasons,
        )?;
        for (op, v, u) in [
            (Op::Increased, inc, exerted),
            (Op::More, more, exerted),
            (Op::More, avg, global),
        ] {
            let value = v.value * u.value / 100.0;
            if value != 0.0 {
                calc.add_effect(Effect {
                    quantity: Quantity::Damage,
                    op,
                    value,
                    conditions: crate::condition::Conditions::none(),
                    source: attack.source(),
                });
            }
        }
    }
    Ok(intimidating)
}

/// The weapon's values (module docs, step 8).
pub(super) struct WeaponData {
    /// Damage per type in [`TYPES`] order: `[min, max]` when both are
    /// above 0.
    pub(super) damage: [Option<[Valued; 2]>; 5],
    /// Attacks per second.
    pub(super) rate: Valued,
    /// Critical strike chance, percent.
    pub(super) crit: Valued,
}

/// A shield attack's weapon (P6-11he; `CalcOffence.lua` 1957 to 1983): a
/// shield has no weapon data (`Item.lua` 2488), so the off hand's source is
/// the skill's own damage ([`Poe1Quantity::ShieldAttackDamage`], PoB's
/// `setOffHand<Type>Min` and `Max`), the level's crit chance
/// (`skillData.CritChance`), and `1000 / attackTime`.
fn shield_weapon_data(
    calc: &mut Calc<'_>,
    attack: &AttackSkill<'_>,
    attack_time: i64,
    reasons: &mut Reasons,
) -> Result<WeaponData, CalcError> {
    let cx = calc.context(Flags::NONE);
    let mut damage = [None; 5];
    for (slot, t) in damage.iter_mut().zip(TYPES) {
        let mut range = Vec::new();
        for b in Bound::ALL {
            let q = Quantity::Poe1(Poe1Quantity::ShieldAttackDamage(t, b));
            range.push(sum_in(calc, q, Op::Base, &cx, Term::Base, reasons)?);
        }
        if range[0].value > 0.0 && range[1].value > 0.0 {
            *slot = Some([range[0], range[1]]);
        }
    }
    if attack_time <= 0 {
        reasons
            .indeterminate
            .insert(IndeterminateReason::MissingData {
                table: "granted_effects_per_level",
                key: "attack_time".to_owned(),
            });
    }
    #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
    let ms = attack_time as f64;
    let ms = leaf(
        calc,
        attack.source(),
        Quantity::Poe1(Poe1Quantity::Weapon(WeaponValue::AttackTime)),
        ms,
    )?;
    let thousand = phases::constant(calc, "milliseconds", 1000.0)?;
    let rate = quotient(calc, thousand, ms, Term::Step("weapon-rate"))?;
    #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
    let crit = attack.level.attack_crit_chance as f64 / 100.0;
    let crit = leaf(
        calc,
        attack.source(),
        Quantity::Poe1(Poe1Quantity::SkillCritChance),
        crit,
    )?;
    Ok(WeaponData { damage, rate, crit })
}

/// Added damage of type `ty` equal to a percent of maximum mana (the agent
/// list, 2026-11-27; PoB's `PercentStat` `Mana`, `ModStore.lua` 487 to
/// 530: each mod `ceil(value x mana x percent / 100)`), both ends; `None`
/// when no effect gives any.
fn mana_added(
    calc: &mut Calc<'_>,
    ty: DamageType,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    let q = Quantity::Poe1(Poe1Quantity::AddedDamagePercentOfMana(ty));
    if !phases::has(calc, q) {
        return Ok(None);
    }
    let effects = super::repeats::applying(calc, q, Op::Base, cx, reasons);
    if effects.is_empty() {
        return Ok(None);
    }
    let Some(mana) = calc.field(Field::Mana).and_then(|r| r.value) else {
        reasons.partial.insert(UnsupportedReason::NotComputed);
        return Ok(None);
    };
    let value: f64 = effects
        .iter()
        .map(|e| crate::math::ceil(e.value * mana / 100.0))
        .sum();
    Ok(Some(phases::constant(calc, "added-from-mana", value)?))
}

/// A shield attack's added damage of type `ty` per 15 of the shield's own
/// armour, evasion rating, or both (P6-11he; PoB's `PerStat` with
/// `ArmourOnWeapon 2` and `EvasionOnWeapon 2`, the shield's armour and
/// evasion before global modifiers, `CalcDefence.lua` 496, 500); `None`
/// when no effect gives any, so other hits keep their graph.
fn shield_added(
    calc: &mut Calc<'_>,
    ty: DamageType,
    b: Bound,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    use super::{ArmourSlot, Defence};
    let mut terms = Vec::new();
    for mask in [1_u8, 2, 3] {
        let q = Quantity::Poe1(Poe1Quantity::ShieldAddedDamage(ty, b, mask));
        if !phases::has(calc, q) {
            continue;
        }
        let mut parts = Vec::new();
        if mask & 1 != 0 {
            parts.push(phases::piece(
                calc,
                ArmourSlot::Weapon2,
                Defence::Armour,
                reasons,
            )?);
        }
        if mask & 2 != 0 {
            parts.push(phases::piece(
                calc,
                ArmourSlot::Weapon2,
                Defence::Evasion,
                reasons,
            )?);
        }
        let stat = phases::total(calc, &parts, Term::Step("shield-defences"))?;
        terms.push(per_value(
            calc,
            q,
            stat,
            SHIELD_DEFENCE_STEP,
            None,
            cx,
            reasons,
        )?);
    }
    if terms.is_empty() {
        return Ok(None);
    }
    Ok(Some(phases::total(
        calc,
        &terms,
        Term::Step("shield-added-damage"),
    )?))
}

/// Critical strike chance (`multiplier` false) or multiplier per 10 of the
/// shield's maximum energy shield (P6-11hg; PoB's `PerStat`
/// `EnergyShieldOnWeapon 2`, the shield's own energy shield before global
/// modifiers, `CalcDefence.lua` 492); `None` without such an effect, so
/// other hits keep their graph.
fn per_shield_energy_shield(
    calc: &mut Calc<'_>,
    multiplier: bool,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    use super::{ArmourSlot, Defence};
    let q = Quantity::Poe1(Poe1Quantity::CritPerShieldEnergyShield(multiplier));
    if !phases::has(calc, q) {
        return Ok(None);
    }
    let es = phases::piece(calc, ArmourSlot::Weapon2, Defence::EnergyShield, reasons)?;
    Ok(Some(per_value(
        calc,
        q,
        es,
        SHIELD_ENERGY_SHIELD_STEP,
        None,
        cx,
        reasons,
    )?))
}

/// Arcane Might's share of the spell damage increases for an attack hit in
/// `cx` (P6-11hk): the increases that need `SPELL`, which match the hit's
/// context with `ATTACK` swapped for `SPELL` and not without either,
/// times the largest [`Poe1Quantity::SpellDamageToAttacks`] over 100.
/// `None` without the effect. The engine's `SPELL` stands for PoB's
/// `ModFlag.Spell` and `KeywordFlag.Spell` both; PoB copies only the
/// first, so a spell keyword's increase is counted here too.
fn spell_damage_to_attacks(
    calc: &mut Calc<'_>,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    let q = Quantity::Poe1(Poe1Quantity::SpellDamageToAttacks);
    let Some(best) = calc
        .store()
        .effects()
        .iter()
        .filter(|e| {
            e.quantity == q && e.op == Op::Base && matches!(e.conditions.applies(cx), Applies::Yes)
        })
        .max_by(|a, b| a.value.total_cmp(&b.value))
        .cloned()
    else {
        return Ok(None);
    };
    let neither = cx.flags.without(Flags::ATTACK);
    let spell_cx = calc.context(neither | Flags::SPELL);
    let neither_cx = calc.context(neither);
    let with_spell = sum_in(
        calc,
        Quantity::Damage,
        Op::Increased,
        &spell_cx,
        Term::Step("spell-damage-increases"),
        reasons,
    )?;
    let without = sum_in(
        calc,
        Quantity::Damage,
        Op::Increased,
        &neither_cx,
        Term::Step("untyped-damage-increases"),
        reasons,
    )?;
    let spell_only = difference(
        calc,
        with_spell,
        without,
        Term::Step("spell-only-increases"),
    )?;
    let share = leaf(calc, best.source.clone(), q, best.value)?;
    let share = percent(calc, share, Term::Step("arcane-might-share"))?;
    Ok(Some(phases::product(
        calc,
        &[spell_only, share],
        Term::Step("arcane-might"),
    )?))
}

/// The share of a pool's increases a Transfiguration gives as damage
/// (PoB's `data.misc.Transfiguration`, `Data.lua` 252).
const TRANSFIGURATION: f64 = 0.3;

/// The shield's armour and evasion per step of "Attack Damage per 75 Armour
/// or Evasion Rating on Shield" (PoB's `div = 75`).
const SHIELD_DAMAGE_STEP: f64 = 75.0;

/// The shield's maximum energy shield per step of the crit lines (PoB's
/// `div = 10`).
const SHIELD_ENERGY_SHIELD_STEP: f64 = 10.0;

/// The shield's armour or evasion per step of a shield attack's added
/// damage (PoB's `div = 15`).
const SHIELD_DEFENCE_STEP: f64 = 15.0;

/// The values of the weapon in `hand` (module docs, step 8).
pub(super) fn weapon_data(
    calc: &mut Calc<'_>,
    item: ItemId,
    hand: Hand,
    reasons: &mut Reasons,
) -> Result<WeaponData, CalcError> {
    let weapon = |v| hand.weapon(v);
    let cx = calc.context(Flags::NONE);
    // "No Physical Damage" (PoB: the weapon's physical damage removed) is
    // not modelled; the canonical increased physical damage line carries
    // the stat as 0.
    let no_physical = calc
        .store()
        .effects()
        .iter()
        .any(|e| e.quantity == weapon(WeaponValue::NoPhysicalDamage) && e.value != 0.0);
    if no_physical {
        reasons.partial.insert(UnsupportedReason::UnmodelledItem {
            item,
            slot: hand.slot().to_owned(),
            what: "no-physical-damage",
        });
    }
    // "Quality does not increase Physical Damage" (P3-8n, PoB's
    // `AlternateQualityWeapon`, `Classes/Item.lua` 2257): the quality
    // scalar is 0. Queried only when the store has the flag.
    let alternate = flag_on(calc, weapon(WeaponValue::AlternateQuality), &cx, reasons)?;
    let quality = match alternate {
        Some(flag) => by_flag(calc, flag, "alternate-quality", 0.0)?,
        None => sum_in(
            calc,
            weapon(WeaponValue::Quality),
            Op::Base,
            &cx,
            Term::Step("weapon-quality"),
            reasons,
        )?,
    };
    let mut damage = [None; 5];
    for (slot, t) in damage.iter_mut().zip(TYPES) {
        let mut range = Vec::new();
        for b in Bound::ALL {
            let q = weapon(WeaponValue::Damage(t, b));
            let base = sum_in(calc, q, Op::Base, &cx, Term::Base, reasons)?;
            let value = match t {
                // `round(min x (1 + physInc / 100) x (1 + quality / 100))`.
                DamageType::Physical => {
                    let inc = increased(calc, q, &cx, reasons)?;
                    let quality = phases::factor(calc, quality, Term::Step("quality"))?;
                    let raw =
                        phases::product(calc, &[base, inc, quality], Term::Step("unrounded"))?;
                    phases::round(calc, raw, Term::Step("weapon-damage"))?
                }
                // `round(min x (1 + localInc / 100))`.
                DamageType::Fire | DamageType::Cold | DamageType::Lightning => {
                    let inc = increased(calc, q, &cx, reasons)?;
                    let raw = phases::product(calc, &[base, inc], Term::Step("unrounded"))?;
                    phases::round(calc, raw, Term::Step("weapon-damage"))?
                }
                DamageType::Chaos => base,
            };
            range.push(value);
        }
        if range[0].value > 0.0 && range[1].value > 0.0 {
            *slot = Some([range[0], range[1]]);
        }
    }
    // The game's model (Q11, divergence from PoB): the attack time in
    // integer milliseconds, floored after the local increases, `floor(time
    // x 100 / (100 + inc))`; the rate `1000 / ms`. Computed from the integer
    // sums so the floor is exact (`667 / (1 - 42 / 100)` is 1149.99... in
    // doubles, `66700 / 58` is 1150). PoB instead rounds: `round(round(1000 /
    // time, 2) x (1 + inc / 100), 2)`. A reduction of 100% or more gives a
    // rate of 0 or below, as PoB's formula does; no reason is added.
    // A minion's weapon: PoB's rate `1 / attackTime` as is (P6-7c; no local
    // modifier reaches it).
    let own_rate = weapon(WeaponValue::AttackRate);
    if calc
        .store()
        .effects()
        .iter()
        .any(|e| e.quantity == own_rate)
    {
        let rate = sum_in(
            calc,
            own_rate,
            Op::Base,
            &cx,
            Term::Step("weapon-rate"),
            reasons,
        )?;
        let crit = sum_in(
            calc,
            weapon(WeaponValue::CritChance),
            Op::Base,
            &cx,
            Term::Step("weapon-crit"),
            reasons,
        )?;
        return Ok(WeaponData { damage, rate, crit });
    }
    let time = sum_in(
        calc,
        weapon(WeaponValue::AttackTime),
        Op::Base,
        &cx,
        Term::Step("attack-time"),
        reasons,
    )?;
    if time.value <= 0.0 {
        // A base without an attack time would give an infinite rate.
        reasons
            .indeterminate
            .insert(IndeterminateReason::MissingData {
                table: "weapon_types",
                key: "attack_time".to_owned(),
            });
    }
    let inc = sum_in(
        calc,
        weapon(WeaponValue::AttackSpeed),
        Op::Increased,
        &cx,
        Term::Step("increased-sum"),
        reasons,
    )?;
    let hundred = phases::constant(calc, "hundred", 100.0)?;
    let scaled = phases::product(calc, &[time, hundred], Term::Step("attack-time-scaled"))?;
    let divisor = phases::total(calc, &[hundred, inc], Term::Step("attack-speed-percent"))?;
    let ms = quotient(calc, scaled, divisor, Term::Step("attack-time-unfloored"))?;
    let ms = floor(calc, ms, Term::Step("attack-time-ms"))?;
    let thousand = phases::constant(calc, "milliseconds", 1000.0)?;
    let rate = quotient(calc, thousand, ms, Term::Step("weapon-rate"))?;
    // `CritChance = round((CritChanceBase + local) x (1 + local inc / 100),
    // 2)`.
    let crit = sum_in(
        calc,
        weapon(WeaponValue::CritChance),
        Op::Base,
        &cx,
        Term::Step("weapon-crit-base"),
        reasons,
    )?;
    let inc = increased(calc, weapon(WeaponValue::CritChance), &cx, reasons)?;
    let crit = phases::product(calc, &[crit, inc], Term::Step("weapon-crit-unrounded"))?;
    let crit = round_2(calc, crit, Term::Step("weapon-crit"))?;
    Ok(WeaponData { damage, rate, crit })
}

/// Accuracy rating (module docs, step 9): `max(0, floor(base x (1 + inc /
/// 100) x more))`.
fn accuracy(
    calc: &mut Calc<'_>,
    cx: &QueryContext,
    hand: Hand,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let input = *calc.input();
    // PoB: `m_max(1, m_min(100, build.characterLevel))`, as `collect`.
    let level = input
        .build
        .character
        .level
        .as_ref()
        .map_or(1, |l| *l.value())
        .clamp(1, 100);
    let per_level = input
        .data
        .character_constant(ACCURACY_PER_LEVEL_STAT)
        .unwrap_or_else(|| {
            reasons
                .indeterminate
                .insert(IndeterminateReason::MissingData {
                    table: "character_constants",
                    key: ACCURACY_PER_LEVEL_STAT.to_owned(),
                });
            0
        });
    // `Accuracy` BASE `per` with `{ type = "Multiplier", var = "Level",
    // base = -per }`: `per x level - per`.
    #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
    let from_level = (per_level * i64::from(level) - per_level) as f64;
    let mut terms = vec![leaf(
        calc,
        Source::Character,
        Quantity::Accuracy,
        from_level,
    )?];
    // No accuracy from dexterity with Supreme Prodigy (P6-11hx).
    if let Some((node, value)) = calc
        .field(Field::Dexterity)
        .and_then(|f| Some((f.node?, f.value?)))
        .filter(|_| !phases::no_attribute_bonuses(calc))
    {
        let per = phases::constant(calc, "accuracy-per-dexterity", ACCURACY_PER_DEXTERITY)?;
        terms.push(phases::product(
            calc,
            &[Valued { node, value }, per],
            Term::Step("from-dexterity"),
        )?);
    }
    // "Gain Accuracy Rating equal to twice your Strength" (P6-11gm; PoB's
    // `Accuracy` BASE 2 per strength).
    let from_strength = Quantity::Poe1(Poe1Quantity::AccuracyFromStrength);
    if phases::has(calc, from_strength)
        && flag_on(calc, from_strength, cx, reasons)?.is_some()
        && let Some((node, value)) = calc
            .field(Field::Strength)
            .and_then(|f| Some((f.node?, f.value?)))
    {
        let per = phases::constant(calc, "accuracy-per-strength", ACCURACY_PER_STRENGTH)?;
        terms.push(phases::product(
            calc,
            &[Valued { node, value }, per],
            Term::Step("from-strength"),
        )?);
    }
    // "Intelligence is added to Accuracy Rating with Wands" (the agent
    // list, S8; PoB's `Accuracy` BASE 1 per intelligence, `PerStat`
    // `floor(int + 0.0001)` of a whole number): only in a wand's context,
    // the row's flag.
    let from_intelligence = Quantity::Poe1(Poe1Quantity::AccuracyFromIntelligence);
    if phases::has(calc, from_intelligence)
        && flag_on(calc, from_intelligence, cx, reasons)?.is_some()
        && let Some((node, value)) = calc
            .field(Field::Intelligence)
            .and_then(|f| Some((f.node?, f.value?)))
    {
        terms.push(phases::total(
            calc,
            &[Valued { node, value }],
            Term::Step("from-intelligence"),
        )?);
    }
    terms.push(sum_in(
        calc,
        Quantity::Accuracy,
        Op::Base,
        cx,
        Term::Step("accuracy-base-effects"),
        reasons,
    )?);
    // The hand's weapon's own accuracy (its `MainHandAttack` or
    // `OffHandAttack` condition), only when it has some.
    let local = hand.weapon(WeaponValue::Accuracy);
    if phases::has(calc, local) {
        terms.push(sum_in(
            calc,
            local,
            Op::Base,
            cx,
            Term::Step("weapon-accuracy"),
            reasons,
        )?);
    }
    let base = phases::total(calc, &terms, Term::Base)?;
    let (inc, more) = modifiers(calc, Quantity::Accuracy, cx, reasons)?;
    let raw = phases::product(calc, &[base, inc, more], Term::Step("accuracy-unrounded"))?;
    let rounded = floor(calc, raw, Term::Step("accuracy-floor"))?;
    let zero = phases::constant(calc, "no-negative-accuracy", 0.0)?;
    phases::pair(calc, rounded, zero, Term::Step("accuracy"), false)
}

/// PoB's `calcs.hitChance(enemyEvasion, accuracy)` (module docs, step 9).
fn accuracy_hit_chance(
    calc: &mut Calc<'_>,
    enemy: &Enemy,
    accuracy: Valued,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    // `enemyEvasion = m_max(round(calcLib.val(enemyDB, "Evasion")), 0)`:
    // the base (the enemy model's) times its INC and MORE, which only
    // Blind gives ([`Poe1Quantity::EnemyEvasion`], P3-12g). Without such an
    // effect the graph keeps the base alone.
    let base = leaf(
        calc,
        Source::Enemy,
        Quantity::Evasion,
        enemy.evasion.unwrap_or(0.0),
    )?;
    let quantity = Quantity::Poe1(Poe1Quantity::EnemyEvasion);
    let evasion = if calc
        .store()
        .effects()
        .iter()
        .any(|e| e.quantity == quantity)
    {
        let (inc, more) = modifiers(calc, quantity, cx, reasons)?;
        phases::product(
            calc,
            &[base, inc, more],
            Term::Step("enemy-evasion-modified"),
        )?
    } else {
        base
    };
    let evasion = phases::round(calc, evasion, Term::Step("enemy-evasion-rounded"))?;
    let zero = phases::constant(calc, "no-negative-evasion", 0.0)?;
    let evasion = phases::pair(calc, evasion, zero, Term::Step("enemy-evasion"), false)?;
    pob_hit_chance(calc, evasion, accuracy, Term::Result)
}

/// PoB's `calcs.hitChance(evasion, accuracy)` (`CalcDefence.lua` 57 to 63)
/// on settled values: `max(min(round(accuracy / (accuracy + (evasion / 5)
/// ^ 0.9) x 125), 100), 5)`. The character's attacks against the enemy
/// and the enemy's against the character (P6-10b part 11) read it.
///
/// # Errors
/// A graph error.
pub(super) fn pob_hit_chance(
    calc: &mut Calc<'_>,
    evasion: Valued,
    accuracy: Valued,
    term: Term,
) -> Result<Valued, CalcError> {
    // `accuracy / (accuracy + (evasion / 5) ^ 0.9) * 125`. Accuracy 0
    // against evasion 0 is 0 / 0 (NaN, as in PoB, whose result then rests
    // on Lua's `math.min`); the level and dexterity give every character
    // some accuracy, so it is not reached.
    let five = phases::constant(calc, "evasion-divisor", 5.0)?;
    let fifth = quotient(calc, evasion, five, Term::Step("evasion-fifth"))?;
    let exponent = phases::constant(calc, "evasion-exponent", 0.9)?;
    let powered = power(calc, fifth, exponent, Term::Step("evasion-power"))?;
    let denominator = phases::total(calc, &[accuracy, powered], Term::Step("hit-denominator"))?;
    let share = quotient(calc, accuracy, denominator, Term::Step("hit-share"))?;
    let scale = phases::constant(calc, "hit-chance-scale", 125.0)?;
    let raw = phases::product(calc, &[share, scale], Term::Step("hit-chance-unrounded"))?;
    let rounded = phases::round(calc, raw, Term::Step("hit-chance-rounded"))?;
    let cap = phases::constant(calc, "hit-chance-cap", 100.0)?;
    let capped = phases::pair(calc, rounded, cap, Term::Step("hit-chance-capped"), true)?;
    let floor = phases::constant(calc, "hit-chance-floor", 5.0)?;
    let value = phases::pair(calc, capped, floor, term, false)?;
    Ok(value)
}

/// A spell's base crit chance: PoB's `critChance`, the level's attack crit
/// chance when not 0, else its spell crit chance, in hundredths (P3-2's
/// data parity rule).
fn skill_crit_base(
    calc: &mut Calc<'_>,
    source: Source,
    level: &StatSetLevel,
) -> Result<Valued, CalcError> {
    let hundredths = if level.attack_crit_chance == 0 {
        level.spell_crit_chance
    } else {
        level.attack_crit_chance
    };
    #[expect(
        clippy::cast_precision_loss,
        reason = "crit chances are small integers, exact as f64"
    )]
    let base = hundredths as f64 / 100.0;
    leaf(
        calc,
        source,
        Quantity::Poe1(Poe1Quantity::SkillCritChance),
        base,
    )
}

/// Critical strike chance (module docs, step 4; `calcs.offence`, "Calculate
/// crit chance"): `skill_base` is a spell's own or an attack's weapon's;
/// `hit_chance` an attack's accuracy hit chance (`None`: a spell's 100).
fn crit_chance(
    calc: &mut Calc<'_>,
    skill_base: Valued,
    hit_chance: Option<Valued>,
    cx: &QueryContext,
) -> Result<(Valued, Reasons), CalcError> {
    let mut reasons = Reasons::new();
    // `NeverCrit`: 0 (`CalcOffence.lua` 2955; Resolute Technique, P3-8m).
    let never = Quantity::Poe1(Poe1Quantity::NeverCrit);
    if let Some(flag) = flag_on(calc, never, cx, &mut reasons)? {
        return Ok((by_flag(calc, flag, "never-crit", 0.0)?, reasons));
    }
    let added = sum_in(
        calc,
        Quantity::CritChance,
        Op::Base,
        cx,
        Term::Step("crit-base-effects"),
        &mut reasons,
    )?;
    let added = match per_shield_energy_shield(calc, false, cx, &mut reasons)? {
        Some(extra) => phases::total(calc, &[added, extra], Term::Step("crit-base-with-shield"))?,
        None => added,
    };
    // `(baseCrit + base)`.
    let base = phases::total(calc, &[skill_base, added], Term::Base)?;
    let (inc, more) = modifiers(calc, Quantity::CritChance, cx, &mut reasons)?;
    let raw = phases::product(calc, &[base, inc, more], Term::Step("unrounded"))?;
    let cap = phases::constant(calc, "crit-chance-cap", CRIT_CHANCE_CAP)?;
    let mut chance = phases::pair(calc, raw, cap, Term::Step("capped"), true)?;
    if base.value > 0.0 {
        let zero = phases::constant(calc, "no-negative-crit-chance", 0.0)?;
        chance = phases::pair(calc, chance, zero, Term::Step("at-least-zero"), false)?;
    }
    // `CritChance x AccuracyHitChance / 100`.
    let hit_chance = match hit_chance {
        Some(h) => h,
        None => phases::constant(calc, "spell-hit-chance", SPELL_HIT_CHANCE)?,
    };
    let scaled = phases::product(calc, &[chance, hit_chance], Term::Step("hit-chance-scaled"))?;
    let value = percent(calc, scaled, Term::Result)?;
    Ok((value, reasons))
}

/// Critical strike multiplier (module docs, step 4): `1 + max(0,
/// round(extra x 1, 2))`, `extra` the `CritMultiplier` BASE sum / 100 (the
/// enemy adds nothing modelled).
fn crit_multiplier(
    calc: &mut Calc<'_>,
    data: &dyn CalcData,
    cx: &QueryContext,
) -> Result<(Valued, Reasons), CalcError> {
    let mut reasons = Reasons::new();
    // `NeverCrit`: 0 (`CalcOffence.lua` 2958); else `NoCritMultiplier`: 1
    // (3115; Elemental Overload). P3-8m.
    let never = Quantity::Poe1(Poe1Quantity::NeverCrit);
    if let Some(flag) = flag_on(calc, never, cx, &mut reasons)? {
        return Ok((by_flag(calc, flag, "never-crit-multiplier", 0.0)?, reasons));
    }
    let no_extra = Quantity::Poe1(Poe1Quantity::NoCritMultiplier);
    if let Some(flag) = flag_on(calc, no_extra, cx, &mut reasons)? {
        return Ok((by_flag(calc, flag, "no-extra-crit-damage", 1.0)?, reasons));
    }
    let mut terms = Vec::new();
    // A minion's base is the monster template's (`initMinionModDB`,
    // `CalcPerform.lua` 1146; P6-7c), the character's the character's.
    let base = match calc.actor() {
        crate::engine::Actor::Player => data.character_constant(CRIT_MULTIPLIER_STAT),
        crate::engine::Actor::Minion => data.monster_constant(CRIT_MULTIPLIER_STAT),
    };
    match base {
        Some(base) => {
            #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
            let extra = (base - 100) as f64;
            terms.push(leaf(
                calc,
                Source::Character,
                Quantity::CritMultiplier,
                extra,
            )?);
        }
        None => {
            reasons
                .indeterminate
                .insert(IndeterminateReason::MissingData {
                    table: "character_constants",
                    key: CRIT_MULTIPLIER_STAT.to_owned(),
                });
        }
    }
    terms.push(sum_in(
        calc,
        Quantity::CritMultiplier,
        Op::Base,
        cx,
        Term::Step("crit-multiplier-effects"),
        &mut reasons,
    )?);
    if let Some(extra) = per_shield_energy_shield(calc, true, cx, &mut reasons)? {
        terms.push(extra);
    }
    let sum = phases::total(calc, &terms, Term::Base)?;
    let extra = percent(calc, sum, Term::Step("extra-damage"))?;
    let rounded = round_2(calc, extra, Term::Step("extra-damage-rounded"))?;
    let zero = phases::constant(calc, "no-negative-extra-damage", 0.0)?;
    let extra = phases::pair(
        calc,
        rounded,
        zero,
        Term::Step("extra-at-least-zero"),
        false,
    )?;
    let one = phases::constant(calc, "one", 1.0)?;
    let value = phases::total(calc, &[one, extra], Term::Result)?;
    Ok((value, reasons))
}

/// Cast rate (module docs, step 6).
fn cast_rate(
    calc: &mut Calc<'_>,
    spell: &SpellSkill<'_>,
    cx: &QueryContext,
) -> Result<(Valued, Reasons), CalcError> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "cast times are small integers, exact as f64"
    )]
    let seconds = spell.cast_time as f64 / 1000.0;
    // A trap's or a mine's spell (P6-6e): PoB replaces its time with the
    // throwing or laying time (`skillData.timeOverride`, `CalcOffence.lua`
    // 1223 to 1337, 2208): `1 / min(1 / base x action speed, tick rate)`,
    // a mine's when it is both (PoB's mine path sets the time), one
    // thrown (throw counts are not read: the reason). Cast speed does not
    // reach it. The throwing or laying speed modifiers (P6-11cy):
    // `baseSpeed x calcLib.mod(TrapThrowingSpeed)` (or `MineLayingSpeed`)
    // `x ActionSpeedMod` (1229, 1317).
    let mine = spell.skill.types.contains("RemoteMined");
    if mine || spell.skill.types.contains("Trapped") {
        let base = if mine {
            MINE_LAYING_TIME
        } else {
            TRAP_THROWING_TIME
        };
        let base = phases::constant(
            calc,
            if mine {
                "mine-laying-time"
            } else {
                "trap-throwing-time"
            },
            base,
        )?;
        let one = phases::constant(calc, "one", 1.0)?;
        let speed = quotient(calc, one, base, Term::Step("throwing-speed"))?;
        let mut reasons = Reasons::new();
        let q = Quantity::Poe1(if mine {
            Poe1Quantity::MineLayingSpeed
        } else {
            Poe1Quantity::TrapThrowingSpeed
        });
        let speed = if phases::has(calc, q) {
            let (inc, more) = modifiers_of(calc, q, cx, &mut reasons)?;
            phases::product(
                calc,
                &[speed, inc, more],
                Term::Step("throwing-speed-modified"),
            )?
        } else {
            speed
        };
        let speed = action_speed(calc, speed, &mut reasons)?;
        // The throw counts (P6-11fa; `CalcOffence.lua` 1230 to 1237, 1319 to
        // 1336): the trap count for a trap, the mine count for a mine (and
        // both for a trap thrown by a mine). A mine's speed is divided by `1
        // + (count - 1) x 0.1` before the tick cap; the time is then divided
        // by each count, so the rate is the capped speed times them.
        let trap = spell.skill.types.contains("Trapped");
        let cooldown = spell.cooldown != 0;
        let mut counts = Vec::new();
        if mine {
            counts.push(throw_count(calc, true, cooldown, cx, &mut reasons)?);
        }
        if trap {
            counts.push(throw_count(calc, false, cooldown, cx, &mut reasons)?);
        }
        let speed = if mine {
            let count = counts[0];
            if count.value >= 1.0 {
                let one = phases::constant(calc, "one", 1.0)?;
                let extra = minus(calc, count, one, Term::Step("additional-mines"))?;
                let tenth = phases::constant(calc, "additional-mine-time", 0.1)?;
                let extra =
                    phases::product(calc, &[extra, tenth], Term::Step("additional-mine-penalty"))?;
                let penalty = phases::total(calc, &[one, extra], Term::Step("mine-throw-penalty"))?;
                quotient(calc, speed, penalty, Term::Step("mine-laying-speed"))?
            } else {
                speed
            }
        } else {
            speed
        };
        let cap = phases::constant(calc, "server-tick-rate", 1.0 / SERVER_TICK_TIME)?;
        let capped = phases::pair(calc, speed, cap, Term::Step("throwing-speed-capped"), true)?;
        let mut terms = vec![capped];
        terms.extend(counts);
        let value = phases::product(calc, &terms, Term::Result)?;
        return Ok((value, reasons));
    }
    let cast_time = leaf(
        calc,
        spell.source(),
        Quantity::Poe1(Poe1Quantity::CastTime),
        seconds,
    )?;
    let mut cooldown_reasons = Reasons::new();
    let limit = cooldown_limit(calc, spell, cx, &mut cooldown_reasons)?;
    let (value, mut reasons) = rate(
        calc,
        cast_time,
        cx,
        !spell.skill.types.contains("Channel"),
        !spell.skill.types.contains("SummonsTotem"),
        limit,
        RateExtras::default(),
    )?;
    reasons.extend(&cooldown_reasons);
    Ok((value, reasons))
}

/// A throw count (P6-11fa): the config's `minesPerThrow` or `trapsPerThrow`
/// when set (an `OVERRIDE`), else 1 for a skill with a cooldown (P6-11hp;
/// `CalcOffence.lua` 1231 to 1233, 1321 to 1323), else `calcLib.val`: the
/// base 1 plus the counts' BASE sum (no INC or MORE is mapped). A value of
/// another type is the input's reason.
fn throw_count(
    calc: &mut Calc<'_>,
    mine: bool,
    cooldown: bool,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let input = if mine {
        MINES_PER_THROW_INPUT
    } else {
        TRAPS_PER_THROW_INPUT
    };
    let set = super::collect::active_config(calc.input().build);
    match set.and_then(|s| super::config::last_input(s, input, loom_domain::ConfigInputKind::Input))
    {
        Some(loom_domain::ConfigValue::Number(n)) => {
            return phases::constant(calc, "throw-count-configured", n.value().0);
        }
        None => {}
        Some(_) => {
            reasons.partial.insert(UnsupportedReason::UnmodelledConfig {
                input: input.to_owned(),
            });
        }
    }
    let q = Quantity::Poe1(if mine {
        Poe1Quantity::MineThrowCount
    } else {
        Poe1Quantity::TrapThrowCount
    });
    if cooldown {
        return phases::constant(calc, "throw-count-cooldown", 1.0);
    }
    let base = phases::constant(calc, "throw-count-base", 1.0)?;
    if !phases::has(calc, q) {
        return Ok(base);
    }
    let added = sum_in(
        calc,
        q,
        Op::Base,
        cx,
        Term::Step("throw-count-added"),
        reasons,
    )?;
    phases::total(calc, &[base, added], Term::Step("throw-count"))
}

/// The rate a cooldown allows (P6-7c3b): PoB's `calcSkillCooldown`
/// (`CalcOffence.lua` 274 to 288), the level's cooldown, rounded up to a
/// server tick (`ceil(cooldown x tick rate) / tick rate`) unless the skill
/// stores more than one use; then `Speed = min(Speed, 1 / Cooldown x
/// Repeats)` (2357 to 2360; the repeats in [`rate`]). The cooldown is
/// divided by `calcLib.mod(CooldownRecovery)` first (P6-11ex: "#% increased
/// Cooldown Recovery Rate", P6-11bs's `SkillTiming::CooldownRecovery`, and
/// the typed ones, [`Poe1Quantity::TypedCooldownRecovery`]; no MORE is
/// mapped); a recovery of -100% or less is a `cooldown-recovery` reason. No
/// added cooldown or additional use stat is mapped (each is unmapped, a
/// reason). `None` without a cooldown.
fn cooldown_limit(
    calc: &mut Calc<'_>,
    spell: &SpellSkill<'_>,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    if spell.cooldown == 0 {
        return Ok(None);
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a cooldown in milliseconds, exact as f64"
    )]
    let seconds = spell.cooldown as f64 / 1000.0;
    let base = phases::constant(calc, "skill-cooldown", seconds)?;
    let global = Quantity::Poe1(Poe1Quantity::SkillTiming(
        super::skill_modifiers::SkillTiming::CooldownRecovery,
    ));
    let typed = Quantity::Poe1(Poe1Quantity::TypedCooldownRecovery);
    let cooldown = if phases::has(calc, global) || phases::has(calc, typed) {
        let a = sum_in(
            calc,
            global,
            Op::Increased,
            cx,
            Term::Step("cooldown-recovery"),
            reasons,
        )?;
        let b = sum_in(
            calc,
            typed,
            Op::Increased,
            cx,
            Term::Step("cooldown-recovery-typed"),
            reasons,
        )?;
        let inc = phases::total(calc, &[a, b], Term::Step("cooldown-recovery-sum"))?;
        if inc.value <= -100.0 {
            reasons.partial.insert(UnsupportedReason::UnmodelledSkill {
                source: spell.source(),
                what: "cooldown-recovery",
            });
            return Ok(None);
        }
        let factor = phases::factor(calc, inc, Term::Step("cooldown-recovery-factor"))?;
        quotient(calc, base, factor, Term::Step("cooldown-recovered"))?
    } else {
        base
    };
    let cooldown = if spell.stored_uses > 1 {
        cooldown
    } else {
        let tick_rate = phases::constant(calc, "server-tick-rate", 1.0 / SERVER_TICK_TIME)?;
        let ticks = phases::product(calc, &[cooldown, tick_rate], Term::Step("cooldown-ticks"))?;
        let value = crate::math::ceil(ticks.value);
        let node = calc.graph_mut().push(
            Node::Ceil {
                term: Term::Step("cooldown-ticks-up"),
                of: ticks.node,
            },
            value,
        )?;
        quotient(
            calc,
            Valued { node, value },
            tick_rate,
            Term::Step("cooldown-rounded"),
        )?
    };
    let one = phases::constant(calc, "one", 1.0)?;
    Ok(Some(quotient(
        calc,
        one,
        cooldown,
        Term::Step("cooldown-rate"),
    )?))
}

/// Attack rate (module docs, step 6): the weapon's rate times the skill's
/// attack speed multiplier (`source.AttackRate x (1 +
/// attackSpeedMultiplier / 100)`) gives the base time, `1 / rate`.
fn attack_rate(
    calc: &mut Calc<'_>,
    attack: &AttackSkill<'_>,
    weapon_rate: Valued,
    hand: Hand,
    cx: &QueryContext,
) -> Result<(Valued, Reasons), CalcError> {
    #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
    let multiplier = attack.attack_speed_multiplier as f64;
    let multiplier = leaf(
        calc,
        attack.source(),
        Quantity::Poe1(Poe1Quantity::AttackSpeedMultiplier),
        multiplier,
    )?;
    let multiplier = phases::factor(calc, multiplier, Term::Step("attack-speed-multiplier"))?;
    let skill_rate = phases::product(
        calc,
        &[weapon_rate, multiplier],
        Term::Step("skill-attack-rate"),
    )?;
    let one = phases::constant(calc, "one", 1.0)?;
    let base_time = quotient(calc, one, skill_rate, Term::Step("attack-time"))?;
    // The off hand's own more attack speed (P6-11gx: "#% more Attack speed
    // with Offhand", PoB's `OffHandAttack` condition), in the more product.
    let off_hand = Quantity::Poe1(Poe1Quantity::OffHandSpeedMore);
    let mut off_reasons = Reasons::new();
    let extra = if hand == Hand::Off && phases::has(calc, off_hand) {
        Some(more(calc, off_hand, cx, &mut off_reasons)?)
    } else {
        None
    };
    // "#% increased Attack Speed per 150 Accuracy Rating" (P6-11hd; PoB's
    // `Speed` INC with `PerStat` `Accuracy`, read with the hand's final
    // accuracy): one more term of the speed's increase.
    let per_accuracy = Quantity::Poe1(Poe1Quantity::AttackSpeedPerAccuracy);
    let mut extra_inc = Vec::new();
    if phases::has(calc, per_accuracy) {
        let accuracy = accuracy(calc, cx, hand, &mut off_reasons)?;
        extra_inc.push(per_value(
            calc,
            per_accuracy,
            accuracy,
            ATTACK_SPEED_ACCURACY_STEP,
            None,
            cx,
            &mut off_reasons,
        )?);
    }
    let (value, mut reasons) = rate(
        calc,
        base_time,
        cx,
        !attack.skill.types.contains("Channel"),
        !attack.skill.types.contains("SummonsTotem"),
        None,
        RateExtras {
            more: extra,
            inc: &extra_inc,
        },
    )?;
    reasons.extend(&off_reasons);
    Ok((value, reasons))
}

/// Computed terms of a rate's speed modifier beyond the store's: a more
/// multiplier (P6-11gx, the off hand's own) and increases (P6-11hd, per
/// accuracy).
#[derive(Default)]
struct RateExtras<'a> {
    more: Option<Valued>,
    inc: &'a [Valued],
}

/// The accuracy per step of "#% increased Attack Speed per 150 Accuracy
/// Rating": the game's text says 150 (the stat ID keeps an older 200), and
/// PoB reads the text (`ModParser.lua` 1490).
const ATTACK_SPEED_ACCURACY_STEP: f64 = 150.0;

/// `min(1 / (base time / round((1 + inc / 100) x more, 2)), 1 / 0.033)`,
/// with a cooldown's `limit` taken before the tick cap.
fn rate(
    calc: &mut Calc<'_>,
    base_time: Valued,
    cx: &QueryContext,
    capped: bool,
    self_cast: bool,
    limit: Option<Valued>,
    extras: RateExtras<'_>,
) -> Result<(Valued, Reasons), CalcError> {
    // Stats that change the rate or DPS a way not modelled (PoB's
    // `showAverage`), read here: a trigger's rate never reaches this
    // function (P6-8a, [`super::mapping::HIT_REASONS`]).
    let mut reasons = phases::marker_reasons(
        calc,
        |q| match q {
            Quantity::Poe1(Poe1Quantity::HitReason(stat)) => Some(stat),
            _ => None,
        },
        |_| false,
    );
    let (inc, more) = if extras.inc.is_empty() {
        modifiers(calc, Quantity::Speed, cx, &mut reasons)?
    } else {
        let inc = increased_with(calc, Quantity::Speed, cx, extras.inc, &mut reasons)?;
        let more = more(calc, Quantity::Speed, cx, &mut reasons)?;
        (inc, more)
    };
    let more = match extras.more {
        Some(extra) => phases::product(calc, &[more, extra], Term::More)?,
        None => more,
    };
    let modifier = phases::product(calc, &[inc, more], Term::Step("modifier"))?;
    let modifier = round_2(calc, modifier, Term::Step("modifier-rounded"))?;
    let time = quotient(calc, base_time, modifier, Term::Step("time"))?;
    let one = phases::constant(calc, "one", 1.0)?;
    let rate = quotient(calc, one, time, Term::Step("uncapped"))?;
    // The player's action speed reaches a self-cast skill only; a totem
    // has its own (P6-6d, module docs of `NOT_SELF_CAST`).
    let rate = if self_cast {
        action_speed(calc, rate, &mut reasons)?
    } else {
        rate
    };
    // Each repeat is a use: the caps are times the repeat count (P6-11aj;
    // `CalcOffence.lua` 2359, 2367).
    let repeats = super::repeats::count(calc);
    let repeats = phases::constant(calc, "repeats", repeats)?;
    let rate = match limit {
        Some(limit) => {
            let limit =
                phases::product(calc, &[limit, repeats], Term::Step("cooldown-rate-repeats"))?;
            phases::pair(calc, rate, limit, Term::Step("cooldown-capped"), true)?
        }
        None => rate,
    };
    // A channelled skill's rate is not capped (`CalcOffence.lua` 2366; P6-5).
    if !capped {
        return Ok((rate, reasons));
    }
    let cap = phases::constant(calc, "server-tick-rate", 1.0 / SERVER_TICK_TIME)?;
    let cap = phases::product(
        calc,
        &[cap, repeats],
        Term::Step("server-tick-rate-repeats"),
    )?;
    let value = phases::pair(calc, rate, cap, Term::Result, true)?;
    Ok((value, reasons))
}

/// A self-cast skill's rate times PoB's action speed modifier
/// (`CalcOffence.lua` 2341: every skill the engine computes is self-cast;
/// `calcs.actionSpeedMod`, `CalcPerform.lua` 1107: `max(minimum / 100, 1 +
/// (temporal chains + ActionSpeed INC) / 100)`, with no minimum, maximum
/// reduction, or Temporal Chains mapped). Queried only when the store has
/// an action speed effect (P3-8n), so other hits keep their graphs. The
/// modifier is the character's: no skill flag.
fn action_speed(
    calc: &mut Calc<'_>,
    rate: Valued,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    if !phases::has(calc, Quantity::Poe1(Poe1Quantity::ActionSpeed)) {
        return Ok(rate);
    }
    let modifier = action_speed_modifier(calc, reasons)?;
    phases::product(calc, &[rate, modifier], Term::Step("action-speed-scaled"))
}

/// PoB's `calcs.actionSpeedMod` as the engine reads it (module docs of
/// [`action_speed`]): `max(minimum, 1 + ActionSpeed INC / 100)`, the
/// character's, with no skill flag; the minimum 1 with "Action Speed cannot
/// be modified to below Base Value" (P6-11ih, PoB's `MinimumActionSpeed`
/// MAX 100), else 0. The hit's rate and the effective movement speed
/// ([`super::movement`]) share it.
pub(super) fn action_speed_modifier(
    calc: &mut Calc<'_>,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let cx = calc.context(Flags::NONE);
    let modifier = increased(
        calc,
        Quantity::Poe1(Poe1Quantity::ActionSpeed),
        &cx,
        reasons,
    )?;
    // Queried only with an effect, so other builds keep their graphs.
    let quantity = Quantity::Poe1(Poe1Quantity::ActionSpeedNotBelowBase);
    let floor = if phases::has(calc, quantity) {
        let (store, graph) = calc.parts();
        let floor = store.flag(graph, quantity, &cx)?;
        reasons.extend(&floor.reasons);
        floor.value > 0.0
    } else {
        false
    };
    let minimum = if floor {
        phases::constant(calc, "minimum-action-speed", 1.0)?
    } else {
        phases::constant(calc, "no-minimum-action-speed", 0.0)?
    };
    phases::pair(calc, modifier, minimum, Term::Step("action-speed"), false)
}

/// PoB's `ScaledDamageEffect` from double damage (`CalcOffence.lua` 3196
/// to 3209): `1 + min(DoubleDamageChance, 100) / 100`, with no triple
/// damage or double damage on crit mapped (P3-8n). `None` when the store
/// has no double damage effect.
fn double_damage(
    calc: &mut Calc<'_>,
    cx: &QueryContext,
    uptime: Option<Valued>,
    weapon: Option<Quantity>,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    let quantity = Quantity::Poe1(Poe1Quantity::DoubleDamageChance);
    let weapon = weapon.filter(|q| phases::has(calc, *q));
    let has = phases::has(calc, quantity) || weapon.is_some();
    let triple_quantity = Quantity::Poe1(Poe1Quantity::TripleDamageChance);
    let has_triple = phases::has(calc, triple_quantity);
    if !has && uptime.is_none() && !has_triple {
        return Ok(None);
    }
    let mut chance = if has {
        sum_in(
            calc,
            quantity,
            Op::Base,
            cx,
            Term::Step("double-damage-chance"),
            reasons,
        )?
    } else {
        phases::constant(calc, "no-double-damage-chance", 0.0)?
    };
    // The weapon's own lines, 100 each (the agent list, 2026-11-27).
    if let Some(q) = weapon {
        let lines = sum_in(
            calc,
            q,
            Op::Base,
            cx,
            Term::Step("weapon-double-damage-lines"),
            reasons,
        )?;
        let hundred = phases::constant(calc, "hundred", 100.0)?;
        let from_weapon =
            phases::product(calc, &[lines, hundred], Term::Step("weapon-double-damage"))?;
        chance = phases::total(
            calc,
            &[chance, from_weapon],
            Term::Step("double-damage-with-weapon"),
        )?;
    }
    // `DoubleDamageChance + IntimidatingUpTimeRatio` (`CalcOffence.lua`
    // 3200 to 3201), capped below.
    if let Some(u) = uptime {
        chance = phases::total(calc, &[chance, u], Term::Step("exerted-double-damage"))?;
    }
    let hundred = phases::constant(calc, "hundred", 100.0)?;
    let chance = phases::pair(
        calc,
        chance,
        hundred,
        Term::Step("double-damage-capped"),
        true,
    )?;
    // Triple damage (P6-11hi; `CalcOffence.lua` 3192 to 3208): `min(T,
    // 100)`; it takes the double damage's place when both happen, `D - T x
    // D / 100`, and adds `2 x T / 100`.
    let (chance, triple) = if has_triple {
        let t = sum_in(
            calc,
            triple_quantity,
            Op::Base,
            cx,
            Term::Step("triple-damage-chance"),
            reasons,
        )?;
        let t = phases::pair(calc, t, hundred, Term::Step("triple-damage-capped"), true)?;
        let t_share = percent(calc, t, Term::Step("triple-damage-share"))?;
        let overlap = phases::product(calc, &[t_share, chance], Term::Step("double-and-triple"))?;
        let d = difference(calc, chance, overlap, Term::Step("double-without-triple"))?;
        let zero = phases::constant(calc, "no-negative-double-damage", 0.0)?;
        let d = phases::pair(calc, d, zero, Term::Step("double-at-least-zero"), false)?;
        let two = phases::constant(calc, "triple-damage-extra", 2.0)?;
        let t2 = phases::product(calc, &[t_share, two], Term::Step("triple-damage-effect"))?;
        (d, Some(t2))
    } else {
        (chance, None)
    };
    let effect = percent(calc, chance, Term::Step("double-damage-effect"))?;
    let one = phases::constant(calc, "one", 1.0)?;
    let mut terms = vec![one, effect];
    terms.extend(triple);
    Ok(Some(phases::total(
        calc,
        &terms,
        Term::Step("scaled-damage-effect"),
    )?))
}

/// `(1 + inc / 100)` and the more product of `quantity` in `cx` (for
/// other phases; Phase 6).
///
/// # Errors
/// A graph error.
pub(super) fn modifiers_of(
    calc: &mut Calc<'_>,
    quantity: Quantity,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<(Valued, Valued), CalcError> {
    modifiers(calc, quantity, cx, reasons)
}

/// The flag on `quantity` in `cx` when one applies (for other phases).
///
/// # Errors
/// A graph error.
pub(super) fn flag_of(
    calc: &mut Calc<'_>,
    quantity: Quantity,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    flag_on(calc, quantity, cx, reasons)
}

/// `a / b` with `term` (for other phases).
///
/// # Errors
/// A graph error.
pub(super) fn ratio(
    calc: &mut Calc<'_>,
    a: Valued,
    b: Valued,
    term: Term,
) -> Result<Valued, CalcError> {
    quotient(calc, a, b, term)
}

/// `a - b` with `term` (for other phases).
///
/// # Errors
/// A graph error.
pub(super) fn minus(
    calc: &mut Calc<'_>,
    a: Valued,
    b: Valued,
    term: Term,
) -> Result<Valued, CalcError> {
    difference(calc, a, b, term)
}

/// `x / 100` with `term` (for other phases).
///
/// # Errors
/// A graph error.
pub(super) fn percent_of(calc: &mut Calc<'_>, x: Valued, term: Term) -> Result<Valued, CalcError> {
    percent(calc, x, term)
}

/// `base ^ exponent` with `term` (for other phases).
///
/// # Errors
/// A graph error.
pub(super) fn power_of(
    calc: &mut Calc<'_>,
    base: Valued,
    exponent: Valued,
    term: Term,
) -> Result<Valued, CalcError> {
    power(calc, base, exponent, term)
}

/// A leaf of the main skill's own value (Phase 6: a skill's base damage
/// over time), as a `Base` contribution of `source`.
///
/// # Errors
/// A graph error.
pub(super) fn skill_leaf(
    calc: &mut Calc<'_>,
    source: Source,
    value: f64,
) -> Result<Valued, CalcError> {
    leaf(calc, source, Quantity::Damage, value)
}

/// Whether "Cannot deal <Type> Damage" holds for `t` in `cx` (PoB's
/// `canDeal`, `DealNo<Type>`).
///
/// # Errors
/// A graph error.
pub(super) fn cannot_deal(
    calc: &mut Calc<'_>,
    t: DamageType,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<bool, CalcError> {
    Ok(flag_on(calc, Quantity::Poe1(Poe1Quantity::DealNo(t)), cx, reasons)?.is_some())
}

/// `1 + inc / 100` and the more product of `Damage` in `cx` (a typed
/// context reads the type's names too), as a hit's damage reads them.
///
/// # Errors
/// A graph error.
pub(super) fn damage_modifiers(
    calc: &mut Calc<'_>,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<(Valued, Valued), CalcError> {
    let extras = damage_per_stat(calc, cx, reasons)?;
    if extras.is_empty() {
        modifiers(calc, Quantity::Damage, cx, reasons)
    } else {
        let inc = increased_with(calc, Quantity::Damage, cx, &extras, reasons)?;
        let more = more(calc, Quantity::Damage, cx, reasons)?;
        Ok((inc, more))
    }
}

/// A damage increase per some of a computed field (PoB's `PerStat` tag on
/// `Damage` INC): the quantity that holds the per-step value, the flag a
/// hit needs, the field, the step (`div`), and the total's limit
/// (`limitTotal`), if any.
type PerStat = (Poe1Quantity, Flags, Field, f64, Option<f64>);

/// The damage increases per stat the hit reads (P6-11bw, P6-11cw).
const DAMAGE_PER_STAT: [PerStat; 4] = [
    // "#% increased Spell Damage per 100 Maximum Mana, up to 40%".
    (
        Poe1Quantity::SpellDamagePer100Mana,
        Flags::SPELL,
        Field::Mana,
        100.0,
        Some(SPELL_DAMAGE_PER_MANA_LIMIT),
    ),
    // "#% increased Attack Damage per 450 Armour" (P6-11cw; `ModFlag.Attack`,
    // `PerStat` `Armour` `div` 450, no limit).
    (
        Poe1Quantity::AttackDamagePer450Armour,
        Flags::ATTACK,
        Field::Armour,
        450.0,
        None,
    ),
    // "#% increased Damage per 100 Strength" (P6-11gm; untagged, `PerStat`
    // `Str` `div` 100, no limit).
    (
        Poe1Quantity::DamagePer100Strength,
        Flags::NONE,
        Field::Strength,
        100.0,
        None,
    ),
    // The agent list: "Deal #% increased Damage Over Time per 100 Player
    // Maximum Life" (`ModFlag.Dot`, `PerStat` `Life` `div` 100): damage over
    // time and ailments, whose contexts carry the flag.
    (
        Poe1Quantity::DotDamagePer100Life,
        Flags::DOT,
        Field::Life,
        100.0,
        None,
    ),
];

/// Each [`DAMAGE_PER_STAT`] term that applies in `cx` ([`per_stat`]).
fn damage_per_stat(
    calc: &mut Calc<'_>,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Vec<Valued>, CalcError> {
    let mut out = Vec::new();
    for spec in DAMAGE_PER_STAT {
        if let Some(v) = per_stat(calc, spec, cx, reasons)? {
            out.push(v);
        }
    }
    // Transfiguration of Body, Mind, and Soul (P6-11hj; `CalcOffence.lua`
    // 511 to 519): `floor(<pool> INC x 0.3)`, the pool's increases with no
    // skill flags, as one more damage increase of attacks, any hit, or
    // spells.
    for (n, pool, need) in [
        (0_u8, Quantity::Life, Flags::ATTACK),
        (1, Quantity::Mana, Flags::NONE),
        (2, Quantity::EnergyShield, Flags::SPELL),
    ] {
        let flag = Quantity::Poe1(Poe1Quantity::Transfiguration(n));
        if !cx.flags.contains(need)
            || !phases::has(calc, flag)
            || flag_on(calc, flag, cx, reasons)?.is_none()
        {
            continue;
        }
        let none = calc.context(Flags::NONE);
        let inc = sum_in(
            calc,
            pool,
            Op::Increased,
            &none,
            Term::Step("transfiguration-increases"),
            reasons,
        )?;
        // Energy shield's increases include intelligence's `floor(Int / 10)`
        // (`CalcPerform.lua` 518), which the engine adds in the defence
        // phase ([`phases::bonus`]), not to the store.
        let inc = if n == 2
            && let Some(int) = phases::bonus(
                calc,
                Field::Intelligence,
                0.1,
                "tenth",
                "energy-shield-from-intelligence",
            )? {
            phases::total(
                calc,
                &[inc, int],
                Term::Step("transfiguration-with-intelligence"),
            )?
        } else {
            inc
        };
        let share = phases::constant(calc, "transfiguration", TRANSFIGURATION)?;
        let scaled = phases::product(calc, &[inc, share], Term::Step("transfiguration-scaled"))?;
        out.push(floor(calc, scaled, Term::Step("transfiguration"))?);
    }
    // Arcane Might (P6-11hk; `CalcOffence.lua` 618 to 631): each `Damage`
    // INC whose flags include `Spell` is also an attack's, at the largest
    // value's percent.
    if cx.flags.contains(Flags::ATTACK)
        && let Some(extra) = spell_damage_to_attacks(calc, cx, reasons)?
    {
        out.push(extra);
    }
    // Attack damage per 75 of the shield's armour and evasion (P6-11hh).
    let per_shield = Quantity::Poe1(Poe1Quantity::DamagePerShieldDefences);
    if phases::has(calc, per_shield) {
        use super::{ArmourSlot, Defence};
        let armour = phases::piece(calc, ArmourSlot::Weapon2, Defence::Armour, reasons)?;
        let evasion = phases::piece(calc, ArmourSlot::Weapon2, Defence::Evasion, reasons)?;
        let stat = phases::total(calc, &[armour, evasion], Term::Step("shield-defences"))?;
        out.push(per_value(
            calc,
            per_shield,
            stat,
            SHIELD_DAMAGE_STEP,
            None,
            cx,
            reasons,
        )?);
    }
    // Projectile speed as bow damage (P6-11hb; `CalcOffence.lua` 645 to
    // 651): each `ProjectileSpeed` INC, as tabulated with no skill flags,
    // is also `Damage` INC with `ModFlag.Bow` and `ModFlag.Hit`.
    let flag = Quantity::Poe1(Poe1Quantity::ProjectileSpeedToBowDamage);
    let speed = Quantity::Poe1(Poe1Quantity::ProjectileSpeed);
    if cx.flags.contains(Flags::BOW | Flags::HIT)
        && phases::has(calc, flag)
        && phases::has(calc, speed)
        && flag_on(calc, flag, cx, reasons)?.is_some()
    {
        let none = calc.context(Flags::NONE);
        out.push(sum_in(
            calc,
            speed,
            Op::Increased,
            &none,
            Term::Step("projectile-speed-as-bow-damage"),
            reasons,
        )?);
    }
    Ok(out)
}

/// The largest spell damage increase per 100 maximum mana: the game's text
/// says "up to 40%" (the stat ID keeps an older 60), and PoB reads the
/// text (`ModParser.lua` 1487: `limit = 40`).
const SPELL_DAMAGE_PER_MANA_LIMIT: f64 = 40.0;

/// A damage increase per some of a field (P6-11bw, P6-11cw; PoB's
/// `PerStat` tag, `Modules/ModStore.lua`: `floor(stat / div + 0.0001) x
/// value`, at most `limitTotal`), for a context with the spec's flag and a
/// store with its quantity; `None` otherwise, so other hits keep their
/// graph. The spell damage per 100 maximum mana is `min(floor(Mana / 100 +
/// 0.0001) x value, 40)`.
///
/// # Errors
/// A graph error.
fn per_stat(
    calc: &mut Calc<'_>,
    (quantity, flag, field, div, limit): PerStat,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    let q = Quantity::Poe1(quantity);
    if !cx.flags.contains(flag) || !phases::has(calc, q) {
        return Ok(None);
    }
    let Some(stat) = calc.field(field).and_then(|f| {
        Some(Valued {
            node: f.node?,
            value: f.value?,
        })
    }) else {
        return Ok(None);
    };
    per_value(calc, q, stat, div, limit, cx, reasons).map(Some)
}

/// `min(floor(stat / div + 0.0001) x value, limit)` of `q`'s `Base` effects
/// in `cx` (PoB's `PerStat` tag) for a computed `stat` ([`per_stat`]'s
/// body; P6-11hd reads it with the accuracy).
///
/// # Errors
/// A graph error.
fn per_value(
    calc: &mut Calc<'_>,
    q: Quantity,
    stat: Valued,
    div: f64,
    limit: Option<f64>,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let per = sum_in(
        calc,
        q,
        Op::Base,
        cx,
        Term::Step("per-stat-effects"),
        reasons,
    )?;
    let step = phases::constant(calc, "per-stat-div", div)?;
    let ratio = quotient(calc, stat, step, Term::Step("per-stat-ratio"))?;
    let epsilon = phases::constant(calc, "per-stat-epsilon", 0.0001)?;
    let ratio = phases::total(calc, &[ratio, epsilon], Term::Step("per-stat-nudged"))?;
    let value = crate::math::floor(ratio.value);
    let node = calc.graph_mut().push(
        Node::Floor {
            term: Term::Step("per-stat-count"),
            of: ratio.node,
        },
        value,
    )?;
    let count = Valued { node, value };
    let total = phases::product(calc, &[count, per], Term::Step("per-stat"))?;
    let Some(limit) = limit else {
        return Ok(total);
    };
    let limit = phases::constant(calc, "per-stat-limit", limit)?;
    phases::pair(calc, total, limit, Term::Step("per-stat-capped"), true)
}

/// `1 + (sum + extras) / 100` of `quantity`'s `Increased` effects in `cx`
/// with computed terms added to the sum (PoB's one `Sum("INC", ...)`).
fn increased_with(
    calc: &mut Calc<'_>,
    quantity: Quantity,
    cx: &QueryContext,
    extras: &[Valued],
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let (store, graph) = calc.parts();
    let effects = store.sum(
        graph,
        quantity,
        Op::Increased,
        cx,
        Term::Step("increased-effects"),
    )?;
    reasons.extend(&effects.reasons);
    let mut terms = vec![Valued {
        node: effects.node,
        value: effects.value,
    }];
    terms.extend_from_slice(extras);
    let sum = phases::total(calc, &terms, Term::Step("increased-sum"))?;
    phases::factor(calc, sum, Term::Increased)
}

/// The sum of `op` effects on `quantity` in `cx` (for other phases).
///
/// # Errors
/// A graph error.
pub(super) fn sum_of(
    calc: &mut Calc<'_>,
    quantity: Quantity,
    op: Op,
    cx: &QueryContext,
    term: Term,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    sum_in(calc, quantity, op, cx, term, reasons)
}

/// The enemy's effect on damage over time of type `t` (`CalcOffence.lua`
/// 5698 to 5711): `(1 - resist / 100) x (1 + taken inc / 100) x taken
/// more`, the resistance without penetration (`calcResistForType`), for
/// physical the enemy's physical damage reduction (nothing modelled gives
/// the enemy any: 0).
///
/// # Errors
/// A graph error.
pub(super) fn dot_enemy_effect(
    calc: &mut Calc<'_>,
    enemy: &Enemy,
    data: &dyn CalcData,
    t: DamageType,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let one = phases::constant(calc, "one", 1.0)?;
    let effect = match resistance_index(t) {
        Some(index) => {
            let resist = enemy_resistance(calc, enemy, data, t, index, cx, reasons)?;
            let share = percent(calc, resist, Term::Step("resistance-share"))?;
            difference(calc, one, share, Term::Step("enemy-effect"))?
        }
        // Physical: `min(max(0, PhysicalDamageReduction), cap)` (4457),
        // 1 when nothing gives the enemy any.
        None => match enemy_physical_reduction(calc, reasons)? {
            Some(pdr) => {
                let zero = phases::constant(calc, "no-negative-reduction", 0.0)?;
                let reduction = phases::pair(calc, zero, pdr, Term::Step("reduction"), false)?;
                let reduction = reduction_capped(calc, data, reduction, reasons)?;
                let share = percent(calc, reduction, Term::Step("reduction-share"))?;
                difference(calc, one, share, Term::Step("enemy-effect"))?
            }
            None => one,
        },
    };
    enemy_reasons(enemy, false, reasons);
    let all = Quantity::Poe1(Poe1Quantity::EnemyDamageTaken);
    let typed = Quantity::Poe1(Poe1Quantity::EnemyTypeDamageTaken(t));
    if !phases::has(calc, all) && !phases::has(calc, typed) {
        return Ok(effect);
    }
    // `effMult = (1 - resist / 100) x (1 + takenInc / 100) x takenMore`, in
    // PoB's order; `takenInc` one sum of the names, `takenMore` their
    // product.
    let [inc, more_all, more_typed] = taken_parts(calc, cx, t, None, reasons)?;
    let taken_more = phases::product(calc, &[more_all, more_typed], Term::More)?;
    phases::product(
        calc,
        &[effect, inc, taken_more],
        Term::Step("effective-multiplier"),
    )
}

/// The enemy's damage taken of type `t` in parts (P3-8m; shared by hits
/// and damage over time): `1 + (inc + typeInc + extra) / 100`, PoB's
/// `takenInc` as one sum (`extra`: a projectile hit's
/// [`projectile_taken`]), then the more of each name.
fn taken_parts(
    calc: &mut Calc<'_>,
    cx: &QueryContext,
    t: DamageType,
    extra: Option<Valued>,
    reasons: &mut Reasons,
) -> Result<[Valued; 3], CalcError> {
    let all = Quantity::Poe1(Poe1Quantity::EnemyDamageTaken);
    let typed = Quantity::Poe1(Poe1Quantity::EnemyTypeDamageTaken(t));
    let inc_all = sum_in(
        calc,
        all,
        Op::Increased,
        cx,
        Term::Step("enemy-damage-taken-increased"),
        reasons,
    )?;
    let inc_typed = sum_in(
        calc,
        typed,
        Op::Increased,
        cx,
        Term::Step("enemy-type-damage-taken-increased"),
        reasons,
    )?;
    let mut terms = vec![inc_all, inc_typed];
    terms.extend(extra);
    let sum = phases::total(calc, &terms, Term::Step("increased-sum"))?;
    let inc = phases::factor(calc, sum, Term::Increased)?;
    let more_all = more(calc, all, cx, reasons)?;
    let more_typed = more(calc, typed, cx, reasons)?;
    Ok([inc, more_all, more_typed])
}

/// The enemy's resistance to `t` a hit meets (`calcResistForType`):
/// `max(min(resist, max), -200)`, `max` the configured resistance when
/// above the base maximum, at most 90; with the hit's penetration taken
/// off: `resist - pen`.
fn effective_resistance(
    calc: &mut Calc<'_>,
    enemy: &Enemy,
    data: &dyn CalcData,
    t: DamageType,
    index: usize,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let resist = enemy_resistance(calc, enemy, data, t, index, cx, reasons)?;
    let pen = sum_in(
        calc,
        Quantity::Penetration(t),
        Op::Base,
        cx,
        Term::Step("penetration"),
        reasons,
    )?;
    // A chance to treat elemental resistances as inverted (P6-11bx;
    // `CalcOffence.lua` 3478 to 3500): `(resist - pen) x (1 - c) + (-resist
    // - pen) x c`, `c` the chance clamped to [0, 1]; only when the store has
    // one, so other hits keep their graph.
    let invert = Quantity::Poe1(Poe1Quantity::InvertElementalResistChance);
    if t != DamageType::Chaos && phases::has(calc, invert) {
        let chance = sum_in(
            calc,
            invert,
            Op::Base,
            cx,
            Term::Step("invert-chance"),
            reasons,
        )?;
        let one = phases::constant(calc, "one", 1.0)?;
        let zero = phases::constant(calc, "zero", 0.0)?;
        let chance = phases::pair(calc, chance, one, Term::Step("invert-capped"), true)?;
        let chance = phases::pair(calc, chance, zero, Term::Step("invert-floored"), false)?;
        if chance.value > 0.0 {
            let kept = difference(calc, resist, pen, Term::Step("resistance-kept"))?;
            let negated = difference(calc, zero, resist, Term::Step("resistance-inverted"))?;
            let inverted = difference(calc, negated, pen, Term::Step("inverted-kept"))?;
            let rest = difference(calc, one, chance, Term::Step("not-inverted-share"))?;
            let a = phases::product(calc, &[kept, rest], Term::Step("resistance-not-inverted"))?;
            let b = phases::product(
                calc,
                &[inverted, chance],
                Term::Step("resistance-inverted-share"),
            )?;
            return phases::total(calc, &[a, b], Term::Step("effective-resistance"));
        }
    }
    difference(calc, resist, pen, Term::Step("effective-resistance"))
}

/// The enemy's resistance to `t` (`calcResistForType`), before
/// penetration: `max(min(resist, max), -200)`.
fn enemy_resistance(
    calc: &mut Calc<'_>,
    enemy: &Enemy,
    data: &dyn CalcData,
    t: DamageType,
    index: usize,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let base = enemy.resistances[index].unwrap_or(0.0);
    let resist = leaf(calc, Source::Enemy, Quantity::Resistance(t), base)?;
    // An override of the resistance where its conditions match (P6-11hc;
    // `calcResistForType`'s `enemyDB:Override(cfg, <Type>Resist)`): it takes
    // the sum's place; the maximum and the floor still apply.
    let on = Quantity::Poe1(Poe1Quantity::EnemyResistOverrideOn(t));
    let overridden = if phases::has(calc, on) && flag_on(calc, on, cx, reasons)?.is_some() {
        Some(sum_in(
            calc,
            Quantity::Poe1(Poe1Quantity::EnemyResistOverride(t)),
            Op::Base,
            cx,
            Term::Step("enemy-resistance-override"),
            reasons,
        )?)
    } else {
        None
    };
    // The enemy's `<Type>Resist` BASE sums its base and the exposure
    // (`calcResistForType`; P3-8k, [`super::exposure`]), only when there
    // is one, so a hit without it keeps its graph.
    let exposure = Quantity::Poe1(Poe1Quantity::EnemyResistance(t));
    let resist = if let Some(o) = overridden {
        o
    } else if phases::has(calc, exposure) {
        let exposed = sum_in(
            calc,
            exposure,
            Op::Base,
            cx,
            Term::Step("enemy-exposure"),
            reasons,
        )?;
        phases::total(calc, &[resist, exposed], Term::Step("enemy-resistance-sum"))?
    } else {
        resist
    };
    let base_max = if let Some(v) = data.monster_constant(ENEMY_MAX_RESIST_STAT) {
        #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
        let v = v as f64;
        v
    } else {
        reasons
            .indeterminate
            .insert(IndeterminateReason::MissingData {
                table: "object_templates",
                key: ENEMY_MAX_RESIST_STAT.to_owned(),
            });
        0.0
    };
    let base_max = leaf(
        calc,
        Source::Enemy,
        Quantity::MaximumResistance(t),
        base_max,
    )?;
    // `m_min(m_max(configInput or EnemyMaxResist, EnemyMaxResist),
    // MaxResistCap)`: only a configured resistance raises it.
    let max = match enemy.resistance_inputs[index] {
        Some(v) => {
            let input = leaf(calc, Source::Enemy, Quantity::Resistance(t), v)?;
            phases::pair(calc, input, base_max, Term::Step("maximum-raised"), false)?
        }
        None => base_max,
    };
    let cap = phases::constant(calc, "maximum-resistance-cap", MAX_RESIST_CAP)?;
    let max = phases::pair(calc, max, cap, Term::Step("enemy-maximum"), true)?;
    let capped = phases::pair(calc, resist, max, Term::Step("enemy-capped"), true)?;
    let floor = phases::constant(calc, "resistance-floor", RESIST_FLOOR)?;
    phases::pair(calc, capped, floor, Term::Step("enemy-resistance"), false)
}

/// The enemy's armour reduction against `raw` physical damage, capped
/// (impale's `impaleResist`, `CalcOffence.lua` 5489 to 5491, Phase 6):
/// `min(max(0, 0 + 0 + armourReductionF(armour, raw)), cap)`, the enemy's
/// physical damage reduction and impale's own 0 (nothing modelled gives
/// them).
///
/// # Errors
/// A graph error.
pub(super) fn impale_armour_reduction(
    calc: &mut Calc<'_>,
    enemy: &Enemy,
    data: &dyn CalcData,
    raw: Valued,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    armour_reduction(calc, enemy, data, raw, false, reasons)
}

/// The enemy's damage taken from hits of type `t` (impale's `impaleTaken`,
/// `CalcOffence.lua` 5495 to 5497, Phase 6): `(1 + inc / 100) x more` in a
/// hit's context, `None` when nothing modelled changes it.
///
/// # Errors
/// A graph error.
pub(super) fn hit_damage_taken(
    calc: &mut Calc<'_>,
    t: DamageType,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    let all = Quantity::Poe1(Poe1Quantity::EnemyDamageTaken);
    let typed = Quantity::Poe1(Poe1Quantity::EnemyTypeDamageTaken(t));
    if !phases::has(calc, all) && !phases::has(calc, typed) {
        return Ok(None);
    }
    let cx = calc.context(Flags::HIT);
    let [inc, more_all, more_typed] = taken_parts(calc, &cx, t, None, reasons)?;
    let more = phases::product(calc, &[more_all, more_typed], Term::More)?;
    Ok(Some(phases::product(
        calc,
        &[inc, more],
        Term::Step("impale-damage-taken"),
    )?))
}

/// The enemy's physical damage reduction (PoB's `PhysicalDamageReduction`
/// BASE on the enemy, read with no skill config; Poacher's Mark, P6-9e),
/// `None` when nothing gives it any.
fn enemy_physical_reduction(
    calc: &mut Calc<'_>,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    let q = Quantity::Poe1(Poe1Quantity::EnemyPhysicalDamageReduction);
    if !phases::has(calc, q) {
        return Ok(None);
    }
    let cx = calc.context(Flags::NONE);
    Ok(Some(sum_in(
        calc,
        q,
        Op::Base,
        &cx,
        Term::Step("enemy-physical-damage-reduction"),
        reasons,
    )?))
}

/// `min(reduction, cap)`, the cap the monster template's
/// (`data.misc.EnemyPhysicalDamageReductionCap`).
fn reduction_capped(
    calc: &mut Calc<'_>,
    data: &dyn CalcData,
    reduction: Valued,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let cap = if let Some(v) = data.monster_constant(ENEMY_MAX_REDUCTION_STAT) {
        #[expect(clippy::cast_precision_loss, reason = "a small integer, exact as f64")]
        let v = v as f64;
        v
    } else {
        reasons
            .indeterminate
            .insert(IndeterminateReason::MissingData {
                table: "object_templates",
                key: ENEMY_MAX_REDUCTION_STAT.to_owned(),
            });
        0.0
    };
    let cap = leaf(
        calc,
        Source::Enemy,
        Quantity::Poe1(Poe1Quantity::MaximumPhysicalDamageReduction),
        cap,
    )?;
    phases::pair(calc, reduction, cap, Term::Step("reduction-capped"), true)
}

/// The share of a physical hit the enemy's armour lets through (module
/// docs, step 3): `1 - min(max(0, pdr + armourReductionF(armour,
/// average)), cap) / 100`, `pdr` the enemy's physical damage reduction
/// ([`enemy_physical_reduction`]); for a hit (`hit`), plus the
/// character's change to it ([`Poe1Quantity::HitEnemyPhysicalDamageReduction`],
/// PoB's `skillModList` `EnemyPhysicalDamageReduction`, `CalcOffence.lua`
/// 3434; impale's reads it not, 5491).
fn armour_reduction(
    calc: &mut Calc<'_>,
    enemy: &Enemy,
    data: &dyn CalcData,
    average: Valued,
    hit: bool,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    // `enemyArmour = m_max(calcLib.val(enemyDB, "Armour"), 0)`.
    let armour = leaf(
        calc,
        Source::Enemy,
        Quantity::Armour,
        enemy.armour.unwrap_or(0.0),
    )?;
    let zero = phases::constant(calc, "no-negative-armour", 0.0)?;
    let armour = phases::pair(calc, armour, zero, Term::Step("enemy-armour"), false)?;
    // `calcs.armourReductionF`: 0 when both are 0, else `armour / (armour
    // + raw x 5) x 100`.
    let reduction = if armour.value == 0.0 && average.value == 0.0 {
        phases::constant(calc, "no-armour-reduction", 0.0)?
    } else {
        let five = phases::constant(calc, "armour-hit-scale", 5.0)?;
        let scaled = phases::product(calc, &[average, five], Term::Step("hit-scaled"))?;
        let denominator = phases::total(calc, &[armour, scaled], Term::Step("armour-denominator"))?;
        let share = quotient(calc, armour, denominator, Term::Step("armour-share"))?;
        let hundred = phases::constant(calc, "hundred", 100.0)?;
        phases::product(calc, &[share, hundred], Term::Step("armour-reduction"))?
    };
    let reduction = match enemy_physical_reduction(calc, reasons)? {
        Some(pdr) => phases::total(calc, &[pdr, reduction], Term::Step("with-enemy-reduction"))?,
        None => reduction,
    };
    let penalty = Quantity::Poe1(Poe1Quantity::HitEnemyPhysicalDamageReduction);
    let reduction = if hit && phases::has(calc, penalty) {
        let cx = calc.context(Flags::NONE);
        let change = sum_in(
            calc,
            penalty,
            Op::Base,
            &cx,
            Term::Step("hit-enemy-physical-damage-reduction"),
            reasons,
        )?;
        phases::total(calc, &[change, reduction], Term::Step("with-hit-reduction"))?
    } else {
        reduction
    };
    let zero = phases::constant(calc, "no-negative-reduction", 0.0)?;
    let reduction = phases::pair(calc, zero, reduction, Term::Step("reduction"), false)?;
    reduction_capped(calc, data, reduction, reasons)
}

/// `1 - armour_reduction / 100` (module docs, step 3).
fn armour_effect(
    calc: &mut Calc<'_>,
    enemy: &Enemy,
    data: &dyn CalcData,
    average: Valued,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let reduction = armour_reduction(calc, enemy, data, average, true, reasons)?;
    let reduction = ignore_reduction(calc, reduction, reasons)?;
    let share = percent(calc, reduction, Term::Step("reduction-share"))?;
    let one = phases::constant(calc, "one", 1.0)?;
    difference(calc, one, share, Term::Step("enemy-effect"))
}

/// The config's Ruthless mode (`ConfigOptions.lua` 954: `AVERAGE`, the
/// default, or `MAX`; P6-11bt).
pub const RUTHLESS_MODE_INPUT: &str = "ruthlessSupportMode";

/// Ruthless's effect on the hit or its bleed and poison (P6-11bt;
/// `CalcOffence.lua` 2889 to 2909): with a count above 0, the chance
/// `round(100 / count)` in the average mode, and the effect `1 - chance /
/// 100 + chance / 100 x (1 + multiplier / 100)`; `None` without a count,
/// so a hit without Ruthless keeps its graph. The `MAX` mode (chance 100
/// and the DPS divided by the count) is a reason.
///
/// # Errors
/// A graph error.
pub(super) fn ruthless_effect(
    calc: &mut Calc<'_>,
    cx: &QueryContext,
    value: super::RuthlessValue,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    let count_q = Quantity::Poe1(Poe1Quantity::Ruthless(super::RuthlessValue::MaxCount));
    if !phases::has(calc, count_q) {
        return Ok(None);
    }
    let count = sum_in(
        calc,
        count_q,
        Op::Base,
        cx,
        Term::Step("ruthless-count"),
        reasons,
    )?;
    if count.value <= 0.0 {
        return Ok(None);
    }
    let set = super::collect::active_config(calc.input().build);
    match set.and_then(|s| {
        super::config::last_input(s, RUTHLESS_MODE_INPUT, loom_domain::ConfigInputKind::Input)
    }) {
        None => {}
        Some(loom_domain::ConfigValue::String(m)) if m == "AVERAGE" => {}
        Some(_) => {
            reasons.partial.insert(UnsupportedReason::UnmodelledConfig {
                input: RUTHLESS_MODE_INPUT.to_owned(),
            });
        }
    }
    let hundred = phases::constant(calc, "hundred", 100.0)?;
    let share = quotient(calc, hundred, count, Term::Step("ruthless-share"))?;
    let chance = phases::round(calc, share, Term::Step("ruthless-chance"))?;
    let multiplier = sum_in(
        calc,
        Quantity::Poe1(Poe1Quantity::Ruthless(value)),
        Op::Base,
        cx,
        Term::Step("ruthless-multiplier"),
        reasons,
    )?;
    let multiplier = phases::factor(calc, multiplier, Term::Step("ruthless-more"))?;
    let chance = percent(calc, chance, Term::Step("ruthless-chance-share"))?;
    let one = phases::constant(calc, "one", 1.0)?;
    let rest = difference(calc, one, chance, Term::Step("ruthless-rest"))?;
    let blow = phases::product(calc, &[chance, multiplier], Term::Step("ruthless-blow"))?;
    Ok(Some(phases::total(
        calc,
        &[rest, blow],
        Term::Step("ruthless-effect"),
    )?))
}

/// The config list of how a chance to ignore the enemy's physical damage
/// reduction counts (`ConfigOptions.lua` 955: `MIN`, `AVERAGE`, `MAX`;
/// `AVERAGE` by default).
pub const PDR_MODE_INPUT: &str = "ChanceToIgnoreEnemyPhysicalDamageReductionMode";

/// A hit's chance to ignore the enemy's physical damage reduction
/// (`CalcOffence.lua` 3423 to 3435; P6-11j): the chance, at most 100; with
/// a chance between 0 and 100 the config's `MAX` counts it 100 and `MIN`
/// 0; at 100 the reduction is 0, else a positive reduction times `1 -
/// chance / 100`. Not modelled, so reasons where they appear: the partial
/// ignore and the flag that ignores it, unmapped stats.
fn ignore_reduction(
    calc: &mut Calc<'_>,
    reduction: Valued,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let q = Quantity::Poe1(Poe1Quantity::IgnorePdrChance);
    if !phases::has(calc, q) {
        return Ok(reduction);
    }
    let cx = calc.context(Flags::NONE);
    let chance = sum_in(
        calc,
        q,
        Op::Base,
        &cx,
        Term::Step("ignore-reduction-chance"),
        reasons,
    )?;
    let hundred = phases::constant(calc, "hundred", 100.0)?;
    let chance = phases::pair(calc, chance, hundred, Term::Step("chance-capped"), true)?;
    let chance = if chance.value > 0.0 && chance.value < 100.0 {
        let set = super::collect::active_config(calc.input().build);
        match set.and_then(|s| {
            super::config::last_input(s, PDR_MODE_INPUT, loom_domain::ConfigInputKind::Input)
        }) {
            Some(loom_domain::ConfigValue::String(m)) if m == "MAX" => {
                phases::constant(calc, "ignore-reduction-mode-max", 100.0)?
            }
            Some(loom_domain::ConfigValue::String(m)) if m == "MIN" => {
                phases::constant(calc, "ignore-reduction-mode-min", 0.0)?
            }
            None => chance,
            Some(loom_domain::ConfigValue::String(m)) if m == "AVERAGE" => chance,
            Some(_) => {
                reasons.partial.insert(UnsupportedReason::UnmodelledConfig {
                    input: PDR_MODE_INPUT.to_owned(),
                });
                chance
            }
        }
    } else {
        chance
    };
    if chance.value >= 100.0 {
        return phases::constant(calc, "reduction-ignored", 0.0);
    }
    if reduction.value <= 0.0 {
        return Ok(reduction);
    }
    let share = percent(calc, chance, Term::Step("ignore-share"))?;
    let one = phases::constant(calc, "one", 1.0)?;
    let kept = difference(calc, one, share, Term::Step("reduction-kept"))?;
    phases::product(
        calc,
        &[reduction, kept],
        Term::Step("reduction-after-ignore"),
    )
}

/// The enemy's damage taken, `(1 + inc / 100) x more` of its
/// [`Poe1Quantity::EnemyDamageTaken`] (PoB's `DamageTaken` INC and MORE on
/// the enemy, `CalcOffence.lua` 3521, 3522; [`super::damage_taken`]);
/// `None` when nothing changes it, so a hit without it keeps its graph.
fn enemy_damage_taken(
    calc: &mut Calc<'_>,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    let quantity = Quantity::Poe1(Poe1Quantity::EnemyDamageTaken);
    let projectile = projectile_taken(calc, cx, reasons)?;
    if projectile.is_none()
        && !calc
            .store()
            .effects()
            .iter()
            .any(|e| e.quantity == quantity)
    {
        return Ok(None);
    }
    let (inc, more) = match projectile {
        None => modifiers(calc, quantity, cx, reasons)?,
        Some(p) => {
            let inc = sum_in(
                calc,
                quantity,
                Op::Increased,
                cx,
                Term::Step("enemy-damage-taken-increased"),
                reasons,
            )?;
            let sum = phases::total(calc, &[inc, p], Term::Step("increased-sum"))?;
            let inc = phases::factor(calc, sum, Term::Increased)?;
            (inc, more(calc, quantity, cx, reasons)?)
        }
    };
    Ok(Some(phases::product(
        calc,
        &[inc, more],
        Term::Step("enemy-damage-taken"),
    )?))
}

/// The enemy's damage taken by a hit of type `t` when it has some of that
/// type (P3-8m): `(1 + (inc + typeInc) / 100) x more x typeMore`, PoB's
/// `takenInc = Sum("INC", cfg, "DamageTaken", "<Type>DamageTaken")` and
/// `takenMore = More(cfg, "DamageTaken", "<Type>DamageTaken")`
/// (`CalcOffence.lua` 3403, 3404), each name's more rounded on its own.
fn type_damage_taken(
    calc: &mut Calc<'_>,
    cx: &QueryContext,
    t: DamageType,
    reasons: &mut Reasons,
) -> Result<Valued, CalcError> {
    let projectile = projectile_taken(calc, cx, reasons)?;
    let parts = taken_parts(calc, cx, t, projectile, reasons)?;
    phases::product(calc, &parts, Term::Step("enemy-damage-taken"))
}

/// What a hit's kind adds to the enemy's `takenInc`, each read with no
/// skill config and never by damage over time (`CalcOffence.lua` 3482 to
/// 3490): a projectile hit's `ProjectileDamageTaken` (Sniper's Mark,
/// P6-9e), a projectile attack's `ProjectileAttackDamageTaken` (Ensnaring
/// Arrow, P6-11fn), and a trap or mine skill's `TrapMineDamageTaken` (Bear
/// Trap, P6-11fj), summed; `None` when the hit is none or nothing gives it.
fn projectile_taken(
    calc: &mut Calc<'_>,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    let kinds = [
        (
            Poe1Quantity::EnemyProjectileDamageTaken,
            cx.flags.contains(Flags::PROJECTILE),
            "enemy-projectile-damage-taken",
        ),
        (
            Poe1Quantity::EnemyProjectileAttackDamageTaken,
            cx.flags.contains(Flags::PROJECTILE) && cx.flags.contains(Flags::ATTACK),
            "enemy-projectile-attack-damage-taken",
        ),
        (
            Poe1Quantity::EnemyTrapMineDamageTaken,
            cx.flags.contains(Flags::KEYWORD_TRAP) || cx.flags.contains(Flags::KEYWORD_MINE),
            "enemy-trap-mine-damage-taken",
        ),
    ];
    let none = calc.context(Flags::NONE);
    let mut terms = Vec::new();
    for (q, applies, step) in kinds {
        let q = Quantity::Poe1(q);
        if applies && phases::has(calc, q) {
            terms.push(sum_in(
                calc,
                q,
                Op::Increased,
                &none,
                Term::Step(step),
                reasons,
            )?);
        }
    }
    match terms.as_slice() {
        [] => Ok(None),
        [one] => Ok(Some(*one)),
        _ => Ok(Some(phases::total(
            calc,
            &terms,
            Term::Step("enemy-kind-damage-taken"),
        )?)),
    }
}

/// The enemy's resistance index ([`Enemy::resistances`]) of a damage type
/// with a resistance; `None` for physical.
const fn resistance_index(t: DamageType) -> Option<usize> {
    match t {
        DamageType::Fire => Some(0),
        DamageType::Cold => Some(1),
        DamageType::Lightning => Some(2),
        DamageType::Chaos => Some(3),
        DamageType::Physical => None,
    }
}

/// The enemy problems a hit reads: configured inputs of another type and
/// an unknown boss kind (as config reasons). Missing table rows (monster
/// levels, the armour constant, boss types) feed only the enemy's armour
/// and evasion, which an elemental spell never reads; they count only
/// when `defences` (a physical hit, or an attack's chance to hit) does.
fn enemy_reasons(enemy: &Enemy, defences: bool, reasons: &mut Reasons) {
    for p in &enemy.problems {
        match p {
            EnemyProblem::InputType { input } => {
                reasons.partial.insert(UnsupportedReason::UnmodelledConfig {
                    input: (*input).to_owned(),
                });
            }
            EnemyProblem::UnknownBossKind { .. } => {
                reasons.partial.insert(UnsupportedReason::UnmodelledConfig {
                    input: "enemyIsBoss".to_owned(),
                });
            }
            EnemyProblem::MissingData { .. } if !defences => {}
            EnemyProblem::MissingData { table, key } => {
                reasons
                    .indeterminate
                    .insert(IndeterminateReason::MissingData {
                        table,
                        key: key.clone(),
                    });
            }
        }
    }
}

/// Where a hit's base damage comes from.
#[derive(Clone, Copy)]
enum DamageSource<'a> {
    /// The skill's own (a spell's `SkillBaseDamage`).
    Skill,
    /// The weapon's (its values per type, [`WeaponData::damage`]).
    Weapon(&'a [Option<[Valued; 2]>; 5]),
}

/// What the damage passes need of the skill.
struct HitShape<'a> {
    /// The active gem.
    source: Source,
    /// Its level row (effectiveness, base multiplier).
    level: &'a StatSetLevel,
    /// The hit's flags.
    flags: Flags,
    /// The base damage's source.
    damage: DamageSource<'a>,
    /// Strength's increase of melee physical damage, percent (attacks).
    melee_physical: Option<Valued>,
    /// Whether the skill's crit chance is not 0: PoB then sets its
    /// `CritInPast8Sec` condition before the hit's damage
    /// (`CalcOffence.lua` 3184; [`keystones::CRIT_IN_PAST_8_SEC`], P3-8m).
    crit_recently: bool,
    /// Intimidating Cry's exerted attacks' uptime, percent, which PoB adds
    /// to the double damage chance (close-out; [`exerted_attacks`]).
    exert_uptime: Option<Valued>,
}

/// The skill's conversion table (module docs, step 2), when it converts:
/// `share[from][to]` the share of `from` converted to `to`, `kept[from]`
/// the share `from` keeps (`conversionTable[type].mult`).
pub(super) struct Conversion {
    share: [[Option<Valued>; 5]; 5],
    pub(super) kept: [Option<Valued>; 5],
}

/// What a hit's damage was computed from (Phase 6, for its ailments,
/// which PoB recomputes in their own config, `calcAilmentSourceDamage`):
/// each type's base damage and the conversion table.
pub(super) struct HitSources {
    /// Each type's `[min, max]` base, in [`TYPES`] order.
    pub(super) bases: [Option<[Valued; 2]>; 5],
    /// The conversion table, when the hit converts.
    pub(super) conversion: Option<Conversion>,
    /// The physical part's average before the enemy (PoB's
    /// `damageTypeHitAvg`), from non-crits and from crits, for impale's
    /// stored hit (Phase 6).
    pub(super) physical: [Option<Valued>; 2],
    /// Each type's average against the enemy (PoB's `damageTypeHitAvg`
    /// after `effMult`), from non-crits and from crits, for leech (P6-10b
    /// part 9).
    pub(super) against: [Vec<(DamageType, Valued)>; 2],
}

/// PoB's `buildConversionTable` (`CalcOffence.lua` 1878 to 1921) over
/// the skill's own conversion and, since P3-8n, the other conversion of a
/// type to a later one (`<From>DamageConvertTo<To>`; the elemental and
/// non-chaos sums of its other names are not mapped): `None` when the
/// store has neither, so a hit without conversion has no conversion nodes.
/// A type with only the skill's own conversion keeps P3-10's nodes.
fn conversion(
    calc: &mut Calc<'_>,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Option<Conversion>, CalcError> {
    let skill_q = |from: usize, to: usize| {
        Quantity::Poe1(Poe1Quantity::SkillConversion(TYPES[from], TYPES[to]))
    };
    let global_q =
        |from: usize, to: usize| Quantity::Poe1(Poe1Quantity::Conversion(TYPES[from], TYPES[to]));
    let gain_q =
        |from: usize, to: usize| Quantity::Poe1(Poe1Quantity::GainAs(TYPES[from], TYPES[to]));
    let any = (0..4).any(|from| {
        (from + 1..5).any(|to| {
            phases::has(calc, skill_q(from, to))
                || phases::has(calc, global_q(from, to))
                || phases::has(calc, gain_q(from, to))
        })
    });
    if !any {
        return Ok(None);
    }
    let mut table = Conversion {
        share: [[None; 5]; 5],
        kept: [None; 5],
    };
    // `m_max(skillModList:Sum("BASE", cfg, name), 0)`.
    let floored = |calc: &mut Calc<'_>,
                   q: Quantity,
                   step: &'static str,
                   reasons: &mut Reasons|
     -> Result<Valued, CalcError> {
        let sum = sum_in(calc, q, Op::Base, cx, Term::Step(step), reasons)?;
        let zero = phases::constant(calc, "no-negative-conversion", 0.0)?;
        phases::pair(calc, sum, zero, Term::Step("conversion"), false)
    };
    // Chaos converts to nothing (`conversionTable["Chaos"] = { mult = 1 }`).
    for from in 0..4 {
        let mut skill = Vec::new();
        let mut global = Vec::new();
        // Gain as extra (close-out): `add[to] = m_max(Sum(BASE, cfg,
        // <From>DamageGainAs<To>), 0)`; `dmgTable[to] = (globalConv +
        // skillConv + add) / 100`, the kept share unchanged (PoB v2.67.2,
        // `CalcOffence.lua` 1976, 2002).
        let mut gain = Vec::new();
        for to in from + 1..5 {
            if phases::has(calc, gain_q(from, to)) {
                gain.push((to, floored(calc, gain_q(from, to), "gain-as", reasons)?));
            }
        }
        for to in from + 1..5 {
            if phases::has(calc, skill_q(from, to)) {
                skill.push((
                    to,
                    floored(calc, skill_q(from, to), "skill-conversion", reasons)?,
                ));
            }
            if phases::has(calc, global_q(from, to)) {
                global.push((
                    to,
                    floored(calc, global_q(from, to), "global-conversion", reasons)?,
                ));
            }
        }
        if skill.is_empty() && global.is_empty() {
            // Gain alone: its share, nothing converted.
            let hundred = phases::constant(calc, "hundred", 100.0)?;
            for (to, value) in gain {
                table.share[from][to] = Some(quotient(
                    calc,
                    value,
                    hundred,
                    Term::Step("conversion-share"),
                )?);
            }
            continue;
        }
        let parts: Vec<Valued> = skill.iter().map(|(_, v)| *v).collect();
        let skill_total = phases::total(calc, &parts, Term::Step("conversion-total"))?;
        let hundred = phases::constant(calc, "hundred", 100.0)?;
        if global.is_empty() {
            // P3-10's nodes. Above 100%: `skillConv[type] = val x 100 /
            // skillTotal`.
            let factor = if skill_total.value > 100.0 {
                Some(quotient(
                    calc,
                    hundred,
                    skill_total,
                    Term::Step("conversion-scale"),
                )?)
            } else {
                None
            };
            let mut converted_to = [None; 5];
            for (to, value) in skill {
                let value = match factor {
                    Some(f) => phases::product(calc, &[value, f], Term::Step("conversion-scaled"))?,
                    None => value,
                };
                converted_to[to] = Some(value);
            }
            for (to, converted) in converted_to.iter().enumerate().skip(from + 1) {
                let g = gain.iter().find(|(t, _)| *t == to).map(|(_, v)| *v);
                let value = match (*converted, g) {
                    (None, None) => continue,
                    (Some(k), None) => k,
                    (None, Some(g)) => g,
                    (Some(k), Some(g)) => {
                        phases::total(calc, &[k, g], Term::Step("conversion-and-gain"))?
                    }
                };
                table.share[from][to] = Some(quotient(
                    calc,
                    value,
                    hundred,
                    Term::Step("conversion-share"),
                )?);
            }
            // `mult = 1 - m_min((globalTotal + skillTotal) / 100, 1)`.
            let one = phases::constant(calc, "one", 1.0)?;
            let converted = quotient(calc, skill_total, hundred, Term::Step("converted-share"))?;
            let converted =
                phases::pair(calc, converted, one, Term::Step("converted-capped"), true)?;
            table.kept[from] = Some(difference(calc, one, converted, Term::Step("kept-share"))?);
            continue;
        }
        let parts: Vec<Valued> = global.iter().map(|(_, v)| *v).collect();
        let mut global_total = phases::total(calc, &parts, Term::Step("global-conversion-total"))?;
        let zero = phases::constant(calc, "no-conversion", 0.0)?;
        if skill_total.value > 100.0 {
            // The skill's own scaled to 100, the others removed.
            let factor = quotient(calc, hundred, skill_total, Term::Step("conversion-scale"))?;
            for (_, value) in &mut skill {
                *value = phases::product(calc, &[*value, factor], Term::Step("conversion-scaled"))?;
            }
            for (_, value) in &mut global {
                *value = zero;
            }
        } else if global_total.value + skill_total.value > 100.0 {
            // `factor = (100 - skillTotal) / globalTotal`.
            let left = difference(calc, hundred, skill_total, Term::Step("conversion-left"))?;
            let factor = quotient(
                calc,
                left,
                global_total,
                Term::Step("global-conversion-scale"),
            )?;
            for (_, value) in &mut global {
                *value = phases::product(
                    calc,
                    &[*value, factor],
                    Term::Step("global-conversion-scaled"),
                )?;
            }
            global_total = phases::product(
                calc,
                &[global_total, factor],
                Term::Step("global-conversion-total-scaled"),
            )?;
        }
        // `conversion[type] = (globalConv[type] + skillConv[type]) / 100`.
        for to in from + 1..5 {
            let g = global.iter().find(|(t, _)| *t == to).map(|(_, v)| *v);
            let k = skill.iter().find(|(t, _)| *t == to).map(|(_, v)| *v);
            let a = gain.iter().find(|(t, _)| *t == to).map(|(_, v)| *v);
            let value = match (g, k, a) {
                (None, None, None) => continue,
                (None, None, Some(a)) => a,
                (Some(g), None, None) => {
                    phases::total(calc, &[g, zero], Term::Step("conversion-sum"))?
                }
                (None, Some(k), None) => {
                    phases::total(calc, &[zero, k], Term::Step("conversion-sum"))?
                }
                (Some(g), Some(k), None) => {
                    phases::total(calc, &[g, k], Term::Step("conversion-sum"))?
                }
                (g, k, Some(a)) => {
                    let g = g.unwrap_or(zero);
                    let k = k.unwrap_or(zero);
                    phases::total(calc, &[g, k, a], Term::Step("conversion-sum"))?
                }
            };
            table.share[from][to] = Some(quotient(
                calc,
                value,
                hundred,
                Term::Step("conversion-share"),
            )?);
        }
        let one = phases::constant(calc, "one", 1.0)?;
        let both = phases::total(
            calc,
            &[global_total, skill_total],
            Term::Step("conversion-both"),
        )?;
        let converted = quotient(calc, both, hundred, Term::Step("converted-share"))?;
        let converted = phases::pair(calc, converted, one, Term::Step("converted-capped"), true)?;
        table.kept[from] = Some(difference(calc, one, converted, Term::Step("kept-share"))?);
    }
    Ok(Some(table))
}

/// PoB's `calcDamage` (module docs, step 2): the damage of type `TYPES[t]`
/// with the modifiers of its own type and of `extra` (the types it is
/// converted to), plus what earlier types convert to it.
#[expect(
    clippy::too_many_arguments,
    reason = "the recursion carries PoB's arguments"
)]
pub(super) fn calc_damage(
    calc: &mut Calc<'_>,
    t: usize,
    extra: Flags,
    bases: &[Option<[Valued; 2]>; 5],
    conversion: Option<&Conversion>,
    cx: &QueryContext,
    melee_physical: Option<Valued>,
    reasons: &mut Reasons,
    zero: &mut Option<Valued>,
) -> Result<[Valued; 2], CalcError> {
    let ty = TYPES[t];
    let mut converted: Option<[Valued; 2]> = None;
    if let Some(table) = conversion {
        let mut terms: [Vec<Valued>; 2] = [Vec::new(), Vec::new()];
        for from in 0..t {
            let Some(share) = table.share[from][t] else {
                continue;
            };
            if share.value <= 0.0 {
                continue;
            }
            let damage = calc_damage(
                calc,
                from,
                extra | Flags::damage(ty),
                bases,
                conversion,
                cx,
                melee_physical,
                reasons,
                zero,
            )?;
            for (bound, terms) in terms.iter_mut().enumerate() {
                terms.push(phases::product(
                    calc,
                    &[damage[bound], share],
                    Term::Step("converted"),
                )?);
            }
        }
        if !terms[0].is_empty() {
            let min = phases::total(calc, &terms[0], Term::Step("converted-in"))?;
            let max = phases::total(calc, &terms[1], Term::Step("converted-in"))?;
            converted = Some(if min.value != 0.0 && max.value != 0.0 {
                [
                    phases::round(calc, min, Term::Step("converted-rounded"))?,
                    phases::round(calc, max, Term::Step("converted-rounded"))?,
                ]
            } else {
                [min, max]
            });
        }
    }
    let base = match bases[t] {
        Some(base) if base[0].value != 0.0 || base[1].value != 0.0 => base,
        _ => {
            return match converted {
                Some(c) => Ok(c),
                None => {
                    let z = zero_node(calc, zero)?;
                    Ok([z, z])
                }
            };
        }
    };
    let typed = QueryContext {
        flags: cx.flags | extra | Flags::damage(ty),
        states: cx.states.clone(),
    };
    let bonus = melee_physical.filter(|_| typed.flags.contains(Flags::MELEE | Flags::PHYSICAL));
    // Damage per stat (P6-11bw, P6-11cw), terms of the same sum.
    let per_stat = damage_per_stat(calc, &typed, reasons)?;
    let (inc, more) = match bonus {
        None if per_stat.is_empty() => modifiers(calc, Quantity::Damage, &typed, reasons)?,
        None => {
            let inc = increased_with(calc, Quantity::Damage, &typed, &per_stat, reasons)?;
            let more = more(calc, Quantity::Damage, &typed, reasons)?;
            (inc, more)
        }
        Some(bonus) => {
            // `1 + (Sum("INC", ...) + strDmgBonus) / 100`: one sum.
            let (store, graph) = calc.parts();
            let effects = store.sum(
                graph,
                Quantity::Damage,
                Op::Increased,
                &typed,
                Term::Step("increased-effects"),
            )?;
            reasons.extend(&effects.reasons);
            let effects = Valued {
                node: effects.node,
                value: effects.value,
            };
            let mut terms = vec![effects, bonus];
            terms.extend(per_stat);
            let sum = phases::total(calc, &terms, Term::Step("increased-sum"))?;
            let inc = phases::factor(calc, sum, Term::Increased)?;
            let more = more(calc, Quantity::Damage, &typed, reasons)?;
            (inc, more)
        }
    };
    // The modifiers of one end of the range (P6-11bo), read only when the
    // store has any, so a hit without them keeps its graph.
    let bounded = calc.store().effects().iter().any(|e| {
        matches!(
            e.quantity,
            Quantity::Poe1(Poe1Quantity::BoundDamage(None, _))
        ) || e.quantity == Quantity::Poe1(Poe1Quantity::BoundDamage(Some(ty), Bound::Min))
            || e.quantity == Quantity::Poe1(Poe1Quantity::BoundDamage(Some(ty), Bound::Max))
    });
    let mut rolled = Vec::with_capacity(2);
    for (bound, b) in base.into_iter().enumerate() {
        let mut raw = phases::product(calc, &[b, inc, more], Term::Step("unrounded"))?;
        let end = Bound::ALL[bound];
        // `(base x inc x more) x genericMore<Bound>Damage + converted`,
        // then `x more<Bound><Type>Damage x inc<Bound><Type>Damage`
        // (`CalcOffence.lua` 136 to 137).
        if bounded {
            let generic = self::more(
                calc,
                Quantity::Poe1(Poe1Quantity::BoundDamage(None, end)),
                &typed,
                reasons,
            )?;
            raw = phases::product(calc, &[raw, generic], Term::Step("bound-more"))?;
        }
        if let Some(c) = converted {
            raw = phases::total(calc, &[raw, c[bound]], Term::Step("with-converted"))?;
        }
        if bounded {
            let (own_inc, own_more) = modifiers(
                calc,
                Quantity::Poe1(Poe1Quantity::BoundDamage(Some(ty), end)),
                &typed,
                reasons,
            )?;
            raw = phases::product(calc, &[raw, own_more, own_inc], Term::Step("bound-type"))?;
        }
        rolled.push(phases::round(calc, raw, Term::Step("damage"))?);
    }
    Ok([rolled[0], rolled[1]])
}

/// The constant 0 of a damage with none, made once and only when needed
/// (a hit without conversion has no such node).
fn zero_node(calc: &mut Calc<'_>, cache: &mut Option<Valued>) -> Result<Valued, CalcError> {
    if let Some(z) = *cache {
        return Ok(z);
    }
    let z = phases::constant(calc, "no-damage", 0.0)?;
    *cache = Some(z);
    Ok(z)
}

/// The non-crit and crit hit, each summed over the damage types (module
/// docs, steps 1 to 3).
fn passes(
    calc: &mut Calc<'_>,
    shape: &HitShape<'_>,
    enemy: &Enemy,
    data: &dyn CalcData,
    multiplier: Valued,
    reasons: &mut Reasons,
) -> Result<(Valued, Valued, HitSources), CalcError> {
    let cx = calc.context(shape.flags).with_state(
        ConditionKey::new(keystones::CRIT_IN_PAST_8_SEC),
        shape.crit_recently,
    );
    // Damage effectiveness and base multiplier: `stored / 10000 + 1` as
    // PoB's exporter writes them (absent: 1), one division of whole
    // numbers here, which is the nearest double to the same decimal.
    let ratio = |stored: i64| {
        #[expect(
            clippy::cast_precision_loss,
            reason = "stored values are small integers, exact as f64"
        )]
        let v = (10_000 + stored) as f64 / 10_000.0;
        v
    };
    // A minion's skill: PoB replaces its damage effectiveness with `1 +
    // minionDamageEffectiveness / 100` of the summoning skill
    // (`CalcActiveSkill.lua` 910), a value no skill of PoB's data sets, so
    // 1 (P6-7c3).
    let stored = if calc.actor() == crate::engine::Actor::Minion {
        0
    } else {
        shape.level.damage_effectiveness.unwrap_or(0)
    };
    let effectiveness = leaf(
        calc,
        shape.source.clone(),
        Quantity::Poe1(Poe1Quantity::DamageEffectiveness),
        ratio(stored),
    )?;
    let base_multiplier = leaf(
        calc,
        shape.source.clone(),
        Quantity::Poe1(Poe1Quantity::BaseDamageMultiplier),
        ratio(shape.level.base_multiplier),
    )?;
    let conversion = conversion(calc, &cx, reasons)?;
    // The hand's weapon's own double damage (the agent list, 2026-11-27):
    // an attack's pass alone.
    let weapon_double = shape.flags.contains(Flags::ATTACK).then(|| {
        if shape.flags.contains(Flags::OFF_HAND_ATTACK) {
            Hand::Off.weapon(WeaponValue::DoubleDamage)
        } else {
            Hand::Main.weapon(WeaponValue::DoubleDamage)
        }
    });
    let scaled = double_damage(calc, &cx, shape.exert_uptime, weapon_double, reasons)?;
    // Ruthless's hit effect joins `allMult` after the scaled damage effect
    // (`CalcOffence.lua` 3352; P6-11bt).
    let scaled = match (
        scaled,
        ruthless_effect(calc, &cx, super::RuthlessValue::Hit, reasons)?,
    ) {
        (s, None) => s,
        (None, Some(r)) => Some(r),
        (Some(s), Some(r)) => Some(phases::product(
            calc,
            &[s, r],
            Term::Step("ruthless-scaled"),
        )?),
    };
    let taken = enemy_damage_taken(calc, &cx, reasons)?;
    let mut bases: [Option<[Valued; 2]>; 5] = [None; 5];
    let mut zero: Option<Valued> = None;
    let mut physical_avg: [Option<Valued>; 2] = [None; 2];
    let mut leech_parts: [Vec<(DamageType, Valued)>; 2] = [Vec::new(), Vec::new()];
    let mut hits = Vec::new();
    let mut crits = Vec::new();
    let mut physical = false;
    let mut enemy_read = false;
    for (t, ty) in TYPES.into_iter().enumerate() {
        let mut range = Vec::new();
        for b in Bound::ALL {
            let source = match shape.damage {
                DamageSource::Skill => sum_in(
                    calc,
                    Quantity::Poe1(Poe1Quantity::SkillBaseDamage(ty, b)),
                    Op::Base,
                    &cx,
                    Term::Step("skill-base-damage"),
                    reasons,
                )?,
                DamageSource::Weapon(damage) => match damage[t] {
                    Some(range) => range[usize::from(b == Bound::Max)],
                    None => phases::constant(calc, "no-weapon-damage", 0.0)?,
                },
            };
            let added = sum_in(
                calc,
                Quantity::AddedDamage(ty, b),
                Op::Base,
                &cx,
                Term::Step("added-damage"),
                reasons,
            )?;
            let added = match shield_added(calc, ty, b, &cx, reasons)? {
                Some(extra) => {
                    phases::total(calc, &[added, extra], Term::Step("added-with-shield"))?
                }
                None => added,
            };
            let added = match mana_added(calc, ty, &cx, reasons)? {
                Some(extra) => phases::total(calc, &[added, extra], Term::Step("added-with-mana"))?,
                None => added,
            };
            // `source x baseMultiplier + added x damageEffectiveness`.
            let from_skill =
                phases::product(calc, &[source, base_multiplier], Term::Step("from-skill"))?;
            let from_added =
                phases::product(calc, &[added, effectiveness], Term::Step("from-added"))?;
            range.push(phases::total(calc, &[from_skill, from_added], Term::Base)?);
        }
        bases[t] = Some([range[0], range[1]]);
        // `canDeal[type]` (`CalcOffence.lua` 1874, 3319; Bitter Frost,
        // P3-8m): a type the hit cannot deal is skipped, its base kept for
        // the types it converts to.
        let deal_no = Quantity::Poe1(Poe1Quantity::DealNo(ty));
        if flag_on(calc, deal_no, &cx, reasons)?.is_some() {
            continue;
        }
        let converted_in = conversion
            .as_ref()
            .is_some_and(|c| (0..t).any(|from| c.share[from][t].is_some_and(|s| s.value > 0.0)));
        if range[0].value == 0.0 && range[1].value == 0.0 && !converted_in {
            continue;
        }
        let [min, max] = calc_damage(
            calc,
            t,
            Flags::NONE,
            &bases,
            conversion.as_ref(),
            &cx,
            shape.melee_physical,
            reasons,
            &mut zero,
        )?;
        if min.value == 0.0 && max.value == 0.0 {
            continue;
        }
        // `allMult = convMult x ScaledDamageEffect` (P3-8n).
        let kept = match (conversion.as_ref().and_then(|c| c.kept[t]), scaled) {
            (k, None) => k,
            (None, Some(s)) => Some(s),
            (Some(k), Some(s)) => Some(phases::product(
                calc,
                &[k, s],
                Term::Step("scaled-multiplier"),
            )?),
        };
        let effect = match resistance_index(ty) {
            Some(index) => {
                enemy_read = true;
                let resist = effective_resistance(calc, enemy, data, ty, index, &cx, reasons)?;
                let resist = percent(calc, resist, Term::Step("resistance-share"))?;
                let one = phases::constant(calc, "one", 1.0)?;
                Some(difference(calc, one, resist, Term::Step("enemy-effect"))?)
            }
            // Physical: the enemy's armour, against each pass's average.
            None => {
                physical = true;
                None
            }
        };
        // The enemy's damage taken of this type joins the hit's (P3-8m).
        let taken = if phases::has(calc, Quantity::Poe1(Poe1Quantity::EnemyTypeDamageTaken(ty))) {
            Some(type_damage_taken(calc, &cx, ty, reasons)?)
        } else {
            taken
        };
        let two = phases::constant(calc, "two", 2.0)?;
        for crit in [false, true] {
            // `allMult = convMult x ...`, times the crit multiplier on the
            // crit pass.
            let all = match (kept, crit) {
                (None, false) => None,
                (None, true) => Some(multiplier),
                (Some(k), false) => Some(k),
                (Some(k), true) => Some(phases::product(
                    calc,
                    &[k, multiplier],
                    Term::Step("all-multiplier"),
                )?),
            };
            let (lo, hi) = match all {
                Some(all) => {
                    let (lo_term, hi_term) = if crit {
                        ("crit-min", "crit-max")
                    } else {
                        ("kept-min", "kept-max")
                    };
                    (
                        phases::product(calc, &[min, all], Term::Step(lo_term))?,
                        phases::product(calc, &[max, all], Term::Step(hi_term))?,
                    )
                }
                None => (min, max),
            };
            let lo = quotient(calc, lo, two, Term::Step("half-min"))?;
            let hi = quotient(calc, hi, two, Term::Step("half-max"))?;
            let average = phases::total(calc, &[lo, hi], Term::Step("average"))?;
            if ty == DamageType::Physical
                && let Some(slot) = physical_avg.get_mut(usize::from(crit))
            {
                *slot = Some(average);
            }
            let effect = match effect {
                Some(e) => e,
                None => armour_effect(calc, enemy, data, average, reasons)?,
            };
            // `effMult = (1 + takenInc / 100) x takenMore x (1 -
            // resist / 100)`; the armour's reduction is computed on the
            // hit before it.
            let effect = match taken {
                Some(t) => phases::product(calc, &[t, effect], Term::Step("effective-multiplier"))?,
                None => effect,
            };
            let against = phases::product(calc, &[average, effect], Term::Step("against-enemy"))?;
            leech_parts[usize::from(crit)].push((ty, against));
            if crit {
                crits.push(against);
            } else {
                hits.push(against);
            }
        }
    }
    if enemy_read || physical {
        enemy_reasons(enemy, physical, reasons);
    }
    let hit = phases::total(calc, &hits, Term::Step("hit"))?;
    let crit = phases::total(calc, &crits, Term::Step("crit-hit"))?;
    Ok((
        hit,
        crit,
        HitSources {
            bases,
            conversion,
            physical: physical_avg,
            against: leech_parts,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rate_cap_should_be_pobs() {
        // PoB's `ServerTickRate = 1 / 0.033`, one division.
        assert_eq!((1.0 / SERVER_TICK_TIME).to_bits(), 0x403E_4D93_64D9_364D);
    }

    #[test]
    fn excluded_types_should_be_game_type_names() {
        for t in NOT_SELF_CAST.iter().chain(&NOT_PLAIN_ATTACK) {
            assert!(t.chars().all(|c| c.is_ascii_alphabetic()), "{t}");
        }
        // Spells by traps and mines are computed (P6-6e); attacks by them
        // are not: the attack list is the spell list's but `Attack`, plus
        // the two.
        let mut spells: Vec<&str> = NOT_SELF_CAST[1..].to_vec();
        spells.extend(["Trapped", "RemoteMined"]);
        spells.sort_unstable();
        let mut attacks = NOT_PLAIN_ATTACK.to_vec();
        attacks.sort_unstable();
        assert_eq!(spells, attacks);
        // The DoT leaves out the same kinds (P6-6d review).
        assert_eq!(
            &super::super::dot::NOT_COMPUTED_KINDS[..],
            &NOT_PLAIN_ATTACK[..]
        );
    }

    /// Every vetted totem skill would be `Complete` but for
    /// [`TOTEM_REASON`] (P6-6d review): the reason is what keeps PoB's
    /// totem data, which the engine does not read, from a wrong value.
    #[test]
    fn vetted_totem_skills_should_be_known() {
        let vetted: Vec<&str> = super::super::pob_skills::SKILLS
            .iter()
            .filter(|s| s.flag("totem") && pob_vetted(s.id, 0))
            .map(|s| s.id)
            .collect();
        assert!(vetted.contains(&"HolyFlameTotem"), "{vetted:?}");
        assert!(vetted.contains(&"SiegeBallistaAltY"), "{vetted:?}");
    }

    /// PoB's parts and flags (P6-6b): Smite's two parts as its data writes
    /// them, the rule's vetting, and a skill whose data the engine does not
    /// read (Ice Spear's `preDamageFunc`).
    #[test]
    fn pob_parts_and_vetting_should_follow_pobs_skill_data() {
        let (melee, i) = pob_part("Smite", 1).unwrap();
        assert_eq!(
            (melee.name, melee.melee, melee.area, i),
            ("Melee Hit", true, false, 1)
        );
        let (area, i) = pob_part("Smite", 9).unwrap();
        assert_eq!(
            (area.name, area.melee, area.area, i),
            ("Area Hit", false, true, 2)
        );
        assert!(pob_vetted("Smite", 1) && pob_vetted("Smite", 2));
        let (nova, i) = pob_part("IceNova", 1).unwrap();
        assert!(nova.area && !nova.projectile && i == 0);
        assert!(pob_vetted("IceNova", 0));
        let (knives, _) = pob_part("EtherealKnives", 1).unwrap();
        assert!(knives.projectile && !knives.area);
        assert!(!pob_vetted("IceSpear", 1));
        assert!(pob_part("NoSuchSkill", 1).is_none());
        assert!(!pob_vetted("NoSuchSkill", 0));
        // A release skill, whose hit time PoB sets in Lua (Snipe,
        // `CalcTriggers.lua` 1405 to 1418; P6-5 review).
        assert!(!pob_vetted("Snipe", 0));
        // Tagged stat map entries (P6-6b review): Conflagration's DoT in part
        // 2 only, Animosity's per stack.
        assert!(!pob_vetted("Conflagration", 0) && !pob_vetted("AnimosityUniqueSkill", 0));
        // Both flags in PoB's data, and vetted: the default attack. The
        // weapon picks one (`weapon_flags`).
        let (melee, i) = pob_part("Melee", 1).unwrap();
        assert!(melee.melee && melee.projectile && pob_vetted("Melee", i));
    }

    /// What the engine reads of PoB's hand-written data (open items, S1):
    /// Shield Crush's tagged stats in both parts, its `dpsMultiplier` read
    /// in part 2 (the agent list, 2026-11-27); Vortex's `preDamageFunc`
    /// only without a cooldown.
    #[test]
    fn read_lists_should_vet_only_what_the_engine_reads() {
        assert!(pob_vetted("ShieldCrush", 1));
        assert!(pob_vetted("ShieldCrush", 2));
        assert!((part_dps_multiplier("ShieldCrush", 2) - 2.0).abs() < f64::EPSILON);
        assert!((part_dps_multiplier("ShieldCrush", 1) - 1.0).abs() < f64::EPSILON);
        assert!((part_dps_multiplier("ShockNova", 1) - 2.0).abs() < f64::EPSILON);
        assert!((part_dps_multiplier("Smite", 1) - 1.0).abs() < f64::EPSILON);
        assert!(!pob_vetted("Vortex", 0));
        assert!(pob_vetted_with("Vortex", 0, true));
        assert!(!pob_vetted_with("IceSpear", 1, true));
        // Every listed entry is in PoB's data as listed.
        for (id, stat) in READ_OWN_STATS {
            let s = super::super::pob_skill::skill(id).unwrap();
            assert!(s.own_stats.contains(&stat), "{id} {stat}");
        }
        for (id, key, part) in READ_PART_BASE_MODS {
            let s = super::super::pob_skill::skill(id).unwrap();
            assert!(part <= s.parts.len(), "{id}");
            assert!(
                s.base_mods.iter().any(|m| m.tagged && m.key == Some(key)),
                "{id} {key}"
            );
        }
        for (id, f) in READ_FUNCTIONS {
            let s = super::super::pob_skill::skill(id).unwrap();
            assert!(s.functions.contains(&f), "{id} {f}");
        }
        // The generic lists name data some skill has, and vet the item
        // triggered skills they were made for.
        let skills = super::super::pob_skills::SKILLS;
        for stat in TRIGGER_OWN_STATS {
            assert!(skills.iter().any(|s| s.own_stats.contains(&stat)), "{stat}");
        }
        for flag in READ_BASE_FLAGS {
            assert!(
                skills.iter().any(|s| s
                    .base_mods
                    .iter()
                    .any(|m| m.kind == "FLAG" && m.name == flag)),
                "{flag}"
            );
        }
        for id in [
            "TriggeredMoltenStrike",
            "UnseenStrike",
            "RainOfArrows",
            "BoneNova",
        ] {
            assert!(pob_vetted(id, 0), "{id}");
        }
        // Icicle Burst's `showAverage` is read only when a trigger sets the
        // rate (open items, S1).
        assert!(!pob_vetted("IcicleBurst", 0));
        assert!(pob_vetted_triggered("IcicleBurst", 0, false));
        // A skill without `showAverage` that fails for another reason fails
        // triggered too.
        assert!(!pob_vetted_triggered("IceSpear", 1, true));
    }

    /// Open items, S1: Cyclone's and Vaal Cyclone's function and own stat
    /// reach only the area of effect; Cyclone of Tumult's stages are not read.
    #[test]
    fn cyclone_should_be_vetted_by_its_area_only_data() {
        assert!(pob_vetted("Cyclone", 0) && pob_vetted("VaalCyclone", 0));
        // Ground Slam's close-range stat is the mapping's state row.
        assert!(pob_vetted("GroundSlam", 0));
        assert!(!pob_vetted("CycloneAltX", 0));
        for (id, f) in NO_SLICE_FUNCTIONS {
            let pob = super::super::pob_skill::skill(id).expect(id);
            assert!(pob.functions.contains(&f), "{id} {f}");
        }
        for (id, s) in NO_SLICE_OWN_STATS {
            let pob = super::super::pob_skill::skill(id).expect(id);
            assert!(pob.own_stats.contains(&s), "{id} {s}");
            assert!(
                super::super::mapping::NO_EFFECT
                    .iter()
                    .any(|(n, _)| *n == s),
                "{s} has an effect"
            );
        }
    }

    /// Open items, S1: Spark's function does nothing in part 1, "1 Hit";
    /// part 2 multiplies the DPS by its hits and is not read.
    #[test]
    fn spark_should_be_vetted_in_its_first_part_only() {
        assert!(pob_vetted("Spark", 1));
        assert!(!pob_vetted("Spark", 2));
        for (id, f, _) in NOOP_PART_FUNCTIONS {
            let pob = super::super::pob_skill::skill(id).expect(id);
            assert!(pob.functions.contains(&f), "{id} {f}");
        }
    }

    /// Agent list: Static Strike's data is read in both parts (its function,
    /// its part-2 stat, its parts' `chaining`), and Shockwave Support's
    /// attack has none to read.
    #[test]
    fn static_strike_and_shockwave_should_be_vetted() {
        assert!(pob_vetted("StaticStrike", 1));
        assert!(pob_vetted("StaticStrike", 2));
        assert!(pob_vetted("SupportBluntWeaponShockwave", 0));
        let pob = super::super::pob_skill::skill("StaticStrike").expect("Static Strike");
        assert!(pob.own_stats.contains(&BEAM_DAMAGE_STAT));
        assert!(pob.functions.contains(&"preDamageFunc"));
        let (beams, index) = pob_part("StaticStrike", 2).expect("parts");
        assert_eq!((beams.name, index), ("Beams", 2));
    }

    /// PoB's weapon rule (P6-6c review): a melee weapon keeps melee, a bow
    /// or wand projectile; a flag the weapon does not clear stays.
    #[test]
    fn the_weapon_should_pick_melee_or_projectile() {
        assert_eq!(weapon_flags("One Hand Axe", true, true), (true, false));
        assert_eq!(weapon_flags("Bow", true, true), (false, true));
        assert_eq!(weapon_flags("Wand", true, true), (false, true));
        assert_eq!(weapon_flags("Claw", false, true), (false, true));
        assert_eq!(weapon_flags("Wand", true, false), (true, false));
        assert_eq!(weapon_flags("Staff", false, false), (false, false));
    }

    /// PoB's `getWeaponFlags` per item class (P6-11b): type, `Weapon`,
    /// hands, and range; an unknown class has none.
    #[test]
    fn weapon_type_flags_should_follow_pobs_weapon_types() {
        let f = |c| weapon_type_flags(c).expect(c);
        let melee1 = Flags::WEAPON | Flags::WEAPON_1H | Flags::WEAPON_MELEE;
        let melee2 = Flags::WEAPON | Flags::WEAPON_2H | Flags::WEAPON_MELEE;
        assert_eq!(
            f("Bow"),
            Flags::BOW | Flags::WEAPON | Flags::WEAPON_2H | Flags::WEAPON_RANGED
        );
        assert_eq!(
            f("Wand"),
            Flags::WAND | Flags::WEAPON | Flags::WEAPON_1H | Flags::WEAPON_RANGED
        );
        assert_eq!(f("Claw"), Flags::CLAW | melee1);
        assert_eq!(f("Rune Dagger"), Flags::DAGGER | melee1);
        assert_eq!(f("Warstaff"), Flags::STAFF | melee2);
        assert_eq!(f("Sceptre"), Flags::MACE | melee1);
        assert_eq!(f("Two Hand Mace"), Flags::MACE | melee2);
        assert_eq!(f("Thrusting One Hand Sword"), Flags::SWORD | melee1);
        assert_eq!(f("Two Hand Axe"), Flags::AXE | melee2);
        assert_eq!(f("FishingRod"), Flags::FISHING | melee2);
        assert_eq!(weapon_type_flags("Shield"), None);
        // An ailment keeps two-handed but not one-handed (PoB's
        // `WeaponMask`, 0x2FFF0000).
        let kept = |c| Flags::from_bits_and(f(c), Flags::AILMENT_WEAPON_MASK);
        assert_eq!(
            kept("Two Hand Axe"),
            Flags::AXE | Flags::WEAPON_2H | Flags::WEAPON_MELEE
        );
        assert_eq!(kept("Claw"), Flags::CLAW | Flags::WEAPON_MELEE);
    }
}
