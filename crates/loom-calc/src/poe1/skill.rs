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
//! level is the gem's plus the build's global gem level modifiers its
//! tags match ([`super::gem_levels`], P3-8i: PoB's `applyGemMods`), kept
//! when the effect has that level, else clamped to `1..=#levels`; the
//! stats are the quality stats (`modf(per 1% x quality)`), the level's integer stats, and the stat set's constant
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

use super::gem_levels::GemLevels;
use super::sockets::{SocketMatch, socket_matches};
use crate::condition::Flags;
use crate::input::{
    ActiveSkill, CalcData, EffectLevel, GemEffect, GrantedEffect, StatSet, StatSetLevel,
};
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
            Self::Uncertain => "main-skill-uncertain",
        }
    }
}

/// One granted effect of a gem in a group, with the gem's level and
/// quality as PoB uses them.
#[derive(Debug, Clone, Copy)]
pub struct GemEffectRef<'d> {
    /// The gem's position among the group's typed gems (0-based).
    pub gem: usize,
    /// The granted effect.
    pub effect: &'d GrantedEffect,
    /// The instance's level: the gem's, plus [`GemEffectRef::bonus`]
    /// (PoB's `level` after `applyGemMods`).
    pub level: i64,
    /// The levels the build's gem level modifiers add (P3-8i,
    /// [`super::gem_levels`]).
    pub bonus: i64,
    /// The instance's quality: the gem's, plus
    /// [`super::sockets::BONUS`] when it sits in a socket of its colour
    /// (PoB's `actualQuality`, P3-12i).
    pub quality: i64,
    /// Whether the gem gains the socket colour bonus.
    pub socket: SocketMatch,
    /// A support an item grants (P6-11r, [`super::item_supports`]): its
    /// `gem` is past the group's gems.
    pub from_item: bool,
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
    /// The cost and reservation multipliers, in percent, the item in the
    /// group's slot gives its socketed skills (P6-11bi, PoB's
    /// `ExtraSkillMod` `SupportManaMultiplier`).
    pub cost_mores: Vec<f64>,
}

/// PoB's weapon type for a game item class: its exporter's
/// `weaponClassMap` (`Export/Scripts/skills.lua` 15 to 31) for a skill's
/// list, and the item bases' `type` for a held weapon (a Warstaff is a
/// `Staff`, a Rune Dagger a `Dagger`). None: a class PoB has no weapon type
/// for (a shield, a quiver).
fn pob_weapon_type(class: &str) -> Option<&'static str> {
    Some(match class {
        "Claw" => "Claw",
        "Dagger" | "Rune Dagger" => "Dagger",
        "Wand" => "Wand",
        "One Hand Sword" => "One Handed Sword",
        "Thrusting One Hand Sword" => "Thrusting One Handed Sword",
        "One Hand Axe" => "One Handed Axe",
        "One Hand Mace" => "One Handed Mace",
        "Bow" => "Bow",
        "Fishing Rod" => "Fishing Rod",
        "Staff" | "Warstaff" => "Staff",
        "Two Hand Sword" => "Two Handed Sword",
        "Two Hand Axe" => "Two Handed Axe",
        "Two Hand Mace" => "Two Handed Mace",
        "Sceptre" => "Sceptre",
        "Unarmed" => "None",
        _ => return None,
    })
}

/// [`MainSkill::can_use`] for an active skill row (none: not restricted).
/// PoB's list keeps only the classes its exporter maps, so a list of other
/// classes (Shield Crush's `Shield`) restricts nothing; a held class
/// without a PoB type is not checked (`getWeaponFlags` finds no info).
fn can_use(active: Option<&ActiveSkill>, class: &str) -> bool {
    let Some(held) = pob_weapon_type(class) else {
        return true;
    };
    let Some(active) = active else { return true };
    let mut listed = active
        .weapon_classes
        .iter()
        .filter_map(|c| pob_weapon_type(c))
        .peekable();
    listed.peek().is_none() || listed.any(|t| t == held)
}

/// [`MainSkill::disabled_by_hands`] for an active skill row and its types.
fn disabled_by_hands(
    active: Option<&ActiveSkill>,
    types: &BTreeSet<String>,
    hands: &super::collect::Hands,
) -> Result<bool, &'static str> {
    use super::collect::Held;
    let has = |t: &str| types.contains(t);
    let restricted = active.is_some_and(|a| {
        a.weapon_classes
            .iter()
            .any(|c| pob_weapon_type(c).is_some())
    });
    if has("RequiresShield") {
        match hands.off_hand {
            Held::Shield => {}
            Held::Unknown => return Err("requires-shield"),
            Held::Empty | Held::Weapon(_) | Held::Other => return Ok(true),
        }
    }
    let dual_only = has("DualWieldOnly");
    // The main hand: PoB's `weaponData1` is the weapon, or the class's
    // unarmed data (type `None`, the game's `Unarmed`) for an empty hand.
    let main = match &hands.main_hand {
        Held::Empty => Some("Unarmed"),
        Held::Weapon(class) => Some(class.as_str()),
        Held::Unknown if restricted || dual_only => return Err("weapon-flags"),
        Held::Unknown | Held::Shield | Held::Other => None,
    };
    match main {
        Some(class) if !can_use(active, class) => return Ok(true),
        None if dual_only => return Ok(true),
        _ => {}
    }
    // The off hand: PoB's `weaponData2` is `{}` (no weapon info) for an
    // empty hand or an item that is not a weapon, which never disables.
    match &hands.off_hand {
        Held::Weapon(class) if !can_use(active, class) => Ok(true),
        Held::Weapon(class) => Ok(has("DualWieldRequiresDifferentTypes") && main == Some(class)),
        Held::Unknown if restricted || dual_only => Err("weapon-flags"),
        Held::Unknown => Ok(false),
        Held::Empty | Held::Shield | Held::Other => Ok(dual_only),
    }
}

impl<'d> MainSkill<'d> {
    /// Whether PoB disables the skill for what the hands hold
    /// (`CalcActiveSkill.lua` 269 to 334): it requires a shield and the off
    /// hand holds none; the main hand's weapon, or `Unarmed` for an empty
    /// hand, is not one it can use; the off hand holds a weapon it cannot
    /// use; it is `DualWieldOnly` without a weapon in each hand, or
    /// `DualWieldRequiresDifferentTypes` with one class in both. PoB then
    /// computes no offence for it (`CalcOffence.lua` 339). `Err`: a hand's
    /// base is not known, so the answer is not either.
    ///
    /// Not modelled, as in [`Self::can_use`]: supports' weapon types,
    /// `countsAsAll1H`, Hollow Palm Technique's unarmed off hand, and
    /// item-granted skills tied to their weapon (`forceSourceWeapon`).
    ///
    /// # Errors
    /// `"requires-shield"` or `"weapon-flags"`: a hand's base is not known.
    pub fn disabled_by_hands(&self, hands: &super::collect::Hands) -> Result<bool, &'static str> {
        disabled_by_hands(self.active, &self.types, hands)
    }

    /// Whether the skill can use a weapon of `class` (PoB's `weaponTypes`,
    /// `CalcActiveSkill.lua` 192 to 223; P6-11e review): the game lists the
    /// classes per active skill (none: any weapon; `Unarmed` for an empty
    /// hand), compared as PoB's weapon types (`pob_weapon_type`: a list
    /// of classes PoB does not map restricts nothing). A skill without an
    /// active skill row is not restricted.
    ///
    /// Not modelled: a support's own `weaponTypes` (PoB adds each applied
    /// support's list, 287 to 291, and the weapon must be in every list),
    /// `countsAsAll1H` (a weapon that counts as every one-handed class,
    /// Varunastra, 200), and a minion skill's empty hand (`summonSkill`
    /// ignores the weapon types, 296 to 299): the active skill's own list
    /// and the hand's own class decide.
    #[must_use]
    pub fn can_use(&self, class: &str) -> bool {
        can_use(self.active, class)
    }

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
    ///
    /// Only the hit's contexts take them today. PoB builds its ailment and
    /// damage over time configs from the skill's keyword flags too (without
    /// `Hit`; `Modules/CalcOffence.lua` 4297, 4570, 4875, 5655): when those
    /// are computed, their contexts must take these flags as well.
    #[must_use]
    pub fn keyword_flags(&self) -> Flags {
        keyword_flags(&self.types)
    }
}

/// The skill keywords of a skill with `types` ([`MainSkill::keyword_flags`]).
#[must_use]
pub fn keyword_flags(types: &BTreeSet<String>) -> Flags {
    KEYWORD_TYPES
        .iter()
        .filter(|(name, _)| types.contains(*name))
        .fold(Flags::NONE, |flags, (_, t)| {
            flags | Flags::skill_keyword(*t)
        })
}

/// The game's skill type names that give a skill keyword (the game's
/// `ActiveSkillType` IDs, which PoB's `SkillType` names match; a test
/// checks them against the real bundle). No mapped stat reads the
/// physical keyword yet (the game has no "with Physical Skills" stat the
/// slice reaches); it is kept as PoB sets it.
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
    gem_variant(gem, data).map(|(_, effects)| effects)
}

/// [`gem_effects`] with the gem effect (the variant) the gem names: PoB's
/// `gemData`, whose tags gem level modifiers match. A skill an item grants
/// (no `gemId`) has none (P3-8i, [`super::gem_levels`]).
#[must_use]
pub fn gem_variant<'d>(
    gem: &Gem,
    data: &'d dyn CalcData,
) -> Option<(Option<&'d GemEffect>, Vec<&'d GrantedEffect>)> {
    let skill = gem.skill.as_ref().map(loom_domain::SkillId::as_str);
    let Some(gem_id) = &gem.gem else {
        return Some((None, vec![data.granted_effect(skill?)?]));
    };
    let row = data.skill_gem(gem_id.as_str())?;
    let variant = attribute(&gem.attributes, "variantId");
    let effects = || {
        row.effects
            .iter()
            .filter_map(|e| data.gem_effect(e.as_str()))
    };
    // PoB's `LoadSkill` (`Classes/SkillsTab.lua` 437 to 459): the variant
    // `variantId` names, else the one whose granted effect is `skillId`;
    // with neither, `ProcessSocketGroup` finds the gem by its name
    // (`FindSkillGem`, 1166 to 1174; old builds whose `skillId` the game has
    // since renamed, Phase 6).
    let effect = effects()
        .find(|e| match variant {
            Some(v) => e.id == v,
            None => e.granted_effect.as_deref() == skill,
        })
        .or_else(|| {
            let name = gem.name.as_deref()?.trim();
            // A variant's name; the default variant has none in the game
            // data, and its name is its active skill's (the gem's base).
            let shown = |e: &GemEffect| -> Option<String> {
                if !e.name.is_empty() {
                    return Some(e.name.clone());
                }
                let granted = data.granted_effect(e.granted_effect.as_deref()?)?;
                Some(
                    data.active_skill(granted.active_skill.as_deref()?)?
                        .name
                        .clone(),
                )
            };
            effects().find(|e| {
                !name.is_empty() && shown(e).is_some_and(|n| n.eq_ignore_ascii_case(name))
            })
        })?;
    let mut out = Vec::new();
    for id in effect
        .granted_effect
        .iter()
        .chain(&effect.additional_granted_effects)
    {
        out.push(data.granted_effect(id.as_str())?);
    }
    Some((Some(effect), out))
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

/// PoB's `addBestSupport` (`CalcSetup.lua` 338 to 367, P6-11r): a support
/// whose granted effect is already in `list` replaces that one only with a
/// higher level, or the same level and a higher quality; otherwise it
/// joins the list. (`plusVersionOf` is not in the bundle.)
fn add_best<'d>(list: &mut Vec<GemEffectRef<'d>>, support: GemEffectRef<'d>) {
    match list.iter_mut().find(|o| o.effect.id == support.effect.id) {
        Some(other) => {
            if (support.level, support.quality) > (other.level, other.quality) {
                *other = support;
            }
        }
        None => list.push(support),
    }
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

/// For each gem of a socket group, in order: the indexes (in the group) of
/// the active gems it supports, as [`main_skill`] matches supports to its
/// active skill (each active with the types the enabled supports before
/// it add). An active gem, a gem the data does not know, and a support
/// that fits no active gem have none. For showing which support sits
/// under which skill; the calculation reads only its main skill.
#[must_use]
pub fn group_links(group: &SocketGroup, data: &dyn CalcData) -> Vec<Vec<usize>> {
    let gems: Vec<_> = group.gems.typed().collect();
    let first = |g: &Gem| gem_effects(g, data).and_then(|e| e.first().copied());
    let mut out = vec![Vec::new(); gems.len()];
    for (a, gem) in gems.iter().enumerate() {
        let Some(effect) = first(gem).filter(|e| !e.support) else {
            continue;
        };
        let active = effect
            .active_skill
            .as_deref()
            .and_then(|s| data.active_skill(s));
        let minion: &[String] = active.map_or(&[], |s| s.minion_types.as_slice());
        let mut types: BTreeSet<String> = active
            .map(|s| s.types.iter().cloned().collect())
            .unwrap_or_default();
        for (s, support) in gems.iter().enumerate() {
            let Some(se) = first(support).filter(|e| e.support) else {
                continue;
            };
            if supports(se, effect, gem.gem.is_some(), &types, minion) {
                if support.enabled {
                    types.extend(se.added_types.iter().cloned());
                }
                if let Some(list) = out.get_mut(s) {
                    list.push(a);
                }
            }
        }
    }
    out
}

/// Whether `support` (a support's granted effect) would support an active
/// gem of `group`, each active with the types the group's enabled supports
/// add: for a gem picker that offers the supports that fit the group.
#[must_use]
pub fn fits_group(support: &GrantedEffect, group: &SocketGroup, data: &dyn CalcData) -> bool {
    let gems: Vec<_> = group.gems.typed().collect();
    let first = |g: &Gem| gem_effects(g, data).and_then(|e| e.first().copied());
    gems.iter().any(|gem| {
        let Some(effect) = first(gem).filter(|e| !e.support) else {
            return false;
        };
        let active = effect
            .active_skill
            .as_deref()
            .and_then(|s| data.active_skill(s));
        let minion: &[String] = active.map_or(&[], |s| s.minion_types.as_slice());
        let mut types: BTreeSet<String> = active
            .map(|s| s.types.iter().cloned().collect())
            .unwrap_or_default();
        for other in &gems {
            if let Some(se) = first(other).filter(|e| e.support)
                && other.enabled
                && supports(se, effect, gem.gem.is_some(), &types, minion)
            {
                types.extend(se.added_types.iter().cloned());
            }
        }
        supports(support, effect, true, &types, minion)
    })
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

/// A group's enabled gems' granted effects in PoB's order: the active ones
/// (with whether they come from a gem and whether PoB counts them only
/// without a global effect) and the supports. Each takes its gem's level
/// plus the levels `levels` gives the gem's tags (P3-8i: PoB's
/// `applyGemMods`, for actives and supports alike), and its gem's quality
/// plus its socket colour bonus.
///
/// # Errors
fn group_effects<'d>(
    group: &SocketGroup,
    matches: &[SocketMatch],
    data: &'d dyn CalcData,
    levels: &GemLevels,
) -> (Vec<Candidate<'d>>, Vec<GemEffectRef<'d>>) {
    let mut actives: Vec<Candidate<'d>> = Vec::new();
    let mut supports: Vec<GemEffectRef<'d>> = Vec::new();
    // The group's slot, whose item's socketed levels reach its gems
    // (P6-11q).
    let slot = group
        .slot
        .as_ref()
        .and_then(|s| super::gem_levels::GemSlot::of(s.as_str()));
    // The supports the slot's item grants come first, unless the group is
    // an item's (P6-11r; PoB's `ExtraSupport`, `CalcSetup.lua` 1550 to
    // 1555). Each has a gem index past the group's gems.
    if let Some(slot) = slot
        && attribute(&group.attributes, "source").is_none()
    {
        let count = group.gems.typed().count();
        for (k, (id, level)) in levels.item_supports(slot).enumerate() {
            if let Some(effect) = data.granted_effect(id).filter(|e| e.support) {
                supports.push(GemEffectRef {
                    gem: count + k,
                    effect,
                    level,
                    bonus: 0,
                    quality: 0,
                    socket: SocketMatch::No,
                    from_item: true,
                });
            }
        }
    }
    for (g, gem) in group.gems.typed().enumerate() {
        if !gem.enabled {
            continue;
        }
        // A gem the data does not have grants nothing: PoB skips a gem
        // without `gemData` (`Modules/CalcSetup.lua` 1649; Phase 6). Its
        // reason is `poe1::gems`' ([`unknown_gems`]).
        let Some((variant, effects)) = gem_variant(gem, data) else {
            continue;
        };
        let bonus = levels.bonus(variant.map(|v| v.tags.as_slice()), slot);
        for (n, effect) in effects.into_iter().enumerate() {
            let socket = matches.get(g).copied().unwrap_or(SocketMatch::No);
            let r = GemEffectRef {
                gem: g,
                effect,
                level: i64::from(*gem.level.value()) + bonus,
                bonus,
                quality: i64::from(*gem.quality.value()) + socket.bonus(),
                socket,
                from_item: false,
            };
            if effect.support {
                add_best(&mut supports, r);
            } else {
                actives.push((r, gem.gem.is_some(), !global_enabled(gem, n + 1)));
            }
        }
    }
    (actives, supports)
}

/// The enabled gems of `group` the data does not have (by position among
/// the group's typed gems): they grant nothing, as in PoB, and each is a
/// reason (Phase 6).
#[must_use]
pub fn unknown_gems(group: &SocketGroup, data: &dyn CalcData) -> Vec<usize> {
    group
        .gems
        .typed()
        .enumerate()
        .filter(|(_, gem)| gem.enabled && gem_variant(gem, data).is_none())
        .map(|(i, _)| i)
        .collect()
}

/// The build's main skill (module docs), its gems at the levels `levels`
/// gives them.
///
/// # Errors
/// A [`SkillProblem`] when the build has no main skill the engine can name.
pub fn main_skill<'d>(
    build: &Build,
    data: &'d dyn CalcData,
    levels: &GemLevels,
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

    let (actives, group_supports) = group_effects(group, matches, data, levels);
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
        if (!s.from_item || from_gem) && supports(s.effect, effect, from_gem, &types, minion) {
            types.extend(s.effect.added_types.iter().cloned());
            applied.push(s);
        } else {
            unapplied.push(s);
        }
    }
    let slot = group
        .slot
        .as_ref()
        .and_then(|s| super::gem_levels::GemSlot::of(s.as_str()));
    Ok(MainSkill {
        group: index,
        active_gem,
        active,
        item_granted: attribute(&group.attributes, "source").is_some(),
        supports: applied,
        unapplied,
        types,
        cost_mores: levels.cost_multipliers(slot),
    })
}

/// A copy of `build` whose main skill is the active skill `active` of the
/// group `group` (both 1-based, as PoB's `mainSocketGroup` and
/// `mainActiveSkill`), in the active skill set (P6-8a: a trigger's source
/// skill, which PoB computes as its own skill, `calcs.buildActiveSkill`).
#[must_use]
pub fn with_main_skill(build: &Build, group: usize, active: usize) -> Build {
    let mut b = build.clone();
    b.character.main_socket_group = u32::try_from(group).ok().map(loom_domain::Sourced::new);
    let active_id = b.skills.active;
    let set = match active_id.filter(|id| b.skills.sets.typed().any(|s| s.id == *id)) {
        Some(id) => b.skills.sets.typed_mut().find(|s| s.id == id),
        None => b.skills.sets.typed_mut().next(),
    };
    if let Some(g) = set.and_then(|s| s.groups.typed_mut().nth(group.wrapping_sub(1))) {
        g.main_active_skill = u32::try_from(active).ok().map(loom_domain::Sourced::new);
    }
    b
}

/// Every active skill of the active skill set's enabled groups, in PoB's
/// `activeSkillList` order (group by group, each group's active skills in
/// order): the group and the active skill's index (both 1-based) and the
/// skill as [`main_skill`] picks it (P6-8a: trigger sources and the skills
/// a trigger triggers). A group whose pick is uncertain is left out.
#[must_use]
pub fn active_skills<'d>(
    build: &Build,
    data: &'d dyn CalcData,
    levels: &GemLevels,
) -> Vec<(usize, usize, MainSkill<'d>)> {
    let Ok(groups) = active_groups(build) else {
        return Vec::new();
    };
    let all = socket_matches(build, &groups, data);
    let mut out = Vec::new();
    for (i, group) in groups.iter().enumerate() {
        if !group.enabled {
            continue;
        }
        let matches: &[SocketMatch] = all.get(i).map_or(&[], Vec::as_slice);
        let (actives, _) = group_effects(group, matches, data, levels);
        for a in 1..=actives.len() {
            let b = with_main_skill(build, i + 1, a);
            if let Ok(skill) = main_skill(&b, data, levels) {
                out.push((i + 1, a, skill));
            }
        }
    }
    out
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
    /// The cost and reservation multipliers, in percent, the item in the
    /// group's slot gives its socketed skills (P6-11bi, PoB's
    /// `ExtraSkillMod` `SupportManaMultiplier`).
    pub cost_mores: Vec<f64>,
}

/// The active effects of a group's enabled gems, each with the group's
/// supports that apply to it, in PoB's order (`CalcSetup.lua` 1645 to
/// 1740: gems in order, each gem's effects in order; supports as for the
/// main skill, module docs). `matches` are the group's gems' socket
/// matches ([`socket_matches`]), each adding its bonus to the gem's
/// quality. A gem the data does not have is skipped, as PoB does
/// ([`unknown_gems`] names it).
#[must_use]
pub fn group_skills<'d>(
    group: &SocketGroup,
    matches: &[SocketMatch],
    data: &'d dyn CalcData,
    levels: &GemLevels,
) -> Vec<GroupSkill<'d>> {
    let (actives, group_supports) = group_effects(group, matches, data, levels);
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
            if (!s.from_item || from_gem) && supports(s.effect, effect, from_gem, &types, minion) {
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
            cost_mores: levels.cost_multipliers(
                group
                    .slot
                    .as_ref()
                    .and_then(|s| super::gem_levels::GemSlot::of(s.as_str())),
            ),
        });
    }
    out
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
    instance_at(effect, level, quality, None, data)
}

/// [`instance`] of a skill an actor of `actor_level` uses (P6-7c3: a
/// minion's skill, PoB's `skillInstance.actorLevel`, `CalcActiveSkill.lua`
/// 567). An effectiveness-scaled stat is then PoB's value at that level
/// ([`effectiveness`]), not the game's resolved one, which is the value at
/// the level requirement.
#[must_use]
pub fn instance_for_actor<'d>(
    effect: &'d GrantedEffect,
    level: i64,
    actor_level: i64,
    data: &'d dyn CalcData,
) -> Instance<'d> {
    instance_at(effect, level, 0, Some(actor_level), data)
}

/// PoB's `SkillDamageBaseEffectiveness` and
/// `SkillDamageIncrementalEffectiveness` (`Data/Misc.lua` 29 and 30).
const SKILL_DAMAGE_EFFECTIVENESS: (f64, f64) = (3.885_209, 0.360_246);

/// PoB's effectiveness of a stat set at `actor_level`
/// (`calcLib.buildSkillInstanceStats`, `CalcTools.lua` 198 to 204):
/// `(base + incremental x (level - 1)) x baseEffectiveness x (1 +
/// incrementalEffectiveness) ^ (level - 1)`, the set's two values as PoB's
/// exporter reads them (single precision, widened).
#[must_use]
pub fn effectiveness(set: &StatSet, actor_level: i64) -> f64 {
    let (base, incremental) = SKILL_DAMAGE_EFFECTIVENESS;
    #[expect(clippy::cast_precision_loss, reason = "a level, exact as f64")]
    let steps = (actor_level - 1) as f64;
    (base + incremental * steps)
        * f64::from(set.base_effectiveness)
        * crate::math::pow(1.0 + f64::from(set.incremental_effectiveness), steps)
}

fn instance_at<'d>(
    effect: &'d GrantedEffect,
    level: i64,
    quality: i64,
    actor_level: Option<i64>,
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
            Some(set) => set_stats(set, level, actor_level, &mut stats, &mut problems),
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
    actor_level: Option<i64>,
    stats: &mut BTreeMap<String, i64>,
    problems: &mut Vec<InstanceProblem>,
) {
    for s in &set.implicit_stats {
        *stats.entry(s.clone()).or_default() += 1;
    }
    if let Some(row) = level.and_then(|l| set.levels.iter().find(|r| r.level == l)) {
        for f in &row.float_stats {
            // Effectiveness interpolation: the game's resolved value (module
            // docs), or PoB's at an actor's level. Every float stat of the
            // 3.29 data has it; another kind is not modelled.
            if f.interpolation == 3 {
                let value = match actor_level {
                    // PoB's `round(availableEffectiveness x level[index])`.
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "a skill's base damage, far inside i64"
                    )]
                    Some(l) => {
                        crate::math::round_half_up(effectiveness(set, l) * f64::from(f.value))
                            as i64
                    }
                    None => f.resolved,
                };
                *stats.entry(f.stat.clone()).or_default() += value;
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
/// support's, at its [`GemEffectRef`] level and quality. Global gem level
/// modifiers are in that level (P3-8i); gem quality modifiers and those
/// that depend on the socket are not modelled: their stats are unmapped,
/// so the fields that read the skill are `Partial` while they are in the
/// build.
#[must_use]
pub fn instances<'d>(skill: &MainSkill<'d>, data: &'d dyn CalcData) -> Vec<Instance<'d>> {
    std::iter::once(&skill.active_gem)
        .chain(&skill.supports)
        .map(|r| instance(r.effect, r.level, r.quality, data))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn active(classes: &[&str]) -> ActiveSkill {
        ActiveSkill {
            id: "TestSkill".to_owned(),
            name: "Test Skill".to_owned(),
            types: vec!["Attack".to_owned()],
            minion_types: Vec::new(),
            weapon_classes: classes.iter().map(|c| (*c).to_owned()).collect(),
            manually_cast: true,
        }
    }

    /// PoB's `weaponTypes` check for the active skill (P6-7b1 review): an
    /// empty list or no row allows any weapon; a list allows its classes
    /// only, `Unarmed` for an empty hand.
    #[test]
    fn can_use_should_follow_the_active_skills_weapon_classes() {
        let any = active(&[]);
        let axes = active(&["One Hand Axe", "Unarmed"]);
        assert!(can_use(None, "Bow"));
        assert!(can_use(Some(&any), "Bow"));
        assert!(can_use(Some(&axes), "One Hand Axe"));
        assert!(can_use(Some(&axes), "Unarmed"));
        assert!(!can_use(Some(&axes), "Bow"));
        // Classes PoB's exporter does not map restrict nothing.
        let shields = active(&["Shield"]);
        assert!(can_use(Some(&shields), "One Hand Sword"));
        assert!(can_use(Some(&shields), "Unarmed"));
        // A Warstaff is PoB's Staff.
        let staves = active(&["Staff"]);
        assert!(can_use(Some(&staves), "Warstaff"));
        assert!(can_use(Some(&active(&["Warstaff"])), "Staff"));
        assert!(!can_use(Some(&staves), "Bow"));
    }

    /// PoB's effectiveness at an actor's level (`CalcTools.lua` 198 to
    /// 204), the Flame Golem's Immolate (base 1.2547, incremental 0.0446):
    /// its fire damage `round(effectiveness x 0.8)` to `round(... x 1.2)`,
    /// the values PoB's formula gives in doubles from the single-precision
    /// data (P6-7c3; the public fixture `minion-summon-flame-golem-witch.xml`
    /// equals PoB's hit).
    #[test]
    fn effectiveness_should_follow_pobs_formula_at_the_actors_level() {
        let set = StatSet {
            id: "FireElementalFlameRedSummoned".to_owned(),
            implicit_stats: Vec::new(),
            constant_stats: Vec::new(),
            base_effectiveness: 1.2547,
            incremental_effectiveness: 0.0446,
            damage_incremental_effectiveness: None,
            levels: Vec::new(),
        };
        #[expect(clippy::cast_possible_truncation, reason = "small damage values")]
        let damage = |level: i64| {
            let e = effectiveness(&set, level);
            (
                crate::math::round_half_up(e * f64::from(0.8_f32)) as i64,
                crate::math::round_half_up(e * f64::from(1.2_f32)) as i64,
            )
        };
        assert_eq!(damage(1), (4, 6));
        assert_eq!(damage(68), (523, 785));
        assert_eq!(damage(90), (1753, 2630));
    }

    /// PoB's disabled skill (`CalcActiveSkill.lua` 269 to 334): an empty
    /// main hand is `Unarmed`, an empty off hand never disables, and a
    /// hand whose base is not known leaves the answer open for a skill
    /// that lists weapon classes.
    #[test]
    fn disabled_by_hands_should_follow_pobs_weapon_checks() {
        use super::super::collect::{Hands, Held};
        let types = |t: &[&str]| t.iter().map(|s| (*s).to_owned()).collect::<BTreeSet<_>>();
        let hands = |main: Held, off: Held| Hands {
            main_hand: main,
            off_hand: off,
            ..Hands::default()
        };
        let weapon = |c: &str| Held::Weapon(c.to_owned());
        let swords = active(&["One Hand Sword"]);
        let attack = types(&["Attack"]);
        let check =
            |a: Option<&ActiveSkill>, t: &BTreeSet<String>, h: &Hands| disabled_by_hands(a, t, h);
        // Unarmed is not among its classes: disabled (Holy Strike).
        assert_eq!(
            check(Some(&swords), &attack, &hands(Held::Empty, Held::Empty)),
            Ok(true)
        );
        assert_eq!(
            check(
                Some(&swords),
                &attack,
                &hands(weapon("One Hand Sword"), Held::Empty)
            ),
            Ok(false)
        );
        assert_eq!(
            check(
                Some(&swords),
                &attack,
                &hands(weapon("One Hand Sword"), weapon("Bow"))
            ),
            Ok(true)
        );
        assert_eq!(
            check(
                Some(&swords),
                &attack,
                &hands(weapon("One Hand Sword"), Held::Shield)
            ),
            Ok(false)
        );
        assert_eq!(
            check(Some(&swords), &attack, &hands(Held::Unknown, Held::Empty)),
            Err("weapon-flags")
        );
        // No classes: any hand, an unknown one too.
        assert_eq!(
            check(
                Some(&active(&[])),
                &attack,
                &hands(Held::Unknown, Held::Empty)
            ),
            Ok(false)
        );
        assert_eq!(
            check(None, &types(&["Spell"]), &hands(Held::Empty, Held::Empty)),
            Ok(false)
        );
        let shield = types(&["Attack", "RequiresShield"]);
        assert_eq!(
            check(None, &shield, &hands(Held::Empty, Held::Empty)),
            Ok(true)
        );
        assert_eq!(
            check(None, &shield, &hands(Held::Empty, Held::Shield)),
            Ok(false)
        );
        assert_eq!(
            check(None, &shield, &hands(Held::Empty, Held::Unknown)),
            Err("requires-shield")
        );
        let dual = types(&["Attack", "DualWieldOnly"]);
        assert_eq!(
            check(None, &dual, &hands(weapon("Claw"), Held::Empty)),
            Ok(true)
        );
        assert_eq!(
            check(None, &dual, &hands(weapon("Claw"), weapon("Claw"))),
            Ok(false)
        );
        let different = types(&["Attack", "DualWieldRequiresDifferentTypes"]);
        assert_eq!(
            check(None, &different, &hands(weapon("Claw"), weapon("Claw"))),
            Ok(true)
        );
        assert_eq!(
            check(None, &different, &hands(weapon("Claw"), weapon("Dagger"))),
            Ok(false)
        );
    }
}
