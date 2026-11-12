//! The main skill's hit when it is a self-cast spell (Phase 3 plan, step
//! 9b): its average damage against the enemy, cast rate, critical strike
//! chance and multiplier, and hit DPS, as PoB v2.67.2 computes them
//! (`Modules/CalcOffence.lua`, `calcs.offence`), with the same order of
//! operations and rounding points. Its steps, each cited below:
//!
//! 1. **Base damage** per damage type (`Calculate base hit damage`): the
//!    skill's own (`skillData.ColdMin`, the gem's resolved level value)
//!    times its base multiplier, plus added damage times the skill's damage
//!    effectiveness: `base x multiplier + added x effectiveness`.
//! 2. **Damage** per type (`calcDamage`, no conversion):
//!    `round(base x (1 + inc / 100) x more)`, the increases and more of
//!    `Damage` and the type's own (`ColdDamage`, `ElementalDamage`). Every
//!    more multiplier here follows PoB's rule: each mod name's product
//!    rounded to two decimals, per list (the skill's gems, the character).
//! 3. **Each pass** (non-crit, then crit with the multiplier): per type
//!    `min / 2 + max / 2` (no luck), times the enemy's effect `1 - (resist -
//!    penetration) / 100`, the resistance capped at its maximum (75, or the
//!    configured resistance above it, at most 90) and at least -200
//!    (`calcResistForType`); summed over the types in PoB's order
//!    (physical, lightning, cold, fire, chaos).
//! 4. **Crit chance**: `min((base + added) x (1 + inc / 100) x more, 100)`,
//!    at least 0, then times the hit chance (100 for spells) / 100.
//!    **Crit multiplier**: `1 + max(0, round(sum / 100, 2))`, the sum from
//!    the character's base (the template's 150, less 100) and the stats.
//! 5. **Average hit** `hit x (1 - crit / 100) + crit hit x crit / 100`;
//!    **average damage** its product with the hit chance (100) / 100.
//! 6. **Cast rate** `1 / (cast time / round((1 + inc / 100) x more, 2))`,
//!    at most the server tick rate (`1 / 0.033`).
//! 7. **Hit DPS** `average damage x rate x 1 x 1` (PoB's DPS and quantity
//!    multipliers, which no mapped stat changes).
//!
//! Every factor PoB applies that is 1 or 0 without a stat the mapping does
//! not know (conversion, luck, double damage, action speed, damage taken,
//! repeats, the enemy's block) is left out: a stat that would change it is
//! unmapped and makes the fields `Partial`. The default enemy's other
//! modifiers reach no mapped stat (`poe1::enemy` module docs).
//!
//! **Which skills** (`spell_skill`): the main skill when it is a spell and
//! a hit (PoB's `skillFlags.hit`: a `Damage` or `Projectile` type), cast
//! by the player: not an attack, channelled, triggered, a totem, trap,
//! mine, minion, or brand, without a cooldown, with a cast time, in an
//! enabled group. Every other main skill leaves the hit fields not
//! computed. Its area flag is the game's `is_area_damage` stat (PoB's
//! hand-written `area` flag; equal for Ice Nova). Physical hit damage
//! needs the enemy's armour, which is not modelled: the fields are
//! `Partial`.

use std::collections::BTreeMap;

use loom_domain::Build;

use super::Poe1Quantity;
use super::enemy::{self, Enemy, EnemyProblem};
use super::phases::{self, Valued};
use super::skill::{self, MainSkill};
use crate::condition::{Applies, Flags, QueryContext};
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::{Node, Term};
use crate::input::{CalcData, StatSetLevel};
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

/// The spells whose hit the oracle has checked (P3-9b's mechanic fixtures):
/// another spell is computed but `Partial` (`unmodelled-skill`,
/// `pob-skill-data`) until a PR adds its fixtures. PoB also computes a skill
/// from its own skill data (parts, cast time overrides, skill functions,
/// the hand-written `area` and `projectile` flags), which the game data
/// does not carry.
pub const VETTED_SPELLS: [&str; 1] = ["IceNova"];

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

/// The enemy's base maximum resistance (PoB's `data.misc.EnemyMaxResist`,
/// the monster template's).
const ENEMY_MAX_RESIST_STAT: &str = "base_maximum_all_resistances_%";
/// The character's base critical strike multiplier (PoB's
/// `CritMultiplier` BASE `base_critical_strike_multiplier - 100`).
const CRIT_MULTIPLIER_STAT: &str = "base_critical_strike_multiplier";

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

/// The build's main skill when the engine computes its hit as a self-cast
/// spell (module docs), else `None`.
#[must_use]
pub fn spell_skill<'d>(build: &Build, data: &'d dyn CalcData) -> Option<SpellSkill<'d>> {
    let skill = skill::main_skill(build, data).ok()?;
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
    let area = skill::instance(effect, gem.level, gem.quality, data)
        .stats
        .iter()
        .any(|(s, v)| s == "is_area_damage" && *v != 0);
    Some(SpellSkill {
        skill,
        group,
        level,
        cast_time: effect.cast_time,
        area,
    })
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

/// `1 + inc / 100` and the more product ([`more`]) of `quantity` in `cx`.
fn modifiers(
    calc: &mut Calc<'_>,
    quantity: Quantity,
    cx: &QueryContext,
    reasons: &mut Reasons,
) -> Result<(Valued, Valued), CalcError> {
    let (store, graph) = calc.parts();
    let inc = store.increased(graph, quantity, cx)?;
    reasons.extend(&inc.reasons);
    let inc = Valued {
        node: inc.node,
        value: inc.value,
    };
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

/// The hit phase (module docs). A main skill that is not a self-cast
/// spell leaves the hit fields not computed.
///
/// # Errors
/// A graph error.
pub fn spell_hit(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let input = *calc.input();
    let Some(spell) = spell_skill(input.build, input.data) else {
        return Ok(());
    };
    let enemy = enemy::enemy(input.build, input.data);
    let mut flags = Flags::SPELL | Flags::CAST | Flags::HIT;
    if spell.area {
        flags = flags | Flags::AREA;
    }
    let cx = QueryContext::with_flags(flags);
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

    let (crit, mut crit_reasons) = crit_chance(calc, &spell, &cx)?;
    crit_reasons.extend(&unvetted);
    calc.set(Field::CritChance, phases::query(crit, crit_reasons));
    let (multiplier, mut multiplier_reasons) = crit_multiplier(calc, input.data, &cx)?;
    multiplier_reasons.extend(&unvetted);
    calc.set(
        Field::CritMultiplier,
        phases::query(multiplier, multiplier_reasons),
    );

    let mut reasons = unvetted.clone();
    let (hit, crit_hit) = passes(
        calc,
        &spell,
        &enemy,
        input.data,
        flags,
        multiplier,
        &mut reasons,
    )?;
    // `output.AverageHit = totalHitAvg x (1 - CritChance / 100) +
    // totalCritAvg x CritChance / 100`.
    let crit_share = percent(calc, crit, Term::Step("crit-share"))?;
    let one = phases::constant(calc, "one", 1.0)?;
    let non_crit_share = difference(calc, one, crit_share, Term::Step("non-crit-share"))?;
    let from_hits = phases::product(calc, &[hit, non_crit_share], Term::Step("from-non-crits"))?;
    let crit_scaled = phases::product(calc, &[crit_hit, crit], Term::Step("crit-scaled"))?;
    let from_crits = percent(calc, crit_scaled, Term::Step("from-crits"))?;
    let average_hit = phases::total(calc, &[from_hits, from_crits], Term::Step("average-hit"))?;
    // `output.AverageDamage = AverageHit x HitChance / 100`.
    let hit_chance = phases::constant(calc, "spell-hit-chance", SPELL_HIT_CHANCE)?;
    let scaled = phases::product(
        calc,
        &[average_hit, hit_chance],
        Term::Step("hit-chance-scaled"),
    )?;
    let average = percent(calc, scaled, Term::Result)?;
    calc.set_derived(
        Field::AverageDamage,
        phases::query(average, reasons),
        &[Field::CritChance, Field::CritMultiplier],
    );

    let (speed, mut speed_reasons) = cast_rate(calc, &spell, &cx)?;
    speed_reasons.extend(&unvetted);
    calc.set(Field::Speed, phases::query(speed, speed_reasons));

    // `output.TotalDPS = AverageDamage x (HitSpeed or Speed) x
    // dpsMultiplier x quantityMultiplier`; both multipliers are 1.
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

/// Critical strike chance (module docs, step 4; `calcs.offence`, "Calculate
/// crit chance"): the skill's base is PoB's `critChance`, the level's
/// attack crit chance when not 0, else its spell crit chance, in hundredths
/// (P3-2's data parity rule).
fn crit_chance(
    calc: &mut Calc<'_>,
    spell: &SpellSkill<'_>,
    cx: &QueryContext,
) -> Result<(Valued, Reasons), CalcError> {
    let mut reasons = Reasons::new();
    let level = spell.level;
    let hundredths = if level.attack_crit_chance == 0 {
        level.spell_crit_chance
    } else {
        level.attack_crit_chance
    };
    #[expect(
        clippy::cast_precision_loss,
        reason = "crit chances are small integers, exact as f64"
    )]
    let skill_base = hundredths as f64 / 100.0;
    let skill_base = leaf(
        calc,
        spell.source(),
        Quantity::Poe1(Poe1Quantity::SkillCritChance),
        skill_base,
    )?;
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
    let hit_chance = phases::constant(calc, "spell-hit-chance", SPELL_HIT_CHANCE)?;
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
    let mut reasons = Reasons::new();
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
    let (inc, more) = modifiers(calc, Quantity::Speed, cx, &mut reasons)?;
    let modifier = phases::product(calc, &[inc, more], Term::Step("modifier"))?;
    let modifier = round_2(calc, modifier, Term::Step("modifier-rounded"))?;
    let time = quotient(calc, cast_time, modifier, Term::Step("time"))?;
    let one = phases::constant(calc, "one", 1.0)?;
    let rate = quotient(calc, one, time, Term::Step("uncapped"))?;
    let cap = phases::constant(calc, "server-tick-rate", 1.0 / SERVER_TICK_TIME)?;
    let value = phases::pair(calc, rate, cap, Term::Result, true)?;
    Ok((value, reasons))
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

/// The enemy problems a hit's damage reads: configured inputs of another
/// type and an unknown boss kind (as config reasons). Missing table rows
/// (monster levels, the armour constant, boss types) feed only the
/// enemy's armour and evasion, which a hit with a resistance never reads;
/// they count only with `physical` damage (itself a reason: armour is not
/// modelled).
fn enemy_reasons(enemy: &Enemy, physical: bool, reasons: &mut Reasons) {
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
            EnemyProblem::MissingData { .. } if !physical => {}
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

/// The non-crit and crit hit, each summed over the damage types (module
/// docs, steps 1 to 3).
fn passes(
    calc: &mut Calc<'_>,
    spell: &SpellSkill<'_>,
    enemy: &Enemy,
    data: &dyn CalcData,
    flags: Flags,
    multiplier: Valued,
    reasons: &mut Reasons,
) -> Result<(Valued, Valued), CalcError> {
    let cx = QueryContext::with_flags(flags);
    let source = spell.source();
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
        source.clone(),
        Quantity::Poe1(Poe1Quantity::DamageEffectiveness),
        ratio(spell.level.damage_effectiveness.unwrap_or(0)),
    )?;
    let base_multiplier = leaf(
        calc,
        source,
        Quantity::Poe1(Poe1Quantity::BaseDamageMultiplier),
        ratio(spell.level.base_multiplier),
    )?;
    let mut hits = Vec::new();
    let mut crits = Vec::new();
    let mut physical = false;
    let mut enemy_read = false;
    for t in TYPES {
        let mut range = Vec::new();
        for b in Bound::ALL {
            let skill = sum_in(
                calc,
                Quantity::Poe1(Poe1Quantity::SkillBaseDamage(t, b)),
                Op::Base,
                &cx,
                Term::Step("skill-base-damage"),
                reasons,
            )?;
            let added = sum_in(
                calc,
                Quantity::AddedDamage(t, b),
                Op::Base,
                &cx,
                Term::Step("added-damage"),
                reasons,
            )?;
            // `source x baseMultiplier + added x damageEffectiveness`.
            let from_skill =
                phases::product(calc, &[skill, base_multiplier], Term::Step("from-skill"))?;
            let from_added =
                phases::product(calc, &[added, effectiveness], Term::Step("from-added"))?;
            range.push(phases::total(calc, &[from_skill, from_added], Term::Base)?);
        }
        let [min_base, max_base] = [range[0], range[1]];
        if min_base.value == 0.0 && max_base.value == 0.0 {
            continue;
        }
        let typed = QueryContext::with_flags(flags | Flags::damage(t));
        let (inc, more) = modifiers(calc, Quantity::Damage, &typed, reasons)?;
        let mut rolled = Vec::new();
        for base in [min_base, max_base] {
            let raw = phases::product(calc, &[base, inc, more], Term::Step("unrounded"))?;
            rolled.push(phases::round(calc, raw, Term::Step("damage"))?);
        }
        let [min, max] = [rolled[0], rolled[1]];
        if min.value == 0.0 && max.value == 0.0 {
            continue;
        }
        let effect = match resistance_index(t) {
            Some(index) => {
                enemy_read = true;
                let resist = effective_resistance(calc, enemy, data, t, index, &cx, reasons)?;
                let resist = percent(calc, resist, Term::Step("resistance-share"))?;
                let one = phases::constant(calc, "one", 1.0)?;
                difference(calc, one, resist, Term::Step("enemy-effect"))?
            }
            // Physical: the enemy's armour is not modelled (a reason below);
            // the hit is not reduced.
            None => {
                physical = true;
                phases::constant(calc, "physical-not-reduced", 1.0)?
            }
        };
        let two = phases::constant(calc, "two", 2.0)?;
        for crit in [false, true] {
            let (lo, hi) = if crit {
                (
                    phases::product(calc, &[min, multiplier], Term::Step("crit-min"))?,
                    phases::product(calc, &[max, multiplier], Term::Step("crit-max"))?,
                )
            } else {
                (min, max)
            };
            let lo = quotient(calc, lo, two, Term::Step("half-min"))?;
            let hi = quotient(calc, hi, two, Term::Step("half-max"))?;
            let average = phases::total(calc, &[lo, hi], Term::Step("average"))?;
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
    if physical {
        // The enemy's armour against physical hits is not modelled.
        reasons.partial.insert(UnsupportedReason::UnmodelledSkill {
            source: spell.source(),
            what: "physical-hit-armour",
        });
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
        for t in NOT_SELF_CAST {
            assert!(t.chars().all(|c| c.is_ascii_alphabetic()), "{t}");
        }
    }
}
