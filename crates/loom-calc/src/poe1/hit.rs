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
//! damage taken of one type or by projectiles, traps, and mines, repeats,
//! the enemy's block, the hit chance modifier) is left out: a stat that
//! would change it is unmapped and makes the fields `Partial`. The default
//! enemy's other modifiers reach no mapped stat
//! (`poe1::enemy` module docs).
//!
//! **Which skills** (`spell_skill`, `attack_skill`): a spell and a hit
//! (PoB's `skillFlags.hit`: a `Damage` or `Projectile` type), cast by the
//! player: not an attack, channelled, triggered, a totem, trap, mine,
//! minion, or brand, without a cooldown, with a cast time; or an attack
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

use loom_domain::{Build, ItemId};

use super::collect;
use super::enemy::{self, Enemy, EnemyProblem};
use super::gem_levels::GemLevels;
use super::keystones;
use super::phases::{self, Valued};
use super::skill::{self, MainSkill};
use super::{Poe1Quantity, WeaponValue};
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
/// weapons, channelling stages, triggers, totems, traps, mines, minions,
/// brands' activations).
pub const NOT_SELF_CAST: [&str; 9] = [
    "Attack",
    "Channel",
    "Triggered",
    "SummonsTotem",
    "Trapped",
    "RemoteMined",
    "Minion",
    "CreatesMinion",
    "Brand",
];

/// Skill types that make an attack something other than a plain attack
/// with the weapon in hand ([`NOT_SELF_CAST`] but `Attack`).
pub const NOT_PLAIN_ATTACK: [&str; 8] = [
    "Channel",
    "Triggered",
    "SummonsTotem",
    "Trapped",
    "RemoteMined",
    "Minion",
    "CreatesMinion",
    "Brand",
];

/// The spells whose hit the oracle has checked (P3-9b's mechanic fixtures):
/// another spell is computed but `Partial` (`unmodelled-skill`,
/// `pob-skill-data`) until a PR adds its fixtures. PoB also computes a skill
/// from its own skill data (parts, cast time overrides, skill functions,
/// the hand-written `area` and `projectile` flags), which the game data
/// does not carry.
pub const VETTED_SPELLS: [&str; 1] = ["IceNova"];

/// The attacks whose hit the oracle has checked (P3-10's mechanic
/// fixtures); another attack is computed but `Partial` as for spells
/// ([`VETTED_SPELLS`]).
pub const VETTED_ATTACKS: [&str; 1] = ["Smite"];

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
}

/// The parts of the vetted attacks, from PoB v2.67.2's skill data
/// (`Data/Skills/act_str.lua`, `skills["Smite"]`: `baseFlags` melee and
/// area; part 1 "Melee Hit" `area = false`, part 2 "Area Hit" `area = true,
/// melee = false`). The game data has no parts.
pub const SKILL_PARTS: [(&str, &[SkillPart]); 1] = [(
    "Smite",
    &[
        SkillPart {
            name: "Melee Hit",
            melee: true,
            area: false,
        },
        SkillPart {
            name: "Area Hit",
            melee: false,
            area: true,
        },
    ],
)];

/// PoB's damage types in its conversion order (`dmgTypeList`), the order
/// it sums each pass in.
const TYPES: [DamageType; 5] = [
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
/// PoB's `data.misc.ServerTickTime`; the rate cap is `1 / 0.033` (times
/// the repeat count, 1).
const SERVER_TICK_TIME: f64 = 0.033;
/// PoB's `data.misc.AccuracyPerDexBase`: accuracy rating per dexterity.
const ACCURACY_PER_DEXTERITY: f64 = 2.0;

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
    /// Whether its hits are area damage (`is_area_damage`).
    pub area: bool,
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
    /// The skill part computed.
    pub part: SkillPart,
    /// Whether PoB's parts of the skill are known ([`SKILL_PARTS`]); if
    /// not, the part's flags are the game's (`Melee` type,
    /// `is_area_damage`).
    pub known_parts: bool,
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
    if !has("Spell")
        || !(has("Damage") || has("Projectile"))
        || NOT_SELF_CAST.iter().any(|t| has(t))
    {
        return None;
    }
    let effect = skill.effect();
    let gem = skill.active_gem;
    if effect.cast_time <= 0 || skill::effect_level(effect, gem.level)?.cooldown != 0 {
        return None;
    }
    let level = skill::stat_set_level(effect, gem.level, data)?;
    let area = is_area_damage(&skill, data);
    Some(SpellSkill {
        skill,
        group,
        level,
        cast_time: effect.cast_time,
        area,
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
    if !has("Attack") || NOT_PLAIN_ATTACK.iter().any(|t| has(t)) {
        return None;
    }
    let effect = skill.effect();
    let gem = skill.active_gem;
    let level_row = skill::effect_level(effect, gem.level)?;
    if level_row.cooldown != 0 {
        return None;
    }
    let level = skill::stat_set_level(effect, gem.level, data)?;
    let hands = collect::hands(input);
    if hands.off_hand_weapon {
        return None;
    }
    let (weapon, base) = hands.main?;
    // PoB disables a skill its weapon cannot use (`weaponTypes`); the game
    // lists the classes per active skill (none: any weapon).
    if let Some(active) = skill.active
        && !active.weapon_classes.is_empty()
        && !active.weapon_classes.contains(&base.class)
    {
        return None;
    }
    let parts = SKILL_PARTS
        .iter()
        .find(|(id, _)| *id == effect.id)
        .map(|(_, parts)| *parts);
    let (part, known_parts) = match parts {
        Some(parts) => {
            // PoB: `m_min(#parts, skillPart or 1)`; an empty or missing
            // attribute is 1.
            let wanted = main_group
                .gems
                .typed()
                .nth(gem.gem)
                .and_then(|g| skill::attribute(&g.attributes, "skillPart"))
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(1)
                .min(parts.len());
            (*parts.get(wanted.checked_sub(1)?)?, true)
        }
        None => (
            SkillPart {
                name: "",
                melee: has("Melee"),
                area: is_area_damage(&skill, data),
            },
            false,
        ),
    };
    Some(AttackSkill {
        skill,
        group,
        level,
        attack_speed_multiplier: level_row.attack_speed_multiplier,
        weapon,
        part,
        known_parts,
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
    for e in calc.store().effects() {
        if e.quantity != quantity || e.op != Op::More {
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
        let skill = matches!(e.source, Source::Gem { .. });
        groups
            .entry((!skill, name_order(e)))
            .or_default()
            .push(e.clone());
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
    match hit_skill(&input, &levels) {
        None => Ok(()),
        Some(HitSkill::Spell(spell)) => spell_hit(calc, &spell),
        Some(HitSkill::Attack(attack)) => attack_hit(calc, &attack),
    }
}

/// A self-cast spell's hit.
fn spell_hit(calc: &mut Calc<'_>, spell: &SpellSkill<'_>) -> Result<(), CalcError> {
    let input = *calc.input();
    let enemy = enemy::enemy(input.build, input.data);
    let mut flags = Flags::SPELL | Flags::CAST | Flags::HIT | spell.skill.keyword_flags();
    if spell.area {
        flags = flags | Flags::AREA;
    }
    let cx = calc.context(flags);
    // A spell PoB's oracle has not checked: PoB computes it from its own
    // skill data too (parts, cast time overrides, skill functions, flags),
    // which the game data does not carry.
    let mut unvetted = Reasons::new();
    if !VETTED_SPELLS.contains(&spell.skill.effect().id.as_str()) {
        unvetted.partial.insert(UnsupportedReason::UnmodelledSkill {
            source: spell.source(),
            what: "pob-skill-data",
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
    };
    let (hit, crit_hit) = passes(calc, &shape, &enemy, input.data, multiplier, &mut reasons)?;
    let average = average_damage(calc, hit, crit_hit, crit, None)?;
    calc.set_derived(
        Field::AverageDamage,
        phases::query(average, reasons),
        &[Field::CritChance, Field::CritMultiplier],
    );

    let (speed, mut speed_reasons) = cast_rate(calc, spell, &cx)?;
    speed_reasons.extend(&unvetted);
    calc.set(Field::Speed, phases::query(speed, speed_reasons));
    hit_dps(calc, average, speed)
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

/// `output.TotalDPS = AverageDamage x (HitSpeed or Speed) x dpsMultiplier
/// x quantityMultiplier`; both multipliers are 1.
fn hit_dps(calc: &mut Calc<'_>, average: Valued, speed: Valued) -> Result<(), CalcError> {
    let rate = phases::total(calc, &[speed], Term::Step("rate"))?;
    let dps_multiplier = phases::product(calc, &[], Term::Step("dps-multiplier"))?;
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

/// An attack's hit with the weapon in the main hand (module docs).
fn attack_hit(calc: &mut Calc<'_>, attack: &AttackSkill<'_>) -> Result<(), CalcError> {
    let input = *calc.input();
    let enemy = enemy::enemy(input.build, input.data);
    let mut flags = Flags::ATTACK | Flags::HIT | attack.skill.keyword_flags();
    if attack.part.melee {
        flags = flags | Flags::MELEE;
    }
    if attack.part.area {
        flags = flags | Flags::AREA;
    }
    let cx = calc.context(flags);
    let mut unvetted = Reasons::new();
    let id = attack.skill.effect().id.as_str();
    if !VETTED_ATTACKS.contains(&id) || !attack.known_parts {
        unvetted.partial.insert(UnsupportedReason::UnmodelledSkill {
            source: attack.source(),
            what: "pob-skill-data",
        });
    }
    // PoB takes a main-hand attack's base crit from the weapon alone
    // (`source.CritChance`); a level's own attack crit chance is read only
    // for the off hand.
    if attack.level.attack_crit_chance != 0 {
        unvetted.partial.insert(UnsupportedReason::UnmodelledSkill {
            source: attack.source(),
            what: "skill-crit-chance",
        });
    }
    let mut weapon_reasons = Reasons::new();
    let weapon = weapon_data(calc, attack.weapon, &mut weapon_reasons)?;
    unvetted.extend(&weapon_reasons);

    // Chance to hit (step 9).
    let mut hit_reasons = unvetted.clone();
    // `CannotBeEvaded`: `AccuracyHitChance = 100` (`CalcOffence.lua` 2162;
    // Resolute Technique, P3-8m): neither accuracy nor the enemy's evasion
    // is read.
    let cannot_be_evaded = Quantity::Poe1(Poe1Quantity::CannotBeEvaded);
    let hit_chance = match flag_on(calc, cannot_be_evaded, &cx, &mut hit_reasons)? {
        Some(flag) => by_flag(calc, flag, "hits-cannot-be-evaded", 100.0)?,
        None => {
            enemy_reasons(&enemy, true, &mut hit_reasons);
            let accuracy = accuracy(calc, &cx, &mut hit_reasons)?;
            accuracy_hit_chance(calc, &enemy, accuracy, &cx, &mut hit_reasons)?
        }
    };
    calc.set_derived(
        Field::HitChance,
        phases::query(hit_chance, hit_reasons),
        &[Field::Dexterity],
    );

    let (crit, mut crit_reasons) = crit_chance(calc, weapon.crit, Some(hit_chance), &cx)?;
    crit_reasons.extend(&unvetted);
    calc.set_derived(
        Field::CritChance,
        phases::query(crit, crit_reasons),
        &[Field::HitChance],
    );
    let (multiplier, mut multiplier_reasons) = crit_multiplier(calc, input.data, &cx)?;
    multiplier_reasons.extend(&unvetted);
    calc.set(
        Field::CritMultiplier,
        phases::query(multiplier, multiplier_reasons),
    );

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
    };
    let (hit, crit_hit) = passes(calc, &shape, &enemy, input.data, multiplier, &mut reasons)?;
    let average = average_damage(calc, hit, crit_hit, crit, Some(hit_chance))?;
    calc.set_derived(
        Field::AverageDamage,
        phases::query(average, reasons),
        &[
            Field::CritChance,
            Field::CritMultiplier,
            Field::HitChance,
            Field::Strength,
        ],
    );

    let (speed, mut speed_reasons) = attack_rate(calc, attack, weapon.rate, &cx)?;
    speed_reasons.extend(&unvetted);
    calc.set(Field::Speed, phases::query(speed, speed_reasons));
    hit_dps(calc, average, speed)
}

/// The weapon's values (module docs, step 8).
struct WeaponData {
    /// Damage per type in [`TYPES`] order: `[min, max]` when both are
    /// above 0.
    damage: [Option<[Valued; 2]>; 5],
    /// Attacks per second.
    rate: Valued,
    /// Critical strike chance, percent.
    crit: Valued,
}

/// The weapon in the main hand's values (module docs, step 8).
fn weapon_data(
    calc: &mut Calc<'_>,
    item: ItemId,
    reasons: &mut Reasons,
) -> Result<WeaponData, CalcError> {
    let weapon = |v| Quantity::Poe1(Poe1Quantity::Weapon(v));
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
            slot: "Weapon 1".to_owned(),
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
    if let Some((node, value)) = calc
        .field(Field::Dexterity)
        .and_then(|f| Some((f.node?, f.value?)))
    {
        let per = phases::constant(calc, "accuracy-per-dexterity", ACCURACY_PER_DEXTERITY)?;
        terms.push(phases::product(
            calc,
            &[Valued { node, value }, per],
            Term::Step("from-dexterity"),
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
    let value = phases::pair(calc, capped, floor, Term::Result, false)?;
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
    match data.character_constant(CRIT_MULTIPLIER_STAT) {
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
    let cast_time = leaf(
        calc,
        spell.source(),
        Quantity::Poe1(Poe1Quantity::CastTime),
        seconds,
    )?;
    rate(calc, cast_time, cx)
}

/// Attack rate (module docs, step 6): the weapon's rate times the skill's
/// attack speed multiplier (`source.AttackRate x (1 +
/// attackSpeedMultiplier / 100)`) gives the base time, `1 / rate`.
fn attack_rate(
    calc: &mut Calc<'_>,
    attack: &AttackSkill<'_>,
    weapon_rate: Valued,
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
    rate(calc, base_time, cx)
}

/// `min(1 / (base time / round((1 + inc / 100) x more, 2)), 1 / 0.033)`.
fn rate(
    calc: &mut Calc<'_>,
    base_time: Valued,
    cx: &QueryContext,
) -> Result<(Valued, Reasons), CalcError> {
    let mut reasons = Reasons::new();
    let (inc, more) = modifiers(calc, Quantity::Speed, cx, &mut reasons)?;
    let modifier = phases::product(calc, &[inc, more], Term::Step("modifier"))?;
    let modifier = round_2(calc, modifier, Term::Step("modifier-rounded"))?;
    let time = quotient(calc, base_time, modifier, Term::Step("time"))?;
    let one = phases::constant(calc, "one", 1.0)?;
    let rate = quotient(calc, one, time, Term::Step("uncapped"))?;
    let rate = action_speed(calc, rate, &mut reasons)?;
    let cap = phases::constant(calc, "server-tick-rate", 1.0 / SERVER_TICK_TIME)?;
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
    let quantity = Quantity::Poe1(Poe1Quantity::ActionSpeed);
    if !phases::has(calc, quantity) {
        return Ok(rate);
    }
    let cx = calc.context(Flags::NONE);
    let modifier = increased(calc, quantity, &cx, reasons)?;
    let zero = phases::constant(calc, "no-minimum-action-speed", 0.0)?;
    let modifier = phases::pair(calc, modifier, zero, Term::Step("action-speed"), false)?;
    phases::product(calc, &[rate, modifier], Term::Step("action-speed-scaled"))
}

/// PoB's `ScaledDamageEffect` from double damage (`CalcOffence.lua` 3196
/// to 3209): `1 + min(DoubleDamageChance, 100) / 100`, with no triple
/// damage or double damage on crit mapped (P3-8n). `None` when the store
/// has no double damage effect.
fn double_damage(
    calc: &mut Calc<'_>,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<Option<Valued>, CalcError> {
    let quantity = Quantity::Poe1(Poe1Quantity::DoubleDamageChance);
    if !phases::has(calc, quantity) {
        return Ok(None);
    }
    let chance = sum_in(
        calc,
        quantity,
        Op::Base,
        cx,
        Term::Step("double-damage-chance"),
        reasons,
    )?;
    let hundred = phases::constant(calc, "hundred", 100.0)?;
    let chance = phases::pair(
        calc,
        chance,
        hundred,
        Term::Step("double-damage-capped"),
        true,
    )?;
    let effect = percent(calc, chance, Term::Step("double-damage-effect"))?;
    let one = phases::constant(calc, "one", 1.0)?;
    Ok(Some(phases::total(
        calc,
        &[one, effect],
        Term::Step("scaled-damage-effect"),
    )?))
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
    let base = enemy.resistances[index].unwrap_or(0.0);
    let resist = leaf(calc, Source::Enemy, Quantity::Resistance(t), base)?;
    // The enemy's `<Type>Resist` BASE sums its base and the exposure
    // (`calcResistForType`; P3-8k, [`super::exposure`]), only when there
    // is one, so a hit without it keeps its graph.
    let exposure = Quantity::Poe1(Poe1Quantity::EnemyResistance(t));
    let resist = if phases::has(calc, exposure) {
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
    let resist = phases::pair(calc, capped, floor, Term::Step("enemy-resistance"), false)?;
    let pen = sum_in(
        calc,
        Quantity::Penetration(t),
        Op::Base,
        cx,
        Term::Step("penetration"),
        reasons,
    )?;
    difference(calc, resist, pen, Term::Step("effective-resistance"))
}

/// The share of a physical hit the enemy's armour lets through (module
/// docs, step 3): `1 - min(max(0, armourReductionF(armour, average)),
/// cap) / 100`.
fn armour_effect(
    calc: &mut Calc<'_>,
    enemy: &Enemy,
    data: &dyn CalcData,
    average: Valued,
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
    let zero = phases::constant(calc, "no-negative-reduction", 0.0)?;
    let reduction = phases::pair(calc, zero, reduction, Term::Step("reduction"), false)?;
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
    let reduction = phases::pair(calc, reduction, cap, Term::Step("reduction-capped"), true)?;
    let share = percent(calc, reduction, Term::Step("reduction-share"))?;
    let one = phases::constant(calc, "one", 1.0)?;
    difference(calc, one, share, Term::Step("enemy-effect"))
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
    if !calc
        .store()
        .effects()
        .iter()
        .any(|e| e.quantity == quantity)
    {
        return Ok(None);
    }
    let (inc, more) = modifiers(calc, quantity, cx, reasons)?;
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
    let sum = phases::total(calc, &[inc_all, inc_typed], Term::Step("increased-sum"))?;
    let inc = phases::factor(calc, sum, Term::Increased)?;
    let more_all = more(calc, all, cx, reasons)?;
    let more_typed = more(calc, typed, cx, reasons)?;
    phases::product(
        calc,
        &[inc, more_all, more_typed],
        Term::Step("enemy-damage-taken"),
    )
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
}

/// The skill's conversion table (module docs, step 2), when it converts:
/// `share[from][to]` the share of `from` converted to `to`, `kept[from]`
/// the share `from` keeps (`conversionTable[type].mult`).
struct Conversion {
    share: [[Option<Valued>; 5]; 5],
    kept: [Option<Valued>; 5],
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
    let any = (0..4).any(|from| {
        (from + 1..5)
            .any(|to| phases::has(calc, skill_q(from, to)) || phases::has(calc, global_q(from, to)))
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
            for (to, value) in skill {
                let value = match factor {
                    Some(f) => phases::product(calc, &[value, f], Term::Step("conversion-scaled"))?,
                    None => value,
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
            let value = match (g, k) {
                (None, None) => continue,
                (Some(g), None) => phases::total(calc, &[g, zero], Term::Step("conversion-sum"))?,
                (None, Some(k)) => phases::total(calc, &[zero, k], Term::Step("conversion-sum"))?,
                (Some(g), Some(k)) => phases::total(calc, &[g, k], Term::Step("conversion-sum"))?,
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
fn calc_damage(
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
    let (inc, more) = match bonus {
        None => modifiers(calc, Quantity::Damage, &typed, reasons)?,
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
            let sum = phases::total(calc, &[effects, bonus], Term::Step("increased-sum"))?;
            let inc = phases::factor(calc, sum, Term::Increased)?;
            let more = more(calc, Quantity::Damage, &typed, reasons)?;
            (inc, more)
        }
    };
    let mut rolled = Vec::with_capacity(2);
    for (bound, b) in base.into_iter().enumerate() {
        let mut raw = phases::product(calc, &[b, inc, more], Term::Step("unrounded"))?;
        if let Some(c) = converted {
            raw = phases::total(calc, &[raw, c[bound]], Term::Step("with-converted"))?;
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
) -> Result<(Valued, Valued), CalcError> {
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
    let effectiveness = leaf(
        calc,
        shape.source.clone(),
        Quantity::Poe1(Poe1Quantity::DamageEffectiveness),
        ratio(shape.level.damage_effectiveness.unwrap_or(0)),
    )?;
    let base_multiplier = leaf(
        calc,
        shape.source.clone(),
        Quantity::Poe1(Poe1Quantity::BaseDamageMultiplier),
        ratio(shape.level.base_multiplier),
    )?;
    let conversion = conversion(calc, &cx, reasons)?;
    let scaled = double_damage(calc, &cx, reasons)?;
    let taken = enemy_damage_taken(calc, &cx, reasons)?;
    let mut bases: [Option<[Valued; 2]>; 5] = [None; 5];
    let mut zero: Option<Valued> = None;
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
    Ok((hit, crit))
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
        assert_eq!(&NOT_SELF_CAST[1..], &NOT_PLAIN_ATTACK[..]);
    }

    #[test]
    fn skill_parts_should_be_vetted_attacks() {
        for (id, parts) in SKILL_PARTS {
            assert!(VETTED_ATTACKS.contains(&id), "{id}");
            assert!(!parts.is_empty());
        }
    }
}
