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
//! skill of [`VETTED`], with the summoning skill's supports in its store
//! (P6-7c2, [`super::minion_actor`]); otherwise its offence fields are
//! `unmodelled-skill` `minion-skill`. A minion with items is not computed
//! at all ([`super::minion_actor`]).

use loom_domain::ItemId;

use super::hit::{self, AttackSkill};
use super::skill::{self, GemEffectRef, MainSkill};
use super::sockets::SocketMatch;
use crate::condition::{Conditions, Flags};
use crate::engine::{Actor, Calc};
use crate::error::CalcError;
use crate::ingest::EffectTemplate;
use crate::quantity::{Op, Quantity};
use crate::result::{Field, MinionField};
use crate::source::Source;
use crate::state::UnsupportedReason;

/// The minion skills the engine computes, by granted effect ID, each a
/// plain attack in PoB's data (no part, stat, base mod, or function; a
/// test checks them with [`hit::pob_vetted`]): PoB's default attack
/// (`Data/Skills/other.lua` 1215: `attack`, `melee`, `projectile`) and the
/// Raging Spirit's (`Data/Skills/minion.lua` 2132: `attack`, `melee`).
/// The Carrion Golem's combo attack (`Data/Skills/minion.lua` 1465) joined
/// in P6-7c3c: its own stat map is [`OWN_STATS`].
pub const VETTED: &[&str] = &["BoneGolemMultiAttack", "Melee", "SumonRagingSpiritMelee"];

/// PoB's own stat maps of the minion skills the engine computes (P6-7c3c),
/// by skill and stat: the quantity, how it combines, PoB's `div`, and the
/// mod's flags. The Carrion Golem's combo attack: `Speed` MORE with
/// `KeywordFlag.Attack` and `Damage` MORE, each `div = 1.5`
/// (`Data/Skills/minion.lua` 1472 to 1481). These entries are untagged, so
/// the generated table's `own_stats` (tagged entries only) leaves them out
/// and [`hit::pob_vetted`] accepts the skill; without these rows its stats
/// would be unmapped.
pub const OWN_STATS: &[(&str, &str, Quantity, Op, f64, Flags)] = &[
    (
        "BoneGolemMultiAttack",
        "bone_golem_attack_speed_+%_final_after_third_combo_index",
        Quantity::Speed,
        Op::More,
        1.5,
        Flags::KEYWORD_ATTACK,
    ),
    (
        "BoneGolemMultiAttack",
        "bone_golem_damage_+%_final_after_third_combo_index",
        Quantity::Damage,
        Op::More,
        1.5,
        Flags::NONE,
    ),
];

/// The effect of `skill`'s own stat `stat` ([`OWN_STATS`]), if PoB's stat
/// map has it.
#[must_use]
pub fn own_stat(skill: &str, stat: &str) -> Option<EffectTemplate> {
    OWN_STATS
        .iter()
        .find(|(s, id, ..)| *s == skill && *id == stat)
        .map(|&(_, _, quantity, op, divisor, flags)| EffectTemplate {
            quantity,
            op,
            scale: 1.0,
            divisor,
            conditions: Conditions::none().with_all(flags),
        })
}

/// The minion spells the engine computes (P6-7c3), by granted effect ID,
/// each a plain self-cast spell in PoB's data (a test checks them: PoB's
/// `spell` flag, no totem flag, [`hit::pob_vetted`]; `spell` checks one
/// level and a cast time) whose own stats the mapping reads (none
/// `unmapped-stat`): the Flame Golem's Immolate (`Data/Skills/minion.lua`
/// 104) and the Lightning Golem's Storm Orb (390; its cooldown caps its
/// rate, P6-7c3b).
pub const VETTED_SPELLS: &[&str] = &[
    "FireElementalFlameRedSummoned",
    "LightningGolemArcSummoned",
    "MMSPyromaniacIceMortar",
];

/// The source of the minion skill's own stats (P6-7c3): its skill list,
/// as the summoning skill's supports are ([`super::minion_actor`]).
pub const SKILL_RULE: &str = "minion-skill";

/// The minion's main skill as the hit path computes it.
enum MinionSkill<'d> {
    Attack(AttackSkill<'d>),
    Spell(hit::SpellSkill<'d>),
}

/// The minion's offence fields the slice compares.
pub const FIELDS: [MinionField; 6] = [
    MinionField::AverageDamage,
    MinionField::Speed,
    MinionField::HitChance,
    MinionField::CritChance,
    MinionField::CritMultiplier,
    MinionField::HitDps,
];

/// The minion's fields the attack path writes that the slice does not
/// compare yet: not computed (P6-7c review), never an unchecked value. The
/// minion's ignite, bleeding, poison, and impale joined the slice in
/// revision 11 (P6-7c3d).
const UNVALIDATED: [MinionField; 2] = [MinionField::HitSpeed, MinionField::DotDps];

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
    let sources: std::collections::BTreeSet<Source> =
        skill::main_skill(input.build, input.data, &levels)
            .map(|s| {
                let group = s.group.saturating_sub(1);
                s.supports
                    .iter()
                    .map(|r| Source::Gem { group, gem: r.gem })
                    .chain(std::iter::once(Source::Rule(SKILL_RULE)))
                    .collect()
            })
            .unwrap_or_default();
    match minion_skill(&input, &levels, skill_id.as_deref(), weapon.as_deref()) {
        Ok(skill) => calc
            .with_actor(Actor::Minion, |c| {
                attributes(c)?;
                // The minion skill's own list: its own stats and the
                // summoning skill's supports (PoB multiplies their mores
                // apart from the minion's; P6-7c review).
                c.set_skill_sources(sources.clone());
                match &skill {
                    MinionSkill::Attack(attack) => hit::attack_hit(c, attack)?,
                    MinionSkill::Spell(spell) => hit::spell_hit(c, spell)?,
                }
                // Fields the slice does not compare yet: not computed until
                // they are (P6-7c review).
                for field in UNVALIDATED {
                    if c.field(Field::Minion(field)).is_some() {
                        c.set_unsupported(Field::Minion(field), UnsupportedReason::NotComputed);
                    }
                }
                // The minion's combined damage per second (P6-7c3d), as
                // the character's from the minion's copies.
                let main = match &skill {
                    MinionSkill::Attack(attack) => &attack.skill,
                    MinionSkill::Spell(spell) => &spell.skill,
                };
                super::combined::combine(c, main, None)
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

/// The minion's main skill as the hit path computes it: a vetted attack
/// with its monster weapon, or a vetted spell (module docs); or why it is
/// not computed.
fn minion_skill<'d>(
    input: &crate::input::CalcInput<'d>,
    levels: &super::gem_levels::GemLevels,
    skill_id: Option<&str>,
    weapon: Option<&str>,
) -> Result<MinionSkill<'d>, UnsupportedReason> {
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
    if VETTED_SPELLS.contains(&id) {
        return spell(input, &summon, id).ok_or_else(|| not("minion-skill"));
    }
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
    // The summoning skill's supports reach the minion's skills through its
    // store ([`super::minion_actor`], P6-7c2).
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
            from_item: false,
        },
        active,
        item_granted: false,
        source: None,
        supports: Vec::new(),
        unapplied: Vec::new(),
        types,
        cost_mores: Vec::new(),
        support_weapons: Vec::new(),
    };
    let arrow = skill::instance(effect, 1, 0, data)
        .stats
        .iter()
        .any(|(s, v)| s == hit::ARROW_STAT && *v != 0);
    Ok(MinionSkill::Attack(AttackSkill {
        group: summon.group.saturating_sub(1),
        level,
        attack_speed_multiplier: level_row.attack_speed_multiplier,
        // The minion's weapon is no item of the build: a sentinel id no
        // build item has (its store holds no item line, so no item reason
        // names it).
        weapon: ItemId(u32::MAX),
        weapon_flags,
        off_weapon: None,
        off_weapon_flags: crate::condition::Flags::NONE,
        double_hits: false,
        arrow,
        part,
        part_index: 0,
        known_parts: hit::pob_vetted(id, 0),
        shield: None,
        skill,
    }))
}

/// A vetted minion spell (P6-7c3): the skill at level 1 (every vetted one
/// has one level, so PoB's highest level the minion reaches is 1), its
/// cast time and PoB's part flags; its own stats are in the minion's store
/// ([`super::minion_actor`]). `None`: its data is missing.
fn spell<'d>(
    input: &crate::input::CalcInput<'d>,
    summon: &MainSkill<'d>,
    id: &str,
) -> Option<MinionSkill<'d>> {
    let data = input.data;
    let effect = data.granted_effect(id)?;
    let active = effect
        .active_skill
        .as_deref()
        .and_then(|a| data.active_skill(a));
    let types = active.map_or_else(Default::default, |a| a.types.iter().cloned().collect());
    let level = skill::stat_set_level(effect, 1, data)?;
    // No cast time: not the self-cast spell path. More than one level:
    // PoB's level is the highest the minion's level reaches, not read
    // here.
    let level_row = skill::effect_level(effect, 1)?;
    if effect.levels.len() != 1 || effect.cast_time <= 0 {
        return None;
    }
    let (part, _) = hit::pob_part(id, 1)?;
    Some(MinionSkill::Spell(hit::SpellSkill {
        skill: MainSkill {
            group: summon.group,
            active_gem: GemEffectRef {
                gem: summon.active_gem.gem,
                effect,
                level: 1,
                bonus: 0,
                quality: 0,
                socket: SocketMatch::No,
                from_item: false,
            },
            active,
            item_granted: false,
            source: None,
            supports: Vec::new(),
            unapplied: Vec::new(),
            types,
            cost_mores: Vec::new(),
            support_weapons: Vec::new(),
        },
        group: summon.group.saturating_sub(1),
        level,
        cast_time: effect.cast_time,
        cooldown: level_row.cooldown,
        stored_uses: level_row.stored_uses,
        area: part.area,
        projectile: part.projectile,
        melee: part.melee,
        vetted: true,
        vetted_triggered: true,
    }))
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

    /// Each own stat row is an untagged entry of its skill's own stat map
    /// in PoB's data (`Data/Skills/minion.lua` 1472 to 1481), and its
    /// skill is computed (P6-7c3c).
    #[test]
    fn own_stats_should_be_pobs_stat_map_entries() {
        for (skill, stat, ..) in OWN_STATS {
            let pob = super::super::pob_skill::skill(skill).expect(skill);
            // PoB's table keeps only tagged own stat map entries; these are
            // untagged, so not among them (vetted as plain).
            assert!(!pob.own_stats.contains(stat), "{skill} {stat}");
            assert!(
                VETTED.contains(skill) || VETTED_SPELLS.contains(skill),
                "{skill}"
            );
            assert_eq!(own_stat(skill, stat).map(|t| t.divisor), Some(1.5));
        }
        assert!(own_stat("Melee", OWN_STATS[0].1).is_none());
    }

    /// Each vetted minion spell is a plain self-cast spell in PoB's data:
    /// its spell flag without a totem flag, vetted by the hit's own check
    /// (`spell` checks the game data's cooldown and cast time).
    #[test]
    fn vetted_minion_spells_should_be_plain_spells() {
        for id in VETTED_SPELLS {
            let pob = super::super::pob_skill::skill(id).expect(id);
            assert!(pob.flag("spell"), "{id}");
            assert!(!pob.flag("totem"), "{id}");
            assert!(hit::pob_vetted(id, 0), "{id}");
            assert!(hit::pob_part(id, 1).is_some(), "{id}");
        }
    }
}
