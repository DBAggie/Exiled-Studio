//! The main skill's minion's own hit (Phase 6 P6-7c, plan decision 7): the
//! minion's main skill computed by the character's attack path under the
//! minion actor ([`crate::engine::Calc::with_actor`]), its fields the
//! minion's copies ([`crate::result::Field::for_minion`]).
//!
//! What PoB v2.67.2 does:
//! - **the minion's skills** (`Modules/CalcActiveSkill.lua` 870 to 922):
//!   each of the minion's `skillList` PoB's data has (the variety's skills,
//!   then the template's), at level 1 (or the highest level the minion's
//!   level reaches), with the summoning skill's supports; the main one is
//!   the gem's `skillMinionSkill` (1-based, clamped), else the first;
//! - **its weapon** (`CalcActiveSkill.lua` 719 to 752): `damage =
//!   monsterAllyDamageTable[level] x damage`, times `attackTime` unless the
//!   type's damage ignores attack speed; `PhysicalMin = round(damage x (1 -
//!   damageSpread))`, `PhysicalMax = round(damage x (1 + damageSpread))`,
//!   `AttackRate = 1 / attackTime`, `CritChance = 5`, its type the
//!   variety's main-hand class through the exporter's class map, else
//!   `None` (unarmed); the off hand holds nothing (`weaponData2 = {}`);
//! - **its mod list** (`initMinionModDB`, `CalcPerform.lua` 1141 to 1146):
//!   `CannotBeEvaded` unless the character's `MinionAccuracyEqualsAccuracy`
//!   (an unmapped stat, the character's reason), and `CritMultiplier` BASE
//!   the monster template's `base_critical_strike_multiplier - 100`;
//! - **the enemy** is shared: PoB's curses, debuffs, exposure, and the
//!   configured enemy conditions are the enemy's, so they reach the
//!   minion's hits too ([`super::minion_actor`] copies them);
//! - `calcs.offence(env, env.minion, env.minion.mainSkill)` (3737).
//!
//! What the engine does: the minion's main skill is computed when it is a
//! skill of [`VETTED`] and the summoning skill has no support; otherwise its
//! offence fields are `unmodelled-skill` (`minion-skill`,
//! `minion-skill-support`). A minion with items is not computed at all
//! ([`super::minion_actor`]).

use loom_domain::ItemId;

use super::hit::{self, AttackSkill};
use super::skill::{self, GemEffectRef, MainSkill};
use super::sockets::SocketMatch;
use crate::engine::{Actor, Calc};
use crate::error::CalcError;
use crate::quantity::Quantity;
use crate::result::{Field, MinionField};
use crate::source::Source;
use crate::state::UnsupportedReason;

/// The minion skills the engine computes, by granted effect ID, each a
/// plain attack in PoB's data (no part, stat, base mod, or function; a
/// test checks them with [`hit::pob_vetted`]): PoB's default attack
/// (`Data/Skills/other.lua` 1215: `attack`, `melee`, `projectile`) and the
/// Raging Spirit's (`Data/Skills/minion.lua` 2132: `attack`, `melee`).
pub const VETTED: &[&str] = &["Melee", "SumonRagingSpiritMelee"];

/// The minion's offence fields the slice compares.
pub const FIELDS: [MinionField; 6] = [
    MinionField::AverageDamage,
    MinionField::Speed,
    MinionField::HitChance,
    MinionField::CritChance,
    MinionField::CritMultiplier,
    MinionField::HitDps,
];

/// The phase: the minion's main skill's hit (module docs).
///
/// # Errors
/// A graph error.
pub fn phase(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    if let Some(reason) = calc.minion_problem().cloned() {
        for field in FIELDS {
            calc.set_unsupported(Field::Minion(field), reason.clone());
        }
        return Ok(());
    }
    if !calc.has_minion() {
        return Ok(());
    }
    let input = *calc.input();
    let levels = super::gem_levels::GemLevels::from_effects(calc.store().effects());
    let (skill_id, weapon) = calc
        .with_actor(Actor::Minion, |c| {
            (
                c.actor_skill().map(str::to_owned),
                c.actor_weapon().map(str::to_owned),
            )
        })
        .unwrap_or_default();
    match attack(&input, &levels, skill_id.as_deref(), weapon.as_deref()) {
        Ok(attack) => calc
            .with_actor(Actor::Minion, |c| {
                attributes(c)?;
                hit::attack_hit(c, &attack)
            })
            .unwrap_or(Ok(())),
        Err(reason) => {
            for field in FIELDS {
                calc.set_unsupported(Field::Minion(field), reason.clone());
            }
            Ok(())
        }
    }
}

/// The minion's attributes (its hit reads strength, its chance to hit
/// dexterity, as the character's): `base x inc x more` of its own store,
/// which holds none but the character's minion modifiers give (PoB's
/// minion has no base attributes).
fn attributes(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let cx = calc.context(crate::condition::Flags::NONE);
    for (field, quantity) in [
        (Field::Strength, Quantity::Strength),
        (Field::Dexterity, Quantity::Dexterity),
        (Field::Intelligence, Quantity::Intelligence),
    ] {
        let q = calc.chain(quantity, &cx)?;
        calc.set(field, q);
    }
    Ok(())
}

/// The minion's main skill as an attack with its monster weapon (module
/// docs), or why it is not computed.
fn attack<'d>(
    input: &crate::input::CalcInput<'d>,
    levels: &super::gem_levels::GemLevels,
    skill_id: Option<&str>,
    weapon: Option<&str>,
) -> Result<AttackSkill<'d>, UnsupportedReason> {
    let data = input.data;
    let summon =
        skill::main_skill(input.build, data, levels).map_err(|_| UnsupportedReason::NotComputed)?;
    let source = Source::Gem {
        group: summon.group.saturating_sub(1),
        gem: summon.active_gem.gem,
    };
    let not = |what: &'static str| UnsupportedReason::UnmodelledSkill {
        source: source.clone(),
        what,
    };
    let id = skill_id.ok_or_else(|| not("minion-skill"))?;
    if !VETTED.contains(&id) {
        return Err(not("minion-skill"));
    }
    // The weapon's flags (`getWeaponFlags`, `CalcActiveSkill.lua` 192 to
    // 223): an empty hand (`None`) only `ModFlag.Unarmed` (205 to 209).
    let (weapon_flags, class) = match weapon {
        Some(class) => (
            hit::weapon_type_flags(class).ok_or_else(|| not("minion-weapon"))?,
            class,
        ),
        None => (crate::condition::Flags::UNARMED, "Unarmed"),
    };
    // PoB's part flags, then the weapon's melee or ranged
    // (`CalcActiveSkill.lua` 302 to 308).
    let (mut part, _) = hit::pob_part(id, 1).ok_or_else(|| not("minion-skill"))?;
    (part.melee, part.projectile) = hit::weapon_flags(class, part.melee, part.projectile);
    // PoB gives the minion's skills the summoning skill's supports, which
    // the engine does not apply to them yet.
    if !summon.supports.is_empty() {
        return Err(not("minion-skill-support"));
    }
    let effect = data.granted_effect(id).ok_or_else(|| not("minion-skill"))?;
    let active = effect
        .active_skill
        .as_deref()
        .and_then(|a| data.active_skill(a));
    let types = active.map_or_else(Default::default, |a| a.types.iter().cloned().collect());
    let level = skill::stat_set_level(effect, 1, data).ok_or_else(|| not("minion-skill"))?;
    let level_row = skill::effect_level(effect, 1).ok_or_else(|| not("minion-skill"))?;
    let skill = MainSkill {
        group: summon.group,
        active_gem: GemEffectRef {
            gem: summon.active_gem.gem,
            effect,
            level: 1,
            bonus: 0,
            quality: 0,
            socket: SocketMatch::No,
        },
        active,
        item_granted: false,
        supports: Vec::new(),
        unapplied: Vec::new(),
        types,
    };
    let arrow = skill::instance(effect, 1, 0, data)
        .stats
        .iter()
        .any(|(s, v)| s == hit::ARROW_STAT && *v != 0);
    Ok(AttackSkill {
        group: summon.group.saturating_sub(1),
        level,
        attack_speed_multiplier: level_row.attack_speed_multiplier,
        // The minion's weapon is no item of the build: no item reason can
        // name it (its store holds no item line).
        weapon: ItemId(0),
        weapon_flags,
        off_weapon: None,
        off_weapon_flags: crate::condition::Flags::NONE,
        double_hits: false,
        arrow,
        part,
        known_parts: hit::pob_vetted(id, 0),
        skill,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each vetted minion skill is a plain attack in PoB's data: vetted by
    /// the hit's own check, with PoB's attack flag and a part.
    #[test]
    fn vetted_minion_skills_should_be_plain_attacks() {
        for id in VETTED {
            let pob = super::super::pob_skill::skill(id).expect(id);
            assert!(pob.flag("attack"), "{id}");
            assert!(hit::pob_vetted(id, 0), "{id}");
            assert!(hit::pob_part(id, 1).is_some(), "{id}");
        }
    }
}
