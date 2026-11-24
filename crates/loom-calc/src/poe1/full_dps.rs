//! The build's full damage per second (Phase 6 P6-12, part 2): PoB
//! v2.67.2's `FullDPS` (`Modules/Calcs.lua` 176 to 386, `calcFullDPS`).
//!
//! What PoB does: for each active skill of a group marked "Include in Full
//! DPS" (`includeInFullDPS`), the skill is computed as the main skill and
//! its count read (`getActiveSkillCount`, 150 to 174: the group's
//! `groupCount`, else the gem's `count`, 1 by default). Then:
//! - `TotalDPS` (the hit's DPS) times the count, when above 0; for a skill
//!   with a minion, the minion's `TotalDPS` times the count too;
//! - the largest ignite, burning ground (P6-11ga), and bleeding of all
//!   the skills;
//! - poison (`TotalPoisonDPS`) and impale, each times the count, summed;
//! - damage over time (`TotalDot`) summed, times the count only for a skill
//!   whose damage over time stacks (`DotCanStack`);
//! - `TotalDotDPS = min(bleed + ignite + burning ground + poison (itself
//!   at most the cap) + damage over time, DotDpsCap)`; `FullDPS = hits + impale + TotalDotDPS`,
//!   times the largest cull multiplier.
//!
//! The engine computes each such skill with [`crate::calculate`] on a copy
//! of the build with that skill as the main one
//! ([`super::skill::with_main_skill`]), marked
//! [`crate::input::CalcOptions::nested`] so it computes no full DPS of its
//! own. Each field it reads carries its reasons; a skill's hit not computed
//! is a reason (`full-dps-skill`), counted as nothing. Not modelled, each a
//! reason: a skill's mirage (PoB adds the mirage's own output), damage over
//! time of a skill with a count above 1 (`DotCanStack` is not known),
//! a skill's own burning ground and caustic ground, corrupting blood, decay, a Vaal gem's two
//! effects (PoB counts each by its `enableGlobal` flag), and the skills
//! PoB counts once by a config condition (Absolution, Dominating Blow,
//! Holy Strike).

use super::Poe1Field;
use super::combined::DOT_DPS_CAP;
use super::phases;
use crate::engine::Calc;
use crate::error::CalcError;
use crate::graph::Term;
use crate::result::{Field, MinionField};
use crate::source::Source;
use crate::state::{Reasons, UnsupportedReason};

/// One included skill's parts.
#[derive(Default)]
struct Parts {
    hits: f64,
    impale: f64,
    poison: f64,
    dot: f64,
    ignite: f64,
    ground: f64,
    bleed: f64,
    cull: f64,
}

/// The phase (module docs).
///
/// # Errors
/// A graph error.
pub fn phase(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let input = *calc.input();
    if input.options.nested {
        return Ok(());
    }
    let levels = super::gem_levels::GemLevels::from_effects(calc.store().effects());
    let Ok(groups) = super::skill::active_groups(input.build) else {
        return Ok(());
    };
    let mut reasons = Reasons::new();
    let mut parts = Parts {
        cull: 1.0,
        ..Parts::default()
    };
    for (g, a, skill) in super::skill::active_skills(input.build, input.data, &levels) {
        let Some(group) = groups.get(g - 1) else {
            continue;
        };
        if super::skill::attribute(&group.attributes, "includeInFullDPS") != Some("true") {
            continue;
        }
        let source = Source::Gem {
            group: g - 1,
            gem: skill.active_gem.gem,
        };
        let unmodelled = |what| UnsupportedReason::UnmodelledSkill {
            source: source.clone(),
            what,
        };
        // A Vaal gem's two effects: PoB counts each by the gem's
        // `enableGlobal1` and `enableGlobal2` (`getActiveSkillCount`, 159
        // to 164), which the engine does not read yet.
        if skill.types.contains("Vaal") {
            reasons.partial.insert(unmodelled("full-dps-vaal"));
        }
        let count = count(group, &skill);
        let b = super::skill::with_main_skill(input.build, g, a);
        let mut options = input.options.clone();
        options.nested = true;
        let sub = crate::input::CalcInput {
            build: &b,
            data: input.data,
            options: &options,
        };
        let Ok(result) = crate::calculate(&sub) else {
            reasons.partial.insert(unmodelled("full-dps-skill"));
            continue;
        };
        // A field's value with its reasons; 0 when absent or not computed.
        let read = |f: Field, reasons: &mut Reasons| -> f64 {
            match result.fields.get(&f) {
                Some(r) if super::triggers::carry(&r.state, reasons) => r.value.unwrap_or(0.0),
                _ => 0.0,
            }
        };
        let hit_field = result.fields.get(&Field::HitDps);
        let hit_computed = hit_field.is_some_and(|r| r.value.is_some());
        if !hit_computed
            && skill
                .types
                .iter()
                .any(|t| ["Damage", "Attack", "Projectile"].contains(&t.as_str()))
        {
            reasons.partial.insert(unmodelled("full-dps-skill"));
        }
        let hit = read(Field::HitDps, &mut reasons).max(0.0);
        let impale = read(Field::Poe1(Poe1Field::ImpaleDps), &mut reasons);
        let poison = read(Field::PoisonDps, &mut reasons);
        let dot = read(Field::DotDps, &mut reasons);
        let ignite = read(Field::IgniteDps, &mut reasons);
        let ground = read(Field::Poe1(Poe1Field::BurningGroundDps), &mut reasons);
        let bleed = read(Field::BleedDps, &mut reasons);
        let combined = read(Field::Poe1(Poe1Field::CombinedDps), &mut reasons);
        // The minion's (a skill with a minion).
        let m_hit = read(Field::Minion(MinionField::HitDps), &mut reasons);
        let m_impale = read(Field::Minion(MinionField::ImpaleDps), &mut reasons);
        let m_poison = read(Field::Minion(MinionField::PoisonDps), &mut reasons);
        let m_ignite = read(Field::Minion(MinionField::IgniteDps), &mut reasons);
        let m_bleed = read(Field::Minion(MinionField::BleedDps), &mut reasons);
        if dot > 0.0 && count > 1.0 {
            reasons.partial.insert(unmodelled("full-dps-dot-stack"));
        }
        parts.hits += hit * count + m_hit * count;
        parts.impale += impale * count + m_impale * count;
        parts.poison += poison * count + m_poison * count;
        parts.dot += dot;
        parts.ignite = parts.ignite.max(ignite).max(m_ignite);
        parts.ground = parts.ground.max(ground);
        parts.bleed = parts.bleed.max(bleed).max(m_bleed);
        // The skill's cull multiplier, from its combined DPS over its parts
        // (its Culling Strike: 10/9).
        let base = hit + impale + (dot + poison + ignite + ground + bleed).min(DOT_DPS_CAP);
        if base > 0.0 && combined > base * (1.0 + 1e-12) {
            parts.cull = parts.cull.max(combined / base);
        }
    }
    let total_dot =
        (parts.bleed + parts.ignite + parts.ground + parts.poison.min(DOT_DPS_CAP) + parts.dot)
            .min(DOT_DPS_CAP);
    let mut value = parts.hits + parts.impale + total_dot;
    if parts.cull > 1.0 {
        value *= parts.cull;
    }
    let node = phases::constant(calc, "full-dps", value)?;
    let result = phases::total(calc, &[node], Term::Result)?;
    calc.set(
        Field::Poe1(Poe1Field::FullDps),
        phases::query(result, reasons),
    );
    Ok(())
}

/// PoB's `getActiveSkillCount` (`Calcs.lua` 150 to 174): the group's
/// `groupCount`, else the active gem's `count`, else 1.
fn count(group: &loom_domain::SocketGroup, skill: &super::skill::MainSkill<'_>) -> f64 {
    let number = |v: Option<&str>| v.and_then(|s| s.parse::<f64>().ok());
    number(super::skill::attribute(&group.attributes, "groupCount"))
        .or_else(|| {
            group
                .gems
                .typed()
                .nth(skill.active_gem.gem)
                .and_then(|g| number(super::skill::attribute(&g.attributes, "count")))
        })
        .unwrap_or(1.0)
}
