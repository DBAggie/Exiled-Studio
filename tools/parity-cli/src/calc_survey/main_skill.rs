//! The main skill of a build, found the way PoB v2.67.2 finds it, from the
//! bundle's skill tables (Phase 3 plan, step 3).
//!
//! PoB's rule (`Modules/CalcSetup.lua`, "Determine main skill group" and
//! "Create active skills"; `Classes/SkillsTab.lua`, gem loading):
//! 1. the socket groups are those of the active skill set
//!    (`Skills.activeSkillSet`);
//! 2. the main group is `Build.mainSocketGroup`, clamped to the groups
//!    (`m_min(m_max(#groups, 1), mainSocketGroup or 1)`);
//! 3. its active skills are, for each enabled gem in order, each granted
//!    effect of the gem (the gem effect's own, then its additional ones)
//!    that is not a support, skipping one PoB marks as having a global
//!    effect while the gem's `enableGlobal<n>` is off;
//! 4. the main skill is `mainActiveSkill` of that list, clamped.
//!
//! A gem is found by its base item (`gemId`) and its gem effect
//! (`variantId`, or the gem effect whose granted effect is `skillId`); a
//! gem without `gemId` is a granted effect by `skillId` (a skill an item
//! grants).
//!
//! PoB's "global effect" mark comes from its own stat map (a `GlobalEffect`
//! tag), not from game data. The survey does not model it: an effect whose
//! `enableGlobal<n>` is off may or may not be counted, so the survey picks
//! the main skill for every combination of such effects counted or not,
//! and the main skill is [`Problem::Uncertain`] unless all pick the same
//! effect.
//!
//! Supports apply as `calcLib.canGrantedEffectSupportActiveSkill` decides,
//! in group order, each adding its types once it applies
//! (`Modules/CalcActiveSkill.lua`, "Pass 1"; its "loop over rejected
//! supports" reconsiders only entries that are not supports, because pass 1
//! queues only those, so each support is decided once, in order): a
//! support applies when the active effect can be supported, a
//! gems-only support supports a skill from a gem (not one an item
//! grants), the support's excluded types do not match, and its allowed
//! types match (or are empty). The type lists are
//! postfix expressions over `AND`, `OR`, and `NOT`
//! (`calcLib.doesTypeExpressionMatch`). Not modelled: supports' weapon
//! types, supports items add (`ExtraSupport`), supports from other groups
//! of the same slot for item-granted skills, and cross-linked groups; the
//! inventory lists item stats that add supports, so a reviewer sees them.

use std::collections::BTreeSet;

use loom_data::bundle::skills::{ActiveSkill, GrantedEffect};
use loom_domain::{Attributes, Build, Gem, SocketGroup};
use serde::Serialize;

use super::Tables;

/// Why a build has no main skill the survey can name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "problem")]
pub enum Problem {
    /// The build has no typed skill set (old format or none).
    NoSkillSet,
    /// The active skill set holds untyped content where a group could be,
    /// so the main group's index is not certain.
    UntypedGroup,
    /// The main group has no active skill.
    NoActiveSkill,
    /// A gem of the main group is not in the bundle.
    UnknownGem {
        /// The gem's `gemId`, or `skillId` without one.
        gem: String,
    },
    /// An effect PoB might count was skipped before the chosen index
    /// (module docs).
    Uncertain,
}

impl Problem {
    /// Stable name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::NoSkillSet => "no-skill-set",
            Self::UntypedGroup => "untyped-group",
            Self::NoActiveSkill => "no-active-skill",
            Self::UnknownGem { .. } => "unknown-gem",
            Self::Uncertain => "main-skill-uncertain",
        }
    }
}

/// The main skill.
#[derive(Debug, Clone)]
pub struct MainSkill<'b> {
    /// The main group's position in the active set (1-based).
    pub group: usize,
    /// The granted effect.
    pub effect: &'b GrantedEffect,
    /// Its active skill row, if it names one.
    pub active: Option<&'b ActiveSkill>,
    /// Whether the group is a skill an item grants (PoB's `source`).
    pub item_granted: bool,
    /// Supports that apply, in group order.
    pub supports: Vec<&'b GrantedEffect>,
    /// Supports in the group that do not apply.
    pub unapplied: Vec<&'b GrantedEffect>,
    /// The skill's types after the supports added theirs.
    pub types: BTreeSet<String>,
}

/// The value of attribute `name`, if any.
pub fn attribute<'a>(attributes: &'a Attributes, name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.value.as_str())
}

/// The active skill set's typed groups, or why there are none.
pub fn active_groups(build: &Build) -> Result<Vec<&SocketGroup>, Problem> {
    let skills = &build.skills;
    let mut sets = skills.sets.typed();
    let first = skills.sets.typed().next().ok_or(Problem::NoSkillSet)?;
    // PoB: `activeSkillSet` (default 1); a set it does not find falls back
    // to the first one (`SkillsTab:SetActiveSkillSet`).
    let set = skills
        .active
        .and_then(|id| sets.find(|s| s.id == id))
        .unwrap_or(first);
    if set.groups.opaque().next().is_some() {
        return Err(Problem::UntypedGroup);
    }
    Ok(set.groups.typed().collect())
}

/// A gem's granted effects in PoB's order (the gem effect's own, then its
/// additional ones), or `None` when the bundle does not have the gem.
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

/// Whether PoB counts the gem's `n`-th effect (1-based) as enabled for a
/// global effect: `enableGlobal1` defaults to on, the others to off.
fn global_enabled(gem: &Gem, n: usize) -> bool {
    match attribute(&gem.attributes, &format!("enableGlobal{n}")) {
        Some(v) => v == "true",
        None => n == 1,
    }
}

/// `calcLib.doesTypeExpressionMatch`: a postfix expression over the
/// skill's types; true when any value left on the stack is true. An empty
/// expression is false.
#[must_use]
pub fn type_expression(expr: &[String], types: &BTreeSet<String>, minion: &[String]) -> bool {
    let mut stack: Vec<bool> = Vec::new();
    for t in expr {
        match t.as_str() {
            "OR" | "AND" => {
                let other = stack.pop().unwrap_or(false);
                if let Some(top) = stack.last_mut() {
                    *top = if t == "OR" {
                        *top || other
                    } else {
                        *top && other
                    };
                }
            }
            "NOT" => {
                if let Some(top) = stack.last_mut() {
                    *top = !*top;
                }
            }
            _ => stack.push(types.contains(t) || minion.iter().any(|m| m == t)),
        }
    }
    stack.into_iter().any(|v| v)
}

/// Whether `support` applies to an active effect with these types (module
/// docs).
fn supports(
    support: &GrantedEffect,
    active: &GrantedEffect,
    from_gem: bool,
    types: &BTreeSet<String>,
    minion: &[String],
) -> bool {
    if active.cannot_be_supported || (support.supports_gems_only && !from_gem) {
        return false;
    }
    if !support.excluded_types.is_empty() && type_expression(&support.excluded_types, types, &[]) {
        return false;
    }
    support.allowed_types.is_empty() || type_expression(&support.allowed_types, types, minion)
}

/// The most optional effects [`choose`] tries every combination of.
const MAX_OPTIONAL: usize = 8;

/// PoB's `mainActiveSkill` pick (clamped) over the active effects, for
/// every combination of the optional ones counted or not; the effect when
/// all combinations pick the same one (module docs).
fn choose<'b>(
    actives: &[(&'b GrantedEffect, bool)],
    wanted: usize,
) -> Result<&'b GrantedEffect, Problem> {
    let optional = actives.iter().filter(|(_, o)| *o).count();
    // Beyond the cap the combinations are not tried: uncertain.
    if optional > MAX_OPTIONAL {
        return Err(Problem::Uncertain);
    }
    // Each combination's pick; `None` is no active skill (PoB then has no
    // main skill).
    let mut picks: Vec<Option<&'b GrantedEffect>> = Vec::new();
    for mask in 0u32..(1 << optional) {
        let mut bit = 0;
        let list: Vec<&GrantedEffect> = actives
            .iter()
            .filter(|(_, o)| {
                if !*o {
                    return true;
                }
                bit += 1;
                mask & (1 << (bit - 1)) != 0
            })
            .map(|(e, _)| *e)
            .collect();
        picks.push(list.get(wanted.clamp(1, list.len().max(1)) - 1).copied());
    }
    let first = picks.first().copied().flatten();
    if picks
        .iter()
        .any(|p| p.map(|e| &e.id) != first.map(|e| &e.id))
    {
        return Err(Problem::Uncertain);
    }
    first.ok_or(Problem::NoActiveSkill)
}

/// The build's main skill (module docs).
pub fn main_skill<'b>(build: &Build, t: &Tables<'b>) -> Result<MainSkill<'b>, Problem> {
    let groups = active_groups(build)?;
    let count = groups.len().max(1);
    let wanted = build
        .character
        .main_socket_group
        .as_ref()
        .map_or(1, |s| *s.value() as usize);
    let index = wanted.clamp(1, count);
    let group = *groups.get(index - 1).ok_or(Problem::NoActiveSkill)?;

    // Active effects in order; `true` marks one PoB counts only when it
    // has no global effect (module docs).
    let mut actives: Vec<(&'b GrantedEffect, bool)> = Vec::new();
    let mut group_supports: Vec<&'b GrantedEffect> = Vec::new();
    let mut gem_sourced: BTreeSet<&'b str> = BTreeSet::new();
    for gem in group.gems.typed().filter(|g| g.enabled) {
        let effects = gem_effects(gem, t).ok_or_else(|| Problem::UnknownGem {
            gem: gem
                .gem
                .as_ref()
                .map(|g| g.as_str().to_owned())
                .or_else(|| gem.skill.as_ref().map(|s| s.as_str().to_owned()))
                .unwrap_or_default(),
        })?;
        for (n, effect) in effects.into_iter().enumerate() {
            if effect.support {
                group_supports.push(effect);
            } else {
                if gem.gem.is_some() {
                    gem_sourced.insert(effect.id.as_str());
                }
                actives.push((effect, !global_enabled(gem, n + 1)));
            }
        }
    }
    let wanted = group
        .main_active_skill
        .as_ref()
        .map_or(1, |s| *s.value() as usize);
    let effect = choose(&actives, wanted)?;
    // PoB's `activeEffect.gemData`: the skill comes from a gem, not from an
    // item (a gem entry without `gemId`).
    let from_gem = gem_sourced.contains(effect.id.as_str());
    let active = effect
        .active_skill
        .as_deref()
        .and_then(|a| t.active.get(a).copied());
    let minion: &[String] = active.map_or(&[], |a| a.minion_types.as_slice());
    let mut types: BTreeSet<String> = active
        .map(|a| a.types.iter().cloned().collect())
        .unwrap_or_default();
    let (mut applied, mut unapplied) = (Vec::new(), Vec::new());
    for s in group_supports {
        if supports(s, effect, from_gem, &types, minion) {
            types.extend(s.added_types.iter().cloned());
            applied.push(s);
        } else {
            unapplied.push(s);
        }
    }
    Ok(MainSkill {
        group: index,
        effect,
        active,
        item_granted: attribute(&group.attributes, "source").is_some(),
        supports: applied,
        unapplied,
        types,
    })
}
