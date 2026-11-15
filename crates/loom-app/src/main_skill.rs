//! The default main skill (Q-UX-1, maintainer 2026-11-15: "do better than
//! PoB").
//!
//! PoB keeps one main socket group number for every skill set, so a build
//! often names a group that is not its damage (after a skill set switch it
//! can point at Flesh and Stone). Exiled Studio picks the main skill itself
//! when a build is imported and when the active skill set changes (a stage
//! or a skill set switch); the user's own pick on the Skills page stands
//! until the next such change.
//!
//! **The rule.** Every enabled active gem of an enabled group of the active
//! skill set is a candidate. Its **kind** ([`Kind`]) is: a damaging skill;
//! a damaging skill that is a movement, triggered, Vaal, warcry, herald,
//! golem, or guard skill; or any other active skill (auras, buffs, curses).
//! Supports are never candidates.
//! 1. **The build's own pick stays** when it deals damage, of either
//!    damaging kind (PoB's `mainSocketGroup` and the group's
//!    `mainActiveSkill`; the author's choice is usually right: Arakaali's
//!    Raise Spiders is a triggered skill), unless another skill's hit DPS,
//!    as the engine computes it **completely**, is higher than the pick's.
//!    A pick of an aura, a buff, or an empty group is replaced (Smite's
//!    "Red Maps" set pointed at Flesh and Stone, Q-UX-1).
//! 2. Otherwise the candidates are ranked by kind, then by the enabled
//!    supports in their group (a build's main skill is usually its longest
//!    link), then by complete hit DPS, then by place (earlier first).
//!
//! **After a skill set switch** (a stage or a skill set), PoB's number was
//! meant for the other set, so rule 1 does not apply: the same skill stays
//! the main skill when the new set has it (by its active skill, a Vaal or
//! transfigured form counting as the same: Smite, Vaal Smite, Smite of
//! Divine Judgement), the best ranked of them; else rule 2 picks.
//!
//! Only a complete hit DPS counts: the engine also returns partial numbers
//! for skills outside its slice (today Ice Nova and Smite are complete,
//! Phase 3), and ranking on those put one-link skills over six-links on
//! the community builds (`tests/game_truth_main_skill.rs`).
//!
//! Skills that deal damage together (Righteous Fire's burning with Fire
//! Trap's ignite) are not combined yet: the engine computes neither.

use std::collections::BTreeSet;

use loom_calc::CalcData;
use loom_calc::Field;
use loom_calc::poe1::skill::{active_groups, gem_effects};
use loom_domain::{Build, Sourced};

use crate::data::GameData;
use crate::session::{self, State};

/// How far a skill is from being a build's main skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// An active skill that deals no damage itself (aura, buff, curse).
    Utility,
    /// A damaging skill players rarely mean as the main one.
    Secondary,
    /// A damaging skill.
    Damage,
}

/// Skill types that make a skill deal damage (the game's
/// `ActiveSkillType` IDs).
/// `Minion` is not one: it marks skills that affect minions (Flesh
/// Offering); `CreatesMinion` marks those that make them.
const DAMAGE_TYPES: [&str; 4] = ["Attack", "Damage", "DamageOverTime", "CreatesMinion"];

/// Skill types of damaging skills that are seldom the main skill.
/// `Movement` is not one: Cyclone and Flicker Strike have it; a movement
/// skill has `Travel` or `Blink`.
const SECONDARY_TYPES: [&str; 8] = [
    "Travel",
    "Blink",
    "Triggered",
    "Vaal",
    "Warcry",
    "Herald",
    "Golem",
    "Guard",
];

/// The kind of a skill with these types.
#[must_use]
pub fn kind(types: &BTreeSet<String>) -> Kind {
    let has = |list: &[&str]| list.iter().any(|t| types.contains(*t));
    if !has(&DAMAGE_TYPES) {
        Kind::Utility
    } else if has(&SECONDARY_TYPES) {
        Kind::Secondary
    } else {
        Kind::Damage
    }
}

/// A main skill: a socket group of the active skill set and an active gem
/// in it, both counted from 1 as PoB counts them (`mainSocketGroup`; the
/// group's `mainActiveSkill`, among its gems that are not supports).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice {
    /// The group.
    pub group: u32,
    /// The active gem.
    pub gem: u32,
}

#[derive(Debug, Clone)]
struct Candidate {
    choice: Choice,
    kind: Kind,
    supports: usize,
    /// Its active skills' base IDs ([`base`]).
    skills: BTreeSet<String>,
}

/// An active skill ID without its Vaal or transfigured form: `vaal_smite`
/// and `smite_alt_x` are `smite`.
fn base(id: &str) -> &str {
    let id = id.strip_prefix("vaal_").unwrap_or(id);
    id.strip_suffix("_alt_x")
        .or_else(|| id.strip_suffix("_alt_y"))
        .unwrap_or(id)
}

/// The candidates of the active skill set (module docs), in build order.
fn candidates(build: &Build, data: &dyn CalcData) -> Vec<Candidate> {
    let Ok(groups) = active_groups(build) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (g, group) in groups.iter().enumerate() {
        if !group.enabled {
            continue;
        }
        // A gem the data does not know counts as active, as the view and
        // the Skills page count it.
        let gems: Vec<_> = group
            .gems
            .typed()
            .map(|gem| (gem, gem_effects(gem, data)))
            .collect();
        let is_support = |e: &Option<Vec<&loom_calc::GrantedEffect>>| {
            e.as_ref()
                .and_then(|e| e.first())
                .is_some_and(|f| f.support)
        };
        let supports = gems
            .iter()
            .filter(|(gem, e)| gem.enabled && is_support(e))
            .count();
        let mut active = 0_u32;
        for (gem, effects) in &gems {
            if is_support(effects) {
                continue;
            }
            active += 1;
            if !gem.enabled {
                continue;
            }
            let Some(effects) = effects else { continue };
            let skills: BTreeSet<String> = effects
                .iter()
                .filter(|e| !e.support)
                .filter_map(|e| e.active_skill.as_deref())
                .map(|a| base(a).to_owned())
                .collect();
            let best = effects
                .iter()
                .filter(|e| !e.support)
                .map(|e| {
                    let types: BTreeSet<String> = e
                        .active_skill
                        .as_deref()
                        .and_then(|a| data.active_skill(a))
                        .map(|a| a.types.iter().cloned().collect())
                        .unwrap_or_default();
                    kind(&types)
                })
                .max();
            let (Some(kind), Ok(group)) = (best, u32::try_from(g + 1)) else {
                continue;
            };
            out.push(Candidate {
                choice: Choice { group, gem: active },
                kind,
                supports,
                skills,
            });
        }
    }
    out
}

/// Sets the build's main skill.
pub fn apply(build: &mut Build, choice: Choice) {
    if build
        .character
        .main_socket_group
        .as_ref()
        .map(|m| *m.value())
        != Some(choice.group)
    {
        match &mut build.character.main_socket_group {
            Some(m) => m.set(choice.group),
            None => build.character.main_socket_group = Some(Sourced::new(choice.group)),
        }
    }
    // PoB: the active set, else the first one ([`active_groups`]).
    let active = build.skills.active;
    let found = active.is_some_and(|id| build.skills.sets.typed().any(|s| s.id == id));
    let Some(set) = (if found {
        build.skills.sets.typed_mut().find(|s| Some(s.id) == active)
    } else {
        build.skills.sets.typed_mut().next()
    }) else {
        return;
    };
    let Some(group) = set
        .groups
        .typed_mut()
        .nth(choice.group.saturating_sub(1) as usize)
    else {
        return;
    };
    if group.main_active_skill.as_ref().map(|m| *m.value()) != Some(choice.gem) {
        match &mut group.main_active_skill {
            Some(m) => m.set(choice.gem),
            None => group.main_active_skill = Some(Sourced::new(choice.gem)),
        }
    }
}

/// The hit DPS the engine computes completely with `choice` as the main
/// skill, if it does.
fn hit_dps(state: &State, data: &GameData, choice: Choice) -> Option<f64> {
    let mut build = state.build.clone();
    apply(&mut build, choice);
    let trial = State {
        build,
        ends: state.ends.clone(),
    };
    let result = session::calculate(&trial, data, &|| false).ok()?;
    result
        .fields
        .get(&Field::HitDps)
        .filter(|f| matches!(f.state, loom_calc::FieldState::Complete))
        .and_then(|f| f.value)
        .filter(|v| v.is_finite() && *v > 0.0)
}

/// The most skills the engine is asked about (each is a full
/// calculation).
const MAX_CALCULATED: usize = 12;

/// The build's own pick: PoB's numbers, each 1 when absent (as the engine
/// reads them).
fn own_pick(build: &Build) -> Choice {
    let group = build
        .character
        .main_socket_group
        .as_ref()
        .map_or(1, |m| *m.value());
    let gem = active_groups(build)
        .ok()
        .and_then(|g| g.get(group.saturating_sub(1) as usize).copied())
        .and_then(|g| g.main_active_skill.as_ref())
        .map_or(1, |m| *m.value());
    Choice { group, gem }
}

/// One ranked candidate ([`ranking`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ranked {
    /// The skill.
    pub choice: Choice,
    /// Its kind.
    pub kind: Kind,
    /// Its complete hit DPS, when the engine was asked and computes it.
    pub dps: Option<f64>,
    /// The enabled supports in its group.
    pub supports: usize,
}

/// The candidates of `state`'s build ranked by rule 2 of the module docs,
/// best first, each damaging one among the [`MAX_CALCULATED`] longest links
/// with its complete hit DPS.
#[must_use]
pub fn ranking(state: &State, data: &GameData) -> Vec<Ranked> {
    let list = candidates(&state.build, data.bundle);
    let mut longest: Vec<&Candidate> = list.iter().filter(|c| c.kind == Kind::Damage).collect();
    longest.sort_by_key(|c| std::cmp::Reverse(c.supports));
    longest.truncate(MAX_CALCULATED);
    let calculate = longest.len() > 1;
    let mut ranked: Vec<(usize, Ranked)> = list
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let dps = (calculate && longest.iter().any(|l| l.choice == c.choice))
                .then(|| hit_dps(state, data, c.choice))
                .flatten();
            (
                i,
                Ranked {
                    choice: c.choice,
                    kind: c.kind,
                    dps,
                    supports: c.supports,
                },
            )
        })
        .collect();
    ranked.sort_by(|(ia, a), (ib, b)| {
        b.kind
            .cmp(&a.kind)
            .then_with(|| b.supports.cmp(&a.supports))
            .then_with(|| b.dps.unwrap_or(0.0).total_cmp(&a.dps.unwrap_or(0.0)))
            .then_with(|| ia.cmp(ib))
    });
    ranked.into_iter().map(|(_, r)| r).collect()
}

/// The default main skill of `state`'s build (module docs); `None` when
/// the active skill set has no active skill.
#[must_use]
pub fn choose(state: &State, data: &GameData) -> Option<Choice> {
    let own = own_pick(&state.build);
    let list = candidates(&state.build, data.bundle);
    if list
        .iter()
        .any(|c| c.choice == own && c.kind != Kind::Utility)
    {
        // Rule 1: it stays unless another computes higher.
        let Some(mine) = hit_dps(state, data, own) else {
            return Some(own);
        };
        let better = list
            .iter()
            .filter(|c| c.kind == Kind::Damage && c.choice != own)
            .take(MAX_CALCULATED)
            .filter_map(|c| hit_dps(state, data, c.choice).map(|d| (c.choice, d)))
            .filter(|(_, d)| *d > mine)
            .max_by(|a, b| a.1.total_cmp(&b.1));
        return Some(better.map_or(own, |(c, _)| c));
    }
    ranking(state, data).first().map(|r| r.choice)
}

/// Sets the default main skill on `state`'s build, if it has one.
pub fn set_default(state: &mut State, data: &GameData) {
    if let Some(choice) = choose(state, data) {
        apply(&mut state.build, choice);
    }
}

/// The base IDs of the build's current main skill (its own pick), to
/// carry across a skill set switch.
#[must_use]
pub fn current_skills(build: &Build, data: &GameData) -> BTreeSet<String> {
    let own = own_pick(build);
    candidates(build, data.bundle)
        .into_iter()
        .find(|c| c.choice == own)
        .map(|c| c.skills)
        .unwrap_or_default()
}

/// The main skill after a skill set switch (module docs): the same skill
/// if the new set has it, else rule 2.
#[must_use]
pub fn choose_after_switch(
    state: &State,
    data: &GameData,
    previous: &BTreeSet<String>,
) -> Option<Choice> {
    let same = candidates(&state.build, data.bundle)
        .into_iter()
        .enumerate()
        .filter(|(_, c)| c.kind != Kind::Utility && !c.skills.is_disjoint(previous))
        .max_by(|(ia, a), (ib, b)| {
            a.kind
                .cmp(&b.kind)
                .then_with(|| a.supports.cmp(&b.supports))
                .then_with(|| ib.cmp(ia))
        })
        .map(|(_, c)| c.choice);
    same.or_else(|| ranking(state, data).first().map(|r| r.choice))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn types(list: &[&str]) -> BTreeSet<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn base_should_join_vaal_and_transfigured_forms() {
        assert_eq!(base("smite"), "smite");
        assert_eq!(base("vaal_smite"), "smite");
        assert_eq!(base("smite_alt_x"), "smite");
        assert_eq!(base("ice_nova_alt_y"), "ice_nova");
        assert_eq!(base("vaal_righteous_fire"), "righteous_fire");
    }

    #[test]
    fn kind_should_rank_damage_over_secondary_over_utility() {
        assert_eq!(kind(&types(&["Attack", "Melee", "Area"])), Kind::Damage);
        assert_eq!(kind(&types(&["Spell", "Damage", "Nova"])), Kind::Damage);
        assert_eq!(
            kind(&types(&["Attack", "Movement", "Travel"])),
            Kind::Secondary
        );
        // Cyclone.
        assert_eq!(
            kind(&types(&["Attack", "Melee", "Movement", "Channel"])),
            Kind::Damage
        );
        // Flesh Offering affects minions; Raise Spectre makes them.
        assert_eq!(
            kind(&types(&["Spell", "Buff", "Minion", "Offering"])),
            Kind::Utility
        );
        assert_eq!(
            kind(&types(&["Spell", "Minion", "CreatesMinion"])),
            Kind::Damage
        );
        assert_eq!(kind(&types(&["Spell", "Damage", "Vaal"])), Kind::Secondary);
        assert_eq!(kind(&types(&["Spell", "Aura", "Buff"])), Kind::Utility);
        assert_eq!(
            kind(&types(&["Spell", "Hex", "AppliesCurse"])),
            Kind::Utility
        );
        assert_eq!(kind(&types(&[])), Kind::Utility);
        assert!(Kind::Damage > Kind::Secondary && Kind::Secondary > Kind::Utility);
    }
}
