//! The main skill of a build, found the way PoB v2.67.2 finds it, from the
//! bundle's skill tables (Phase 3 plan, step 3).
//!
//! The rule is the engine's (`loom_calc::poe1::skill`, moved there in
//! P3-9a from this module, unchanged): the survey and the calculation pick
//! the same skill. This module keeps the survey's names for it and the
//! gem lookup over its indexed tables, which the inventory uses.

use loom_data::bundle::skills::GrantedEffect;
use loom_domain::{Build, Gem};

use super::Tables;

pub use loom_calc::poe1::skill::{MainSkill, SkillProblem as Problem, active_groups, attribute};

/// A gem's granted effects in PoB's order (the gem effect's own, then its
/// additional ones), or `None` when the bundle does not have the gem
/// (`loom_calc::poe1::skill::gem_effects` over the survey's indexes).
pub fn gem_effects<'b>(gem: &Gem, t: &Tables<'b>) -> Option<Vec<&'b GrantedEffect>> {
    let skill = gem.skill.as_ref().map(loom_domain::SkillId::as_str);
    let Some(gem_id) = &gem.gem else {
        return Some(vec![*t.granted.get(skill?)?]);
    };
    let row = t.gems.get(gem_id.as_str())?;
    let variant = attribute(&gem.attributes, "variantId");
    let effect = row
        .effects
        .iter()
        .filter_map(|e| t.gem_effects.get(e.as_str()))
        .find(|e| match variant {
            Some(v) => e.id == v,
            None => e.granted_effect.as_deref() == skill,
        })?;
    let mut out = Vec::new();
    for id in effect
        .granted_effect
        .iter()
        .chain(&effect.additional_granted_effects)
    {
        out.push(*t.granted.get(id.as_str())?);
    }
    Some(out)
}

/// The build's main skill (the engine's rule over the survey's bundle).
/// Gem levels change no pick, so none are read: the gems keep the levels
/// the build writes.
///
/// # Errors
/// A [`Problem`] when the build has no main skill the survey can name.
pub fn main_skill<'b>(build: &Build, t: &Tables<'b>) -> Result<MainSkill<'b>, Problem> {
    loom_calc::poe1::skill::main_skill(
        build,
        t.bundle,
        &loom_calc::poe1::gem_levels::GemLevels::none(),
    )
}
