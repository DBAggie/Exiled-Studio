//! The skill model (Phase 3 plan, step 9a): the build's main skill as PoB
//! v2.67.2 finds it, its supports, and each gem instance's stats at its
//! level and quality, from the game data's skill tables.
//!
//! **Main skill** (`Modules/CalcSetup.lua`, "Determine main skill group"
//! and "Create active skills"; `Classes/SkillsTab.lua`, gem loading):
//! 1. the socket groups are those of the active skill set
//!    (`Skills.activeSkillSet`, else the first set);
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
//! tag), not from game data, so it is not modelled: an effect whose
//! `enableGlobal<n>` is off may or may not be counted, the main skill is
//! picked for every combination of such effects counted or not, and it is
//! [`SkillProblem::Uncertain`] unless all pick the same effect.
//!
//! Supports apply as `calcLib.canGrantedEffectSupportActiveSkill` decides,
//! in group order, each adding its types once it applies
//! (`Modules/CalcActiveSkill.lua`, "Pass 1"; its "loop over rejected
//! supports" reconsiders only entries that are not supports, so each
//! support is decided once, in order): a support applies when the active
//! effect can be supported, a gems-only support supports a skill from a gem
//! (not one an item grants), the support's excluded types do not match,
//! and its allowed types match (or are empty). Type lists are postfix
//! expressions over `AND`, `OR`, and `NOT` (`calcLib.doesTypeExpressionMatch`).
//! Not modelled: supports' weapon types, supports items add
//! (`ExtraSupport`), supports from other groups of the same slot for
//! item-granted skills, and cross-linked groups.
//!
//! This is the rule `calc-survey` (P3-3) used; the survey now calls it.
//!
//! **A gem instance's stats** (`Modules/CalcTools.lua`,
//! `calcLib.validateGemLevel` and `calcLib.buildSkillInstanceStats`): the
//! level is the gem's, kept when the effect has that level, else clamped
//! to `1..=#levels`; the stats are the quality stats (`modf(per 1% x
//! quality)`), the level's integer stats, and the stat set's constant
//! stats, summed per stat, with the stat set's implicit stats as 1 each
//! (PoB's `level[index] or 1`); only the first stat set counts, as PoB's
//! exporter writes only it. Effectiveness-scaled float stats (base damage,
//! `statInterpolation` 3) are the game's resolved values
//! (`BaseResolvedValues`, P3-9b): PoB computes them from the level
//! requirement, and they equal PoB's on 12,016 of 12,144 rows, every Ice
//! Nova level among them (`docs/phase-3/spell-hits.md`); linear
//! interpolation (code 2) is not modelled and is a reason.

use std::collections::{BTreeMap, BTreeSet};

use loom_domain::{Attributes, Build, Gem, SocketGroup};

use super::sockets::{SocketMatch, socket_matches};
use crate::condition::Flags;
use crate::input::{ActiveSkill, CalcData, EffectLevel, GrantedEffect, StatSet, StatSetLevel};
use crate::quantity::DamageType;

/// Why a build has no main skill the engine can name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillProblem {
    /// The build has no typed skill set (old format or none).
    NoSkillSet,
    /// The active skill set holds untyped content where a group could be,
    /// so the main group's index is not certain.
    UntypedGroup,
    /// The main group has no active skill.
    NoActiveSkill,
    /// A gem of the main group is not in the data (by `gemId`, else
    /// `skillId`).
    UnknownGem {
        /// The gem.
        gem: String,
    },
    /// An effect PoB might count was skipped before the chosen index
    /// (module docs).
    Uncertain,
}

impl SkillProblem {
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

/// One granted effect of a gem in the main group, with the gem's level
/// and quality as the build writes them.
#[derive(Debug, Clone, Copy)]
pub struct GemEffectRef<'d> {
    /// The gem's position among the group's typed gems (0-based).
    pub gem: usize,
    /// The granted effect.
    pub effect: &'d GrantedEffect,
    /// The gem's level.
    pub level: i64,
    /// The instance's quality: the gem's, plus
    /// [`super::sockets::BONUS`] when it sits in a socket of its colour
    /// (PoB's `actualQuality`, P3-12i).
    pub quality: i64,
    /// Whether the gem gains the socket colour bonus.
    pub socket: SocketMatch,
}

/// The main skill.
#[derive(Debug, Clone)]
pub struct MainSkill<'d> {
    /// The main group's position in the active set (1-based).
    pub group: usize,
    /// The active granted effect, with its gem.
    pub active_gem: GemEffectRef<'d>,
    /// Its active skill row, if it names one.
    pub active: Option<&'d ActiveSkill>,
    /// Whether the group is a skill an item grants (PoB's `source`).
    pub item_granted: bool,
    /// Supports that apply, in group order, with their gems.
    pub supports: Vec<GemEffectRef<'d>>,
    /// Supports in the group that do not apply.
    pub unapplied: Vec<GemEffectRef<'d>>,
    /// The skill's types after the supports added theirs.
    pub types: BTreeSet<String>,
}

impl<'d> MainSkill<'d> {
    /// The active granted effect.
    #[must_use]
    pub const fn effect(&self) -> &'d GrantedEffect {
        self.active_gem.effect
    }

    /// The skill's keyword flags (P3-8h): PoB v2.67.2 sets
    /// `KeywordFlag.Lightning`, `.Cold`, `.Fire`, `.Chaos`, `.Physical`
    /// from the skill's types after the supports added theirs
    /// (`Modules/CalcActiveSkill.lua` 391 to 405), whatever damage the hit
    /// deals; "with Cold Skills" needs the keyword.
    #[must_use]
    pub fn keyword_flags(&self) -> Flags {
        KEYWORD_TYPES
            .iter()
            .filter(|(name, _)| self.types.contains(*name))
            .fold(Flags::NONE, |flags, (_, t)| flags | Flags::skill(*t))
    }
}

/// The game's skill type names that give a skill keyword (the game's
/// `ActiveSkillType` IDs, which PoB's `SkillType` names match).
pub const KEYWORD_TYPES: [(&str, DamageType); 5] = [
    ("Physical", DamageType::Physical),
    ("Fire", DamageType::Fire),
    ("Cold", DamageType::Cold),
    ("Lightning", DamageType::Lightning),
    ("Chaos", DamageType::Chaos),
];

/// The value of attribute `name`, if any.
#[must_use]
pub fn attribute<'a>(attributes: &'a Attributes, name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.value.as_str())
}

/// The active skill set's typed groups, or why there are none.
///
/// # Errors
/// [`SkillProblem::NoSkillSet`], [`SkillProblem::UntypedGroup`].
pub fn active_groups(build: &Build) -> Result<Vec<&SocketGroup>, SkillProblem> {
    let skills = &build.skills;
    let first = skills.sets.typed().next().ok_or(SkillProblem::NoSkillSet)?;
    // PoB: `activeSkillSet` (default 1); a set it does not find falls back
    // to the first one (`SkillsTab:SetActiveSkillSet`).
    let set = skills
        .active
        .and_then(|id| skills.sets.typed().find(|s| s.id == id))
        .unwrap_or(first);
    if set.groups.opaque().next().is_some() {
        return Err(SkillProblem::UntypedGroup);
    }
    Ok(set.groups.typed().collect())
}

/// A gem's granted effects in PoB's order (the gem effect's own, then its
/// additional ones), or `None` when the data does not have the gem.
#[must_use]
pub fn gem_effects<'d>(gem: &Gem, data: &'d dyn CalcData) -> Option<Vec<&'d GrantedEffect>> {
    let skill = gem.skill.as_ref().map(loom_domain::SkillId::as_str);
    let Some(gem_id) = &gem.gem else {
        return Some(vec![data.granted_effect(skill?)?]);
    };
    let row = data.skill_gem(gem_id.as_str())?;
    let variant = attribute(&gem.attributes, "variantId");
    let effect = row
        .effects
        .iter()
        .filter_map(|e| data.gem_effect(e.as_str()))
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
        out.push(data.granted_effect(id.as_str())?);
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
/// expression is false; an operator without operands leaves the stack as
/// PoB's `nil` does (a malformed expression is false, as in PoB).
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

/// One active effect: the effect with its gem, whether it came from a gem
/// (`gemId`), and whether PoB counts it only without a global effect.
type Candidate<'d> = (GemEffectRef<'d>, bool, bool);

/// PoB's `mainActiveSkill` pick (clamped) over the active effects, for
/// every combination of the optional ones counted or not; the effect when
/// all combinations pick the same one (module docs).
fn choose<'d>(actives: &[Candidate<'d>], wanted: usize) -> Result<Candidate<'d>, SkillProblem> {
    let optional = actives.iter().filter(|(_, _, o)| *o).count();
    // Beyond the cap the combinations are not tried: uncertain.
    if optional > MAX_OPTIONAL {
        return Err(SkillProblem::Uncertain);
    }
    // Each combination's pick; `None` is no active skill (PoB then has no
    // main skill).
    let mut picks: Vec<Option<Candidate<'d>>> = Vec::new();
    for mask in 0u32..(1 << optional) {
        let mut bit = 0;
        let list: Vec<Candidate<'d>> = actives
            .iter()
            .filter(|(_, _, o)| {
                if !*o {
                    return true;
                }
                bit += 1;
                mask & (1 << (bit - 1)) != 0
            })
            .copied()
            .collect();
        picks.push(list.get(wanted.clamp(1, list.len().max(1)) - 1).copied());
    }
    let id = |p: &Option<Candidate<'d>>| p.map(|(e, _, _)| e.effect.id.as_str());
    let first = picks.first().copied().flatten();
    if picks.iter().any(|p| id(p) != id(&first)) {
        return Err(SkillProblem::Uncertain);
    }
    first.ok_or(SkillProblem::NoActiveSkill)
}

/// The build's main skill (module docs).
///
/// # Errors
/// A [`SkillProblem`] when the build has no main skill the engine can name.
pub fn main_skill<'d>(
    build: &Build,
    data: &'d dyn CalcData,
) -> Result<MainSkill<'d>, SkillProblem> {
    let groups = active_groups(build)?;
    let count = groups.len().max(1);
    let wanted = build
        .character
        .main_socket_group
        .as_ref()
        .map_or(1, |s| *s.value() as usize);
    let index = wanted.clamp(1, count);
    let group = *groups.get(index - 1).ok_or(SkillProblem::NoActiveSkill)?;
    let all = socket_matches(build, &groups, data);
    let matches: &[SocketMatch] = all.get(index - 1).map_or(&[], Vec::as_slice);

    let mut actives: Vec<Candidate<'d>> = Vec::new();
    let mut group_supports: Vec<GemEffectRef<'d>> = Vec::new();
    for (g, gem) in group.gems.typed().enumerate() {
        if !gem.enabled {
            continue;
        }
        let effects = gem_effects(gem, data).ok_or_else(|| SkillProblem::UnknownGem {
            gem: gem
                .gem
                .as_ref()
                .map(|g| g.as_str().to_owned())
                .or_else(|| gem.skill.as_ref().map(|s| s.as_str().to_owned()))
                .unwrap_or_default(),
        })?;
        for (n, effect) in effects.into_iter().enumerate() {
            let socket = matches.get(g).copied().unwrap_or(SocketMatch::No);
            let r = GemEffectRef {
                gem: g,
                effect,
                level: i64::from(*gem.level.value()),
                quality: i64::from(*gem.quality.value()) + socket.bonus(),
                socket,
            };
            if effect.support {
                group_supports.push(r);
            } else {
                actives.push((r, gem.gem.is_some(), !global_enabled(gem, n + 1)));
            }
        }
    }
    let wanted = group
        .main_active_skill
        .as_ref()
        .map_or(1, |s| *s.value() as usize);
    // PoB's `activeEffect.gemData`: the skill comes from a gem, not from an
    // item (a gem entry without `gemId`). An effect both a gem and an item
    // entry grant counts as from a gem, as in PoB's per-id lookup.
    let (active_gem, _, _) = choose(&actives, wanted)?;
    let effect = active_gem.effect;
    let from_gem = actives
        .iter()
        .any(|(r, gem, _)| *gem && r.effect.id == effect.id);
    let active = effect
        .active_skill
        .as_deref()
        .and_then(|a| data.active_skill(a));
    let minion: &[String] = active.map_or(&[], |a| a.minion_types.as_slice());
    let mut types: BTreeSet<String> = active
        .map(|a| a.types.iter().cloned().collect())
        .unwrap_or_default();
    let (mut applied, mut unapplied) = (Vec::new(), Vec::new());
    for s in group_supports {
        if supports(s.effect, effect, from_gem, &types, minion) {
            types.extend(s.effect.added_types.iter().cloned());
            applied.push(s);
        } else {
            unapplied.push(s);
        }
    }
    Ok(MainSkill {
        group: index,
        active_gem,
        active,
        item_granted: attribute(&group.attributes, "source").is_some(),
        supports: applied,
        unapplied,
        types,
    })
}

/// One active effect of a socket group with the supports that apply to it
/// (P3-12h: every group's skills, not only the main one's).
#[derive(Debug, Clone)]
pub struct GroupSkill<'d> {
    /// The active granted effect, with its gem.
    pub active_gem: GemEffectRef<'d>,
    /// Its active skill row, if it names one.
    pub active: Option<&'d ActiveSkill>,
    /// Whether PoB creates the skill only when the gem's `enableGlobal<n>`
    /// is on and the effect has a global effect (`CalcSetup.lua` 1652):
    /// `true` when the flag is off, so the skill exists only if the effect
    /// has none.
    pub global_off: bool,
    /// Supports that apply, in group order, with their gems.
    pub supports: Vec<GemEffectRef<'d>>,
    /// The skill's types after the supports added theirs.
    pub types: BTreeSet<String>,
}

/// The active effects of a group's enabled gems, each with the group's
/// supports that apply to it, in PoB's order (`CalcSetup.lua` 1645 to
/// 1740: gems in order, each gem's effects in order; supports as for the
/// main skill, module docs). `matches` are the group's gems' socket
/// matches ([`socket_matches`]), each adding its bonus to the gem's
/// quality. `Err` names a gem the data does not have.
///
/// # Errors
/// [`SkillProblem::UnknownGem`].
pub fn group_skills<'d>(
    group: &SocketGroup,
    matches: &[SocketMatch],
    data: &'d dyn CalcData,
) -> Result<Vec<GroupSkill<'d>>, SkillProblem> {
    let mut actives: Vec<Candidate<'d>> = Vec::new();
    let mut group_supports: Vec<GemEffectRef<'d>> = Vec::new();
    for (g, gem) in group.gems.typed().enumerate() {
        if !gem.enabled {
            continue;
        }
        let effects = gem_effects(gem, data).ok_or_else(|| SkillProblem::UnknownGem {
            gem: gem
                .gem
                .as_ref()
                .map(|g| g.as_str().to_owned())
                .or_else(|| gem.skill.as_ref().map(|s| s.as_str().to_owned()))
                .unwrap_or_default(),
        })?;
        for (n, effect) in effects.into_iter().enumerate() {
            let socket = matches.get(g).copied().unwrap_or(SocketMatch::No);
            let r = GemEffectRef {
                gem: g,
                effect,
                level: i64::from(*gem.level.value()),
                quality: i64::from(*gem.quality.value()) + socket.bonus(),
                socket,
            };
            if effect.support {
                group_supports.push(r);
            } else {
                actives.push((r, gem.gem.is_some(), !global_enabled(gem, n + 1)));
            }
        }
    }
    let mut out = Vec::new();
    for (active_gem, _, global_off) in &actives {
        let effect = active_gem.effect;
        let from_gem = actives
            .iter()
            .any(|(r, gem, _)| *gem && r.effect.id == effect.id);
        let active = effect
            .active_skill
            .as_deref()
            .and_then(|a| data.active_skill(a));
        let minion: &[String] = active.map_or(&[], |a| a.minion_types.as_slice());
        let mut types: BTreeSet<String> = active
            .map(|a| a.types.iter().cloned().collect())
            .unwrap_or_default();
        let mut applied = Vec::new();
        for s in &group_supports {
            if supports(s.effect, effect, from_gem, &types, minion) {
                types.extend(s.effect.added_types.iter().cloned());
                applied.push(*s);
            }
        }
        out.push(GroupSkill {
            active_gem: *active_gem,
            active,
            global_off: *global_off,
            supports: applied,
            types,
        });
    }
    Ok(out)
}

/// Why a gem instance's stats are not all known.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum InstanceProblem {
    /// The effect has no level row PoB could fall back to.
    NoLevel,
    /// The effect names a stat set the data does not have.
    MissingStatSet {
        /// The stat set.
        id: String,
    },
    /// The effect has more than one stat set (skill parts or stages):
    /// which one a stat comes from is not modelled.
    SeveralStatSets,
    /// A level stat with linear interpolation (code 2).
    Interpolated {
        /// The stat.
        stat: String,
    },
}

impl InstanceProblem {
    /// Stable name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::NoLevel => "no-level",
            Self::MissingStatSet { .. } => "missing-stat-set",
            Self::SeveralStatSets => "several-stat-sets",
            Self::Interpolated { .. } => "interpolated-stat",
        }
    }
}

/// A gem instance's stats at its level and quality (module docs).
#[derive(Debug, Clone, PartialEq)]
pub struct Instance<'d> {
    /// The granted effect.
    pub effect: &'d GrantedEffect,
    /// The level used: the gem's, or PoB's fallback; `None` with
    /// [`InstanceProblem::NoLevel`].
    pub level: Option<i64>,
    /// The quality used.
    pub quality: i64,
    /// The stats, summed per stat, sorted by stat ID: integer stats, and
    /// the resolved values of effectiveness-scaled float stats (base
    /// damage); the stat set's implicit stats (flags such as
    /// `is_area_damage`) count 1 each, as PoB's `level[index] or 1` gives a
    /// stat without a level value.
    pub stats: Vec<(String, i64)>,
    /// What is not known.
    pub problems: Vec<InstanceProblem>,
}

/// `calcLib.validateGemLevel`: `level` when the effect has it, else
/// clamped to `1..=#levels` (PoB's `#levels` is the count of the levels
/// numbered from 1 without a gap). `None` when PoB would fall back further
/// (a gem's natural maximum, then any level): not modelled.
fn valid_level(effect: &GrantedEffect, level: i64) -> Option<i64> {
    let has = |l: i64| effect.levels.iter().any(|e| e.level == l);
    if has(level) {
        return Some(level);
    }
    // PoB's `#levels`: the border of the array part, levels 1..n present.
    let mut count = 0;
    while has(count + 1) {
        count += 1;
    }
    let mut clamped = level.max(1);
    if count > 0 {
        clamped = clamped.min(count);
    }
    has(clamped).then_some(clamped)
}

/// The level row of `effect`'s first stat set at the level PoB uses for a
/// gem of `level` (`calcLib.validateGemLevel`): its crit chance, damage
/// effectiveness, base multiplier, and stats. `None` without one.
#[must_use]
pub fn stat_set_level<'d>(
    effect: &GrantedEffect,
    level: i64,
    data: &'d dyn CalcData,
) -> Option<&'d StatSetLevel> {
    let level = valid_level(effect, level)?;
    let set = data.stat_set(effect.stat_sets.first()?)?;
    set.levels.iter().find(|r| r.level == level)
}

/// The granted effect's own level row (`GrantedEffectsPerLevel`: costs,
/// cooldown, stored uses) at the level PoB uses for a gem of `level`.
#[must_use]
pub fn effect_level(effect: &GrantedEffect, level: i64) -> Option<&EffectLevel> {
    let level = valid_level(effect, level)?;
    effect.levels.iter().find(|r| r.level == level)
}

/// The stats of `effect` at `level` and `quality` (module docs).
#[must_use]
pub fn instance<'d>(
    effect: &'d GrantedEffect,
    level: i64,
    quality: i64,
    data: &'d dyn CalcData,
) -> Instance<'d> {
    let mut stats: BTreeMap<String, i64> = BTreeMap::new();
    let mut problems = Vec::new();
    if quality > 0 {
        for q in &effect.quality {
            // PoB's `math.modf(stat[2] * quality)` with `stat[2]` the
            // per-1% value its exporter writes, `per_mille / 1000` as a
            // double; the product is truncated towards zero.
            #[expect(
                clippy::cast_precision_loss,
                reason = "per-mille values and qualities are small integers, exact as f64"
            )]
            let value = (q.per_mille as f64 / 1000.0) * quality as f64;
            #[expect(
                clippy::cast_possible_truncation,
                reason = "the truncated value is a small integer"
            )]
            let value = crate::math::trunc(value) as i64;
            *stats.entry(q.stat.clone()).or_default() += value;
        }
    }
    let level = valid_level(effect, level);
    if level.is_none() {
        problems.push(InstanceProblem::NoLevel);
    }
    // PoB's exporter writes the first (main) stat set only; a later one
    // (skill parts, stages) is flagged and not counted.
    if effect.stat_sets.len() > 1 {
        problems.push(InstanceProblem::SeveralStatSets);
    }
    if let Some(id) = effect.stat_sets.first() {
        match data.stat_set(id) {
            None => problems.push(InstanceProblem::MissingStatSet { id: id.clone() }),
            Some(set) => set_stats(set, level, &mut stats, &mut problems),
        }
    }
    problems.sort();
    problems.dedup();
    Instance {
        effect,
        level,
        quality,
        stats: stats.into_iter().collect(),
        problems,
    }
}

/// A stat set's implicit, level, and constant stats into `stats`.
fn set_stats(
    set: &StatSet,
    level: Option<i64>,
    stats: &mut BTreeMap<String, i64>,
    problems: &mut Vec<InstanceProblem>,
) {
    for s in &set.implicit_stats {
        *stats.entry(s.clone()).or_default() += 1;
    }
    if let Some(row) = level.and_then(|l| set.levels.iter().find(|r| r.level == l)) {
        for f in &row.float_stats {
            // Effectiveness interpolation: the game's resolved value (module
            // docs). Every float stat of the 3.29 data has it; another kind
            // is not modelled.
            if f.interpolation == 3 {
                *stats.entry(f.stat.clone()).or_default() += f.resolved;
            } else {
                problems.push(InstanceProblem::Interpolated {
                    stat: f.stat.clone(),
                });
            }
        }
        for s in &row.stats {
            match s.interpolation {
                None | Some(1) => *stats.entry(s.stat.clone()).or_default() += s.value,
                Some(_) => problems.push(InstanceProblem::Interpolated {
                    stat: s.stat.clone(),
                }),
            }
        }
    }
    for c in &set.constant_stats {
        *stats.entry(c.stat.clone()).or_default() += c.value;
    }
}

/// The main skill's gem instances: the active effect's, then each applied
/// support's, at its gem's level and quality as the build writes them.
/// Not modelled yet: modifiers that raise gem levels or quality ("+1 to
/// Level of all Cold Skill Gems", PoB's `GemLevel`/`GemSupportLevel`
/// mods); their stats are unmapped, so the fields that read the skill are
/// `Partial` while they are in the build.
#[must_use]
pub fn instances<'d>(skill: &MainSkill<'d>, data: &'d dyn CalcData) -> Vec<Instance<'d>> {
    std::iter::once(&skill.active_gem)
        .chain(&skill.supports)
        .map(|r| instance(r.effect, r.level, r.quality, data))
        .collect()
}
