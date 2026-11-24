//! The main skill's combined damage per second (Phase 6 P6-12, part 1):
//! PoB v2.67.2's `CombinedDPS` (`Modules/CalcOffence.lua` 5982 to 6139).
//!
//! What PoB does, for a skill that does not show its average damage:
//! - `CombinedDPS = TotalDPS + ImpaleDPS + TotalDotDPS`, `TotalDotDPS =
//!   min(TotalDot + TotalPoisonDPS + IgniteDPS + BurningGroundDPS +
//!   BleedDPS (+ caustic ground, corrupting blood, decay), DotDpsCap)`
//!   (the ignite's burning ground, P6-11ga), the cap `35791394` (`Data.lua`
//!   255);
//! - times the cull multiplier `100 / (100 - CullPercent)` (3217 to 3221:
//!   Culling Strike's 10, [`Poe1Quantity::CullingStrike`]) and the
//!   reservation multiplier `100 / (100 - LifeReservationPercent)` of the
//!   enemy (1 here: its stats stay reasons).
//!
//! The engine reads the fields the earlier phases computed: the hit's DPS
//! (a dependency: a hitting skill whose hit is not computed leaves this
//! field unsupported; a skill without a hit, Righteous Fire, adds PoB's
//! 0), and each of impale,
//! damage over time, ignite, bleeding, and poison that the build has (each
//! a dependency, so an unsupported one leaves this field unsupported and a
//! partial one makes it partial). Not modelled: ground damage over time
//! and corrupting blood (the damage over time field is then unsupported),
//! decay, critical culling, and skills that show their average damage
//! (their stat stays a reason).
//!
//! **Mirage Archer** (P6-8b; `CalcMirages.lua` 64 to 113, `CalcOffence.lua`
//! 6088 to 6123): a skill Mirage Archer supports (its
//! `skill_can_own_mirage_archers`, PoB's `triggeredByMirageArcher` for a
//! `MirageArcherCanUse` skill) is computed again as the mirage uses it: a
//! copy with the support's less damage and attack speed as `Damage` and
//! `Speed` MORE ([`crate::input::MirageUse`]; the engine runs
//! [`crate::calculate`] with it). Then:
//! - `CombinedDPS` gains the mirage's `TotalDPS` and `ImpaleDPS`, each
//!   times the count (1 per `summon_mirage_archer_on_hit`, PoB's
//!   `MirageArcherMaxCount` BASE 1);
//! - a mirage ignite or bleeding larger than the skill's own sets the
//!   skill's to 0 (and adds the mirage's only to PoB's `MirageDPS`, not to
//!   `CombinedDPS`);
//! - the cull multiplier is the larger of the two, the skill's (the copy
//!   has the skill's Culling Strike);
//! - a mirage poison (PoB adds its single poison's `PoisonDPS`, which the
//!   engine does not compute) or damage over time (added by
//!   `DotCanStack`, not modelled) is a reason.

use super::phases::{self, Valued};
use super::{Poe1Field, Poe1Quantity};
use crate::condition::Flags;
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::{Node, Term};
use crate::quantity::Quantity;
use crate::result::Field;
use crate::state::{Reasons, UnsupportedReason};

/// Whether a skill of `types` hits: the game's `Damage`, `Attack`, or
/// `Projectile` type, as [`super::hit::spell_skill`] reads a spell (P6-12:
/// Righteous Fire has none, so its hit DPS is PoB's 0).
fn has_hit(types: &std::collections::BTreeSet<String>) -> bool {
    ["Damage", "Attack", "Projectile"]
        .iter()
        .any(|t| types.contains(*t))
}

/// PoB's `data.misc.DotDpsCap`: `(2^31 - 1) / 60`.
pub const DOT_DPS_CAP: f64 = 35_791_394.0;

/// Culling Strike's percent (PoB's `CullPercent` MAX 10).
pub const CULL_PERCENT: f64 = 10.0;

/// The culling flags and their percents (P6-12, P6-11ig), largest first:
/// PoB's `CullPercent` is the largest (`Max`).
const CULLS: [(Poe1Quantity, f64); 2] = [
    (Poe1Quantity::CullUnder20, 20.0),
    (Poe1Quantity::CullingStrike, CULL_PERCENT),
];

/// The source of a mirage's less damage and speed (P6-8b).
pub const MIRAGE_RULE: &str = "mirage-archer";
/// Mirage Archer's mark on the skill (PoB's `triggeredByMirageArcher`,
/// `SkillStatMap.lua` 285).
const MIRAGE_STAT: &str = "skill_can_own_mirage_archers";
/// Mirage Archer's less damage (PoB's `MirageArcherLessDamage`).
const MIRAGE_DAMAGE_STAT: &str = "support_mirage_archer_damage_+%_final";
/// Mirage Archer's less attack speed (`MirageArcherLessAttackSpeed`).
const MIRAGE_SPEED_STAT: &str = "support_mirage_archer_attack_speed_+%_final";
/// One mirage per stat (`MirageArcherMaxCount` BASE 1).
const MIRAGE_COUNT_STAT: &str = "summon_mirage_archer_on_hit";

/// The mirage's fields the combined damage per second reads.
pub(super) struct MirageOut {
    hit: f64,
    impale: Option<f64>,
    ignite: Option<f64>,
    bleed: Option<f64>,
    count: f64,
    reasons: Reasons,
}

/// The mirage of the character's main skill, when Mirage Archer supports
/// it (module docs): `Ok(None)` without one, `Err` when its copy's hit is
/// not computed.
fn mirage(
    calc: &Calc<'_>,
    skill: &super::skill::MainSkill<'_>,
) -> Result<Option<MirageOut>, UnsupportedReason> {
    let input = *calc.input();
    if input.options.mirage.is_some() || !skill.types.contains("MirageArcherCanUse") {
        return Ok(None);
    }
    let (mut damage, mut speed, mut count, mut marked) = (0, 0, 0_u32, false);
    for i in super::skill::instances(skill, input.data) {
        for (stat, value) in &i.stats {
            match stat.as_str() {
                MIRAGE_STAT => marked |= *value != 0,
                MIRAGE_DAMAGE_STAT => damage += value,
                MIRAGE_SPEED_STAT => speed += value,
                MIRAGE_COUNT_STAT => count += 1,
                _ => {}
            }
        }
    }
    if !marked {
        return Ok(None);
    }
    let source = crate::source::Source::Rule(MIRAGE_RULE);
    let unsupported = |what| UnsupportedReason::UnmodelledSkill {
        source: source.clone(),
        what,
    };
    let mut options = input.options.clone();
    options.mirage = Some(crate::input::MirageUse {
        damage_more: damage,
        speed_more: speed,
    });
    options.nested = true;
    let sub = crate::input::CalcInput {
        build: input.build,
        data: input.data,
        options: &options,
    };
    let result = crate::calculate(&sub).map_err(|_| unsupported("mirage"))?;
    let mut reasons = Reasons::new();
    let mut read = |f: Field| -> Result<Option<f64>, UnsupportedReason> {
        let Some(r) = result.fields.get(&f) else {
            return Ok(None);
        };
        if !super::triggers::carry(&r.state, &mut reasons) {
            return Err(unsupported("mirage"));
        }
        Ok(r.value)
    };
    let hit = read(Field::HitDps)?.ok_or_else(|| unsupported("mirage"))?;
    let impale = read(Field::Poe1(Poe1Field::ImpaleDps))?;
    let ignite = read(Field::IgniteDps)?;
    let bleed = read(Field::BleedDps)?;
    let poison = read(Field::PoisonDps)?.is_some_and(|v| v != 0.0);
    let dot = read(Field::DotDps)?.is_some_and(|v| v != 0.0);
    if poison {
        reasons.partial.insert(unsupported("mirage-poison"));
    }
    if dot {
        reasons
            .partial
            .insert(unsupported("mirage-damage-over-time"));
    }
    Ok(Some(MirageOut {
        hit,
        impale,
        ignite,
        bleed,
        count: f64::from(count.max(1)),
        reasons,
    }))
}

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
    match mirage(calc, &skill) {
        Ok(m) => combine(calc, &skill, m.as_ref()),
        Err(reason) => {
            calc.set_unsupported(Field::Poe1(Poe1Field::CombinedDps), reason);
            Ok(())
        }
    }
}

/// Whether PoB's `showAverage` holds for a trap's or a mine's skill
/// (P6-11hp): its `base_skill_show_average_damage_instead_of_dps` (the
/// marker [`Poe1Quantity::HitReason`]) and a cooldown.
fn shows_average(calc: &Calc<'_>, skill: &super::skill::MainSkill<'_>) -> bool {
    let trap_or_mine = [super::skill::TRAP_TYPE, super::skill::MINE_TYPE]
        .iter()
        .any(|t| skill.types.contains(*t));
    trap_or_mine
        && super::skill::effect_level(skill.effect(), skill.active_gem.level)
            .is_some_and(|l| l.cooldown != 0)
        && phases::has(
            calc,
            Quantity::Poe1(Poe1Quantity::HitReason(SHOW_AVERAGE_STAT)),
        )
}

/// PoB's `showAverage` stat (`SkillStatMap.lua` 136).
const SHOW_AVERAGE_STAT: &str = "base_skill_show_average_damage_instead_of_dps";

/// The combined damage per second of `skill` from the fields the actor
/// computed (the character's main skill, or the minion's under the minion
/// actor, P6-7c3d: every field is then the minion's copy).
///
/// # Errors
/// A graph error.
pub(super) fn combine(
    calc: &mut Calc<'_>,
    skill: &super::skill::MainSkill<'_>,
    mirage: Option<&MirageOut>,
) -> Result<(), CalcError> {
    let field = Field::Poe1(Poe1Field::CombinedDps);
    let value_of = |calc: &Calc<'_>, f: Field| {
        calc.field(f).and_then(|r| {
            Some(Valued {
                node: r.node?,
                value: r.value?,
            })
        })
    };
    let mut deps = Vec::new();
    let mut terms = Vec::new();
    // PoB's `showAverage` (P6-11hp; `CalcOffence.lua` 333, 5983): the base is
    // `AverageDamage`, not `TotalDPS`. A trap's or a mine's keeps it only
    // with a cooldown (1218 to 1222; Bear Trap). Elsewhere the stat is the
    // rate's reason (`hit::rate`), which a trap's or a mine's rate does not
    // read.
    let show_average = shows_average(calc, skill);
    if show_average && let Some(avg) = value_of(calc, Field::AverageDamage) {
        deps.push(Field::AverageDamage);
        terms.push(avg);
    } else if let Some(hit) = value_of(calc, Field::HitDps) {
        deps.push(Field::HitDps);
        terms.push(hit);
    } else if has_hit(&skill.types) {
        calc.set_unsupported(
            field,
            UnsupportedReason::DependsOnNotComputed {
                field: Field::HitDps,
            },
        );
        return Ok(());
    } else {
        // A skill without a hit (Righteous Fire): PoB's `TotalDPS` is 0.
        terms.push(phases::constant(calc, "no-hit", 0.0)?);
    }
    if calc.field(Field::Poe1(Poe1Field::ImpaleDps)).is_some() {
        deps.push(Field::Poe1(Poe1Field::ImpaleDps));
        if let Some(v) = value_of(calc, Field::Poe1(Poe1Field::ImpaleDps)) {
            terms.push(v);
        }
    }
    let mut reasons = Reasons::new();
    // The mirage's hit and impale, times the count (6089 to 6111).
    if let Some(m) = mirage {
        reasons.extend(&m.reasons);
        let count = phases::constant(calc, "mirage-count", m.count)?;
        for (name, v) in [
            ("mirage-hit-dps", Some(m.hit)),
            ("mirage-impale-dps", m.impale),
        ] {
            if let Some(v) = v {
                let v = phases::constant(calc, name, v)?;
                terms.push(phases::product(calc, &[v, count], Term::Step(name))?);
            }
        }
    }
    let mut dots = Vec::new();
    for f in [
        Field::DotDps,
        Field::PoisonDps,
        Field::IgniteDps,
        Field::Poe1(Poe1Field::BurningGroundDps),
        Field::BleedDps,
    ] {
        if calc.field(f).is_some() {
            deps.push(f);
            // A mirage ignite or bleeding larger than the skill's own sets
            // the skill's to 0 (6096 to 6103).
            let mirage_value = mirage.and_then(|m| match f {
                Field::IgniteDps => m.ignite,
                Field::BleedDps => m.bleed,
                _ => None,
            });
            if let Some(v) = value_of(calc, f) {
                if mirage_value.is_some_and(|mv| mv > v.value) {
                    let zero = phases::constant(calc, "mirage-replaces", 0.0)?;
                    let state = calc
                        .field(f)
                        .map(|r| r.state.clone())
                        .unwrap_or(crate::state::FieldState::Complete);
                    let mut field_reasons = Reasons::new();
                    super::triggers::carry(&state, &mut field_reasons);
                    calc.set(f, phases::query(zero, field_reasons));
                    continue;
                }
                dots.push(v);
            }
        }
    }
    if terms.is_empty() && dots.is_empty() {
        calc.set_unsupported(field, UnsupportedReason::NotComputed);
        return Ok(());
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
    // Culling Strike in the skill's context: an attack's with its main
    // hand's flags (P6-11ij: "Bow Attacks have Culling Strike", PoB's
    // `skillCfg` with `weapon1Flags`, `CalcActiveSkill.lua` 455). A hand the
    // engine cannot read gives no flags; the hit and the DoT carry its
    // reason, so the one here is dropped.
    let mut flags = skill.keyword_flags();
    if skill.types.contains("Attack") {
        let mut dropped = Reasons::default();
        flags = flags
            | Flags::ATTACK
            | super::dot::attack_weapon_flags(
                calc.input(),
                skill,
                &crate::source::Source::Rule("culling-strike"),
                &mut dropped,
            );
    }
    if skill.types.contains("Spell") {
        flags = flags | Flags::SPELL;
    }
    let cx = calc.context(flags);
    for (quantity, percent) in CULLS {
        let (store, graph) = calc.parts();
        let cull = store.flag(graph, Quantity::Poe1(quantity), &cx)?;
        reasons.extend(&cull.reasons);
        if cull.value > 0.0 {
            let value = 100.0 / (100.0 - percent);
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
            break;
        }
    }
    let result = phases::total(calc, &[combined], Term::Result)?;
    calc.set_derived(field, phases::query(result, reasons), &deps);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skill_hits_with_a_damage_attack_or_projectile_type() {
        let types = |names: &[&str]| names.iter().map(|s| (*s).to_owned()).collect();
        assert!(has_hit(&types(&["Spell", "Damage", "Area"])));
        assert!(has_hit(&types(&["Attack", "DamageOverTime"])));
        assert!(!has_hit(&types(&[
            "Spell",
            "Buff",
            "DamageOverTime",
            "CausesBurning"
        ])));
    }
}
