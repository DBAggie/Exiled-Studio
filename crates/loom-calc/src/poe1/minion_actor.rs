//! The main skill's minion (Phase 6 P6-7b3a, plan decision 7; plan
//! `dev/active/phase-6-mechanics/p6-7b-minion-actor-plan.md`): which minion
//! it is, its level, its base values, its mod list, and its life and
//! defences, as PoB v2.67.2 computes them for `env.minion`.
//!
//! **Which minion** (`Modules/CalcActiveSkill.lua` 658 to 700): the skill's
//! `minionList` ([`PobSkill::minions`]) and the minions its supports add
//! (`addMinionList`), the gem's `skillMinion` when it is in that list, else
//! the first. PoB's name maps to a game variety through its template
//! ([`super::pob_minion`]). Not computed, each with a reason on the minion
//! fields:
//! - a skill that makes minions without a list (Raise Spectre picks from
//!   the build's spectre library, P6-7c): `minion-list`;
//! - a minion PoB writes by hand (no variety) or one the data lacks:
//!   `minion-data`;
//! - a minion with items (`minionHasItemSet`, `minionUses`,
//!   `minionUseBowAndQuiver`, `minionUseMainHandWeapon`: its defences come
//!   from items, not modelled): `minion-items`;
//! - a hostile minion (`#hostile`): `hostile-minion`;
//! - a level PoB takes from the enemy, the character, or a support's level
//!   requirement: `minion-level`.
//!
//! **Level** (`CalcActiveSkill.lua` 706 to 710): `skillData.minionLevel`,
//! else the gem level's level requirement, clamped to 1 to 100. The bundle
//! does not carry level requirements; the game's
//! `base_display_minion_actor_level` stat is read instead (stat-ID first).
//! It equals PoB's level requirement at 1,822 of PoB's 1,861 minion gem
//! levels; the 39 others are Vaal Summon Skeletons', where the game's stat
//! is lower (a divergence when a fixture reaches it).
//!
//! **Base values** (`Modules/CalcPerform.lua` `initMinionModDB`, 1119 to
//! 1145; `calcs.initModDB`, `CalcSetup.lua` 18 to 27), source
//! [`BASE_RULE`]:
//! - life: `floor(table[level] x life)`, the table `monsterLifeTable2` or
//!   `3` for `AltLife1` or `AltLife2`, else `monsterAllyLifeTable`;
//! - energy shield, when the type has any: `floor(monsterAllyLifeTable[level]
//!   x life x energyShield)`, `energyShield = 0.4 x EnergyShield / 100`;
//! - armour and evasion: `round(table[level] x (multiplier or 1))`: PoB's
//!   exporter omits a zero multiplier, so such a minion gets the whole
//!   table (a `pob-derivation` with an open question);
//! - the type's merciless resistances;
//! - the maximum resistances, block, and spell block of the character
//!   constants, read through the mapping as the character's are.
//!
//! The multipliers are the numbers PoB's exporter wrote with Lua's
//! `tostring` (14 significant digits, [`lua_number`]).
//!
//! **Mod list** (`initMinionModDB` 1146 to 1148; the exporter, `minions.lua`
//! 215 to 241), source [`MOD_RULE`]: the variety's mods, special mods, and
//! `.ot` stats ([`loom_data::bundle::poe1::ObjectStats`]), read through
//! [`MinionMapping`]. PoB's exporter turns a stat into a mod only through
//! the skill stat map; any other is a comment with no effect.
//!
//! **Template extras** (`#mod`, [`TEMPLATE_MODS`]), source
//! [`TEMPLATE_RULE`].
//!
//! **Fields** ([`phase`]): life (`doActorLifeMana`, after the resistances in
//! `calcs.defence`, `CalcDefence.lua` 647 to 657), energy shield, armour,
//! evasion, the four resistances, and attack block, with the character's
//! formulas ([`super::phases`]); the minion has no item. Its reasons are its
//! own and the character's (an unmapped "Minions have ..." line).
//!
//! Not yet: the character's `MinionModifier` lines, keystones granted to
//! the minion, `StrengthAddedToMinions` and `ResistanceAddedToMinions`
//! (P6-7b3b); buffs and auras on the minion (P6-7b4). Each such source is a
//! reason on the minion fields ([`deferred_reasons`]).

use loom_data::bundle::poe1::{LifeScaling, MinionType, MinionVariety};
use loom_domain::{StatId, StatLine, StatValue};

use super::mapping::Poe1Mapping;
use super::phases::{self, Valued};
use super::pob_minion::{self, PobMinion};
use super::pob_skill::{self, PobSkill};
use super::{Defence, gem_levels::GemLevels, skill, states};
use crate::condition::{ConditionKey, Conditions};
use crate::engine::{Actor, ActorState, Calc};
use crate::error::CalcError;
use crate::ingest::{EffectTemplate, Ingest, StatMap, StatMapping};
use crate::input::CalcInput;
use crate::math;
use crate::quantity::{DamageType, Op, Quantity};
use crate::result::{Field, MinionField};
use crate::source::Source;
use crate::state::{Reasons, UnsupportedReason};
use crate::store::{Effect, ModStore};

/// The source of the minion's base values.
pub const BASE_RULE: &str = "minion-base";
/// The source of the variety's mods and `.ot` stats.
pub const MOD_RULE: &str = "minion-mod";
/// The source of PoB's template extras (`#mod`).
pub const TEMPLATE_RULE: &str = "minion-template";

/// The skills whose minion takes items: every skill PoB v2.67.2 gives
/// `minionHasItemSet` or `minionUses` (`Data/Skills/act_dex.lua` 190, 390,
/// 494; `act_str.lua` 474, 561, 6225; `other.lua` 77, 2552; `glove.lua`
/// 1198, 1236, 1274, 1312), sorted. Its defences come from them, not
/// modelled. Animate Weapon of Ranged Arms (`AnimateWeaponAltX`) takes the
/// character's weapon through [`ITEM_SKILL_DATA`] (P6-7b3a review).
const ITEM_SKILLS: [&str; 12] = [
    "AnimateGuardian",
    "AnimateGuardianAltX",
    "AnimateGuardiansWeapon",
    "AnimateWeapon",
    "AnimateWeaponAltY",
    "EnchantmentOfReflectionWhenHit",
    "EnchantmentOfReflectionWhenHit2",
    "EnchantmentOfReflectionWhenHit3",
    "EnchantmentOfReflectionWhenHit4",
    "HolyStrike",
    "ManifestDancingDervishes",
    "VaalAnimateWeapon",
];

/// PoB's skill data keys that give the minion the character's weapon
/// (`initMinionModDB` 1158 to 1172).
const ITEM_SKILL_DATA: [&str; 2] = ["minionUseBowAndQuiver", "minionUseMainHandWeapon"];

/// The game's minion level stat ([`level`]).
pub const LEVEL_STAT: &str = "base_display_minion_actor_level";

/// The stats that set the minion's level from the enemy or the character
/// (`SkillStatMap.lua` 307 to 312): not modelled.
const LEVEL_FROM_OTHERS: [&str; 2] = [
    "display_skill_minions_level_is_corpse_level",
    "minion_actor_level_is_user_level_up_to_maximum",
];

/// Why the minion is not computed.
fn not_computed(source: &Source, what: &'static str) -> UnsupportedReason {
    UnsupportedReason::UnmodelledSkill {
        source: source.clone(),
        what,
    }
}

/// A number as PoB's exporter wrote it: Lua's `tostring`, 14 significant
/// digits, read back.
#[must_use]
pub fn lua_number(x: f64) -> f64 {
    format!("{x:.13e}").parse().unwrap_or(x)
}

/// The main skill's minion, collected (module docs), or why it is not
/// computed; `None` when the main skill makes no minion.
#[must_use]
pub fn minion(
    input: &CalcInput<'_>,
    player: &ModStore,
) -> Option<Result<ActorState, UnsupportedReason>> {
    let build = input.build;
    // A build without any minion skill gem pays no more than this scan.
    let any_minion_gem = skill::active_groups(build).ok()?.iter().any(|g| {
        g.gems.typed().any(|gem| {
            skill::gem_effects(gem, input.data)
                .unwrap_or_default()
                .iter()
                .any(|e| {
                    pob_skill::skill(&e.id)
                        .is_some_and(|p| !p.minions.is_empty() || !p.add_minions.is_empty())
                        || e.active_skill
                            .as_deref()
                            .and_then(|id| input.data.active_skill(id))
                            .is_some_and(|a| a.types.iter().any(|t| t == "CreatesMinion"))
                })
        })
    });
    if !any_minion_gem {
        return None;
    }
    let levels = GemLevels::from_effects(player.effects());
    let skill = skill::main_skill(build, input.data, &levels).ok()?;
    let group_index = skill.group.checked_sub(1)?;
    let group = skill::active_groups(build)
        .ok()?
        .get(group_index)
        .copied()?;
    if !group.enabled {
        return None;
    }
    let source = Source::Gem {
        group: group_index,
        gem: skill.active_gem.gem,
    };
    let pob = pob_skill::skill(&skill.effect().id);
    let own: &[&str] = pob.map_or(&[], |p| p.minions);
    let mut list: Vec<&str> = own.to_vec();
    for support in &skill.supports {
        for name in pob_skill::skill(&support.effect.id).map_or(&[][..], |p| p.add_minions) {
            if !list.contains(name) {
                list.push(name);
            }
        }
    }
    let Some(&first) = list.first() else {
        return skill
            .types
            .contains("CreatesMinion")
            .then(|| Err(not_computed(&source, "minion-list")));
    };
    let chosen = group
        .gems
        .typed()
        .nth(skill.active_gem.gem)
        .and_then(|g| skill::attribute(&g.attributes, "skillMinion"));
    let name = chosen.filter(|c| list.contains(c)).unwrap_or(first);
    Some(collect(
        input,
        player,
        &skill,
        pob,
        &source,
        name,
        own.contains(&name),
    ))
}

/// The minion `name`'s state (module docs).
fn collect(
    input: &CalcInput<'_>,
    player: &ModStore,
    skill: &skill::MainSkill<'_>,
    pob: Option<&PobSkill>,
    source: &Source,
    name: &str,
    own: bool,
) -> Result<ActorState, UnsupportedReason> {
    let data = input.data;
    if ITEM_SKILLS
        .binary_search(&skill.effect().id.as_str())
        .is_ok()
        || pob.is_some_and(|p| {
            ITEM_SKILL_DATA
                .iter()
                .any(|k| p.skill_data(k).is_some_and(pob_skill::Value::truthy))
        })
    {
        return Err(not_computed(source, "minion-items"));
    }
    let template = pob_minion::minion(name).ok_or_else(|| not_computed(source, "minion-data"))?;
    if template.hostile.is_some_and(|h| h != "false") {
        return Err(not_computed(source, "hostile-minion"));
    }
    let minions = data
        .poe1()
        .and_then(|p| p.minions())
        .ok_or_else(|| not_computed(source, "minion-data"))?;
    let variety = minions
        .varieties
        .iter()
        .find(|v| v.id == template.variety)
        .ok_or_else(|| not_computed(source, "minion-data"))?;
    let ty = minions
        .types
        .iter()
        .find(|t| t.id == variety.monster_type)
        .ok_or_else(|| not_computed(source, "minion-data"))?;
    let gem = skill.active_gem;
    let instance = skill::instance(gem.effect, gem.level, gem.quality, data);
    let level =
        level(&instance.stats, pob, own).ok_or_else(|| not_computed(source, "minion-level"))?;

    let mapping = MinionMapping;
    let mut ingest = Ingest::new(&mapping);
    for reason in deferred_reasons(input, player, skill) {
        ingest.reason(reason);
    }
    base_values(input, variety, ty, level, &mut ingest)
        .ok_or_else(|| not_computed(source, "minion-data"))?;
    mod_list(input, minions, variety, &mut ingest);
    let mut states = std::collections::BTreeMap::new();
    template_mods(template, &mut ingest, &mut states);
    // The minion holds no weapon (PoB's `weaponData2` is empty without
    // items): it never dual wields.
    states.insert(ConditionKey::new(states::DUAL_WIELDING), false);
    let (store, global) = ingest.finish();
    Ok(ActorState {
        store,
        global,
        states,
        skill_sources: std::collections::BTreeSet::new(),
    })
}

/// The minion's level (module docs), clamped to 1 to 100; `None` when PoB
/// takes it from elsewhere.
fn level(stats: &[(String, i64)], pob: Option<&PobSkill>, own: bool) -> Option<i64> {
    let get = |id: &str| stats.iter().find(|(s, _)| s == id).map(|(_, v)| *v);
    if !own || LEVEL_FROM_OTHERS.iter().any(|s| get(s).is_some()) {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "PoB's minion levels are small integers"
    )]
    let from_data = pob
        .and_then(|p| p.skill_data("minionLevel"))
        .and_then(pob_skill::Value::num)
        .map(|n| n as i64);
    let level = from_data.or_else(|| get(LEVEL_STAT))?;
    Some(level.clamp(1, 100))
}

/// A base value of the minion.
fn base(quantity: Quantity, op: Op, value: f64) -> Effect {
    Effect {
        quantity,
        op,
        value,
        conditions: Conditions::none(),
        source: Source::Rule(BASE_RULE),
    }
}

/// The minion's base life, energy shield, armour, and evasion.
#[derive(Debug, Clone, Copy, PartialEq)]
struct BaseNumbers {
    life: f64,
    /// `None` when the type has no energy shield (PoB writes none).
    energy_shield: Option<f64>,
    armour: f64,
    evasion: f64,
}

/// The base numbers (module docs) from the level's table rows: `life`
/// percent is the variety's, `armour` PoB's armour table value, `evasion`
/// the evasion table's; `None` for an `AltLife1` type without its column.
fn base_numbers(
    life: i64,
    ty: &MinionType,
    lives: crate::input::MonsterLife,
    armour: f64,
    evasion: i64,
) -> Option<BaseNumbers> {
    #[expect(clippy::cast_precision_loss, reason = "game integers, exact as f64")]
    let f = |v: i64| v as f64;
    let life_mult = lua_number(f(life) / 100.0);
    let table = match ty.life_scaling() {
        Some(LifeScaling::AltLife1) => lives.alt1?,
        Some(LifeScaling::AltLife2) => lives.alt2,
        None => lives.ally,
    };
    // The ally table even for an AltLife type (`initMinionModDB` 1127).
    let energy_shield = (ty.energy_shield != 0).then(|| {
        let es = lua_number(0.4 * f(ty.energy_shield) / 100.0);
        math::floor(f(lives.ally) * life_mult * es)
    });
    let or_one = |v: i64| {
        if v == 0 {
            1.0
        } else {
            lua_number(f(v) / 100.0)
        }
    };
    Some(BaseNumbers {
        life: math::floor(f(table) * life_mult),
        energy_shield,
        armour: math::round_half_up(armour * or_one(ty.armour)),
        evasion: math::round_half_up(f(evasion) * or_one(ty.evasion)),
    })
}

/// The base values (module docs); `None` when the data lacks a table row.
fn base_values(
    input: &CalcInput<'_>,
    variety: &MinionVariety,
    ty: &MinionType,
    level: i64,
    ingest: &mut Ingest<'_>,
) -> Option<()> {
    let data = input.data;
    let n = base_numbers(
        variety.life,
        ty,
        data.monster_life(level)?,
        super::enemy::monster_armour(data, level)?,
        data.monster_level(level)?.evasion,
    )?;
    ingest.effect(base(Quantity::Life, Op::Base, n.life));
    if let Some(es) = n.energy_shield {
        ingest.effect(base(Quantity::EnergyShield, Op::Base, es));
    }
    ingest.effect(base(Quantity::Armour, Op::Base, n.armour));
    ingest.effect(base(Quantity::Evasion, Op::Base, n.evasion));
    #[expect(clippy::cast_precision_loss, reason = "game integers, exact as f64")]
    let f = |v: i64| v as f64;
    for (t, value) in phases::RESISTANCES.iter().zip(ty.resistances) {
        ingest.effect(base(Quantity::Resistance(*t), Op::Base, f(value)));
    }
    for stat in [
        "base_maximum_all_resistances_%",
        "maximum_block_%",
        "base_maximum_spell_block_%",
    ] {
        let value = data.character_constant(stat)?;
        ingest.stats_with(
            &Poe1Mapping,
            &Source::Rule(BASE_RULE),
            &[StatLine {
                stat: StatId::new(stat),
                value: StatValue::Exact {
                    min: value,
                    max: value,
                },
            }],
        );
    }
    Some(())
}

/// The variety's mods, special mods, and `.ot` stats (module docs). A mod
/// the data lacks, or `.ot` stats the bundle does not hold, are reasons.
fn mod_list(
    input: &CalcInput<'_>,
    minions: &loom_data::bundle::poe1::Minions,
    variety: &MinionVariety,
    ingest: &mut Ingest<'_>,
) {
    let source = Source::Rule(MOD_RULE);
    let line = |stat: &str, value: i64| StatLine {
        stat: StatId::new(stat),
        value: StatValue::Exact {
            min: value,
            max: value,
        },
    };
    for id in variety.mods.iter().chain(&variety.special_mods) {
        match input.data.mod_stats(id) {
            // PoB's `Stat<i>Value[1]`: the lowest value.
            Some(stats) => {
                let lines: Vec<StatLine> = stats.iter().map(|(s, min)| line(s, *min)).collect();
                ingest.stats(&source, &lines);
            }
            None => ingest.reason(UnsupportedReason::UnmodelledSkill {
                source: source.clone(),
                what: "minion-mod",
            }),
        }
    }
    if variety.object_type.is_empty() || variety.object_type == "Metadata/Monsters/Monster" {
        return;
    }
    match minions.object_stats(&variety.object_type) {
        Some(o) => {
            let lines: Vec<StatLine> = o.stats.iter().map(|(s, v)| line(s, *v)).collect();
            ingest.stats(&source, &lines);
            if o.unread.is_some() || !o.values.is_empty() {
                ingest.reason(UnsupportedReason::UnmodelledSkill {
                    source,
                    what: "minion-object-stats",
                });
            }
        }
        None => ingest.reason(UnsupportedReason::UnmodelledSkill {
            source,
            what: "minion-object-stats",
        }),
    }
}

/// How PoB's template extras for the skills' minions read
/// (`Export/Minions/Minions.txt`; every one is listed, `Spectres.txt`'s are
/// P6-7c's).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TemplateMod {
    /// An effect on the minion.
    Effect(Quantity, Op, f64),
    /// A condition PoB sets on the minion (`Condition:FullLife`).
    State(&'static str),
    /// A mod no minion field reads (damage, speed, damage taken): P6-7c's.
    Later,
}

/// PoB's `#mod` extras for the skills' minions, as written, and how they
/// read.
pub const TEMPLATE_MODS: [(&str, TemplateMod); 7] = [
    (
        "mod(\"Life\", \"MORE\", 698), -- MonsterUnique2",
        TemplateMod::Effect(Quantity::Life, Op::More, 698.0),
    ),
    (
        "mod(\"Damage\", \"MORE\", 70), -- MonsterUnique5",
        TemplateMod::Later,
    ),
    (
        "mod(\"Speed\", \"MORE\", 33, ModFlag.Attack), -- MonsterUnique8",
        TemplateMod::Later,
    ),
    (
        "mod(\"Damage\", \"MORE\", -33, ModFlag.Attack), -- MonsterUnique8",
        TemplateMod::Later,
    ),
    ("mod(\"Damage\", \"MORE\", 300)", TemplateMod::Later),
    ("mod(\"DamageTaken\", \"MORE\", -70)", TemplateMod::Later),
    (
        "mod(\"Condition:FullLife\", \"FLAG\", true)",
        TemplateMod::State("FullLife"),
    ),
];

/// The template's extras (module docs): an unknown one is a reason.
fn template_mods(
    template: &PobMinion,
    ingest: &mut Ingest<'_>,
    states: &mut std::collections::BTreeMap<ConditionKey, bool>,
) {
    for m in template.mods {
        match TEMPLATE_MODS.iter().find(|(text, _)| *text == m.text) {
            Some((_, TemplateMod::Effect(quantity, op, value))) => ingest.effect(Effect {
                quantity: *quantity,
                op: *op,
                value: *value,
                conditions: Conditions::none(),
                source: Source::Rule(TEMPLATE_RULE),
            }),
            Some((_, TemplateMod::State(key))) => {
                states.insert(ConditionKey::new(*key), true);
            }
            Some((_, TemplateMod::Later)) => {}
            None => ingest.reason(UnsupportedReason::UnmodelledSkill {
                source: Source::Rule(TEMPLATE_RULE),
                what: "minion-template-mod",
            }),
        }
    }
}

/// How a monster stat reads on a minion ([`MinionMapping`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MinionStat {
    /// An effect a minion field reads.
    Effect(Quantity, Op, Option<&'static str>),
    /// PoB maps it to a mod no minion field reads (damage, speed, damage
    /// taken, immunities, charges): P6-7c's.
    Later,
}

/// The stats PoB's exporter turns into mods on the skills' minions (the
/// skill stat map; `Data/Minions.lua`'s `mod(...)` lines), sorted, and how
/// they read; every other stat is a comment in PoB, with no effect.
pub const MINION_STATS: [(&str, MinionStat); 29] = [
    ("active_skill_attack_speed_+%_final", MinionStat::Later),
    ("attack_maximum_added_physical_damage", MinionStat::Later),
    ("attack_minimum_added_physical_damage", MinionStat::Later),
    ("base_block_%_damage_taken", MinionStat::Later),
    ("base_cannot_be_damaged", MinionStat::Later),
    ("base_cannot_be_stunned", MinionStat::Later),
    (
        "base_maximum_energy_shield",
        MinionStat::Effect(Quantity::EnergyShield, Op::Base, None),
    ),
    (
        "base_physical_damage_%_to_convert_to_chaos",
        MinionStat::Later,
    ),
    (
        "base_physical_damage_%_to_convert_to_cold",
        MinionStat::Later,
    ),
    (
        "base_physical_damage_%_to_convert_to_fire",
        MinionStat::Later,
    ),
    (
        "block_while_dual_wielding_%",
        MinionStat::Effect(Quantity::BlockChance, Op::Base, Some(states::DUAL_WIELDING)),
    ),
    ("cannot_be_knocked_back", MinionStat::Later),
    (
        "dual_wield_inherent_attack_speed_+%_final",
        MinionStat::Later,
    ),
    (
        "life_leech_from_physical_attack_damage_permyriad",
        MinionStat::Later,
    ),
    ("life_regeneration_rate_per_minute_%", MinionStat::Later),
    (
        "main_hand_local_maximum_added_physical_damage",
        MinionStat::Later,
    ),
    (
        "main_hand_local_minimum_added_physical_damage",
        MinionStat::Later,
    ),
    (
        "monster_base_block_%",
        MinionStat::Effect(Quantity::BlockChance, Op::Base, None),
    ),
    ("monster_inherent_damage_taken_+%_final", MinionStat::Later),
    (
        "physical_damage_reduction_rating_+%",
        MinionStat::Effect(Quantity::Armour, Op::Increased, None),
    ),
    ("set_base_avoid_projectiles_%_chance", MinionStat::Later),
    ("set_base_cannot_be_damaged", MinionStat::Later),
    (
        "set_energy_shield_recharge_rate_per_minute_%",
        MinionStat::Later,
    ),
    ("set_immune_to_curses", MinionStat::Later),
    ("set_max_endurance_charges", MinionStat::Later),
    ("set_max_frenzy_charges", MinionStat::Later),
    ("set_max_power_charges", MinionStat::Later),
    (
        "set_maximum_life_is_one",
        MinionStat::Effect(Quantity::Life, Op::Override, None),
    ),
    ("stun_threshold_+%", MinionStat::Later),
];

/// The mapping of a minion's mod list (module docs): [`MINION_STATS`];
/// every other stat has no effect, as PoB writes it as a comment. The
/// character constants of the base values are read with [`Poe1Mapping`].
#[derive(Debug, Clone, Copy, Default)]
pub struct MinionMapping;

impl StatMapping for MinionMapping {
    fn map(&self, stat: &StatId) -> StatMap {
        match MINION_STATS.binary_search_by(|(id, _)| (*id).cmp(stat.as_str())) {
            Ok(i) => match MINION_STATS.get(i).map(|(_, s)| *s) {
                Some(MinionStat::Effect(quantity, op, state)) => {
                    let conditions = state.map_or_else(Conditions::none, |key| {
                        Conditions::none().with_state(ConditionKey::new(key), true)
                    });
                    StatMap::Effects(vec![EffectTemplate {
                        quantity,
                        op,
                        scale: 1.0,
                        divisor: 1.0,
                        conditions,
                    }])
                }
                Some(MinionStat::Later) | None => StatMap::NoEffect,
            },
            Err(_) => StatMap::NoEffect,
        }
    }
}

/// The minion's fields, written as [`Field::Minion`] (module docs); with a
/// minion the engine cannot compute, each carries why.
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
    calc.with_actor(Actor::Minion, fields).unwrap_or(Ok(()))
}

/// The minion's fields.
pub const FIELDS: [MinionField; 9] = [
    MinionField::Life,
    MinionField::EnergyShield,
    MinionField::Armour,
    MinionField::Evasion,
    MinionField::Resistance(DamageType::Fire),
    MinionField::Resistance(DamageType::Cold),
    MinionField::Resistance(DamageType::Lightning),
    MinionField::Resistance(DamageType::Chaos),
    MinionField::BlockChance,
];

/// What the character gives the minion that P6-7b3b and P6-7b4 model, a
/// reason on every minion field until then (module docs):
/// - each [`super::Poe1Quantity::MinionPending`] effect of the character
///   (a `MinionModifier` on the minion's life or defences), with its
///   source: `minion-modifier`;
/// - each stat of the main skill's gems (the active one and its supports)
///   that PoB's stat map sends to the minion
///   ([`PobSkill::minion_stats`]: the gem's "Minions have #% more Life"):
///   `minion-modifier`, with the gem as source;
/// - each other enabled gem (in any group, the main one included) whose
///   active skill is an aura, or a buff PoB applies to minions
///   (`skillData.buffMinions`, `buffAllies`; `CalcPerform.lua` 2282 and
///   2427): `minion-buffs`.
///
/// PoB also adds the skill's `extraSkillModList` to the minion
/// (`initMinionModDB` 1149 to 1151): "Socketed Gems have ..." item lines.
/// The engine maps none of them, so each is already an `unmapped-stat`
/// reason of the character, which every minion field carries.
fn deferred_reasons(
    input: &CalcInput<'_>,
    player: &ModStore,
    skill: &skill::MainSkill<'_>,
) -> Vec<UnsupportedReason> {
    let mut out = Vec::new();
    for e in player.effects() {
        if e.quantity == Quantity::Poe1(super::Poe1Quantity::MinionPending) {
            out.push(UnsupportedReason::UnmodelledSkill {
                source: e.source.clone(),
                what: "minion-modifier",
            });
        }
    }
    let main_group = skill.group.saturating_sub(1);
    for gem in std::iter::once(&skill.active_gem).chain(&skill.supports) {
        let Some(pob) = pob_skill::skill(&gem.effect.id) else {
            continue;
        };
        let instance = skill::instance(gem.effect, gem.level, gem.quality, input.data);
        if instance
            .stats
            .iter()
            .any(|(s, v)| *v != 0 && pob.minion_stats.binary_search(&s.as_str()).is_ok())
        {
            out.push(UnsupportedReason::UnmodelledSkill {
                source: Source::Gem {
                    group: main_group,
                    gem: gem.gem,
                },
                what: "minion-modifier",
            });
        }
    }
    let Ok(groups) = skill::active_groups(input.build) else {
        return out;
    };
    for (index, group) in groups.iter().enumerate() {
        if !group.enabled {
            continue;
        }
        let main = index + 1 == skill.group;
        for (gem_index, gem) in group.gems.typed().enumerate() {
            // The main group's other active skills (an aura linked beside
            // the summon gem) reach the minion too (P6-7b3a review).
            if !gem.enabled || (main && gem_index == skill.active_gem.gem) {
                continue;
            }
            for effect in skill::gem_effects(gem, input.data).unwrap_or_default() {
                let active = effect
                    .active_skill
                    .as_deref()
                    .and_then(|id| input.data.active_skill(id));
                let aura = active.is_some_and(|a| a.types.iter().any(|t| t == "Aura"));
                let buff = pob_skill::skill(&effect.id).is_some_and(|p| {
                    ["buffMinions", "buffAllies"]
                        .iter()
                        .any(|k| p.skill_data(k).is_some_and(pob_skill::Value::truthy))
                });
                if aura || buff {
                    out.push(UnsupportedReason::UnmodelledSkill {
                        source: Source::Gem {
                            group: index,
                            gem: gem_index,
                        },
                        what: "minion-buffs",
                    });
                }
            }
        }
    }
    out
}

fn fields(calc: &mut Calc<'_>) -> Result<(), CalcError> {
    let set = |calc: &mut Calc<'_>, field: MinionField, value: Valued, reasons: Reasons| {
        calc.set(Field::Minion(field), phases::query(value, reasons));
    };
    for t in phases::RESISTANCES {
        let (value, reasons) = phases::resistance(calc, t)?;
        set(calc, MinionField::Resistance(t), value, reasons);
    }
    let (value, reasons) = phases::minion_life(calc)?;
    set(calc, MinionField::Life, value, reasons);
    for (defence, field, quantity) in [
        (
            Defence::EnergyShield,
            MinionField::EnergyShield,
            Quantity::EnergyShield,
        ),
        (Defence::Armour, MinionField::Armour, Quantity::Armour),
        (Defence::Evasion, MinionField::Evasion, Quantity::Evasion),
    ] {
        let (value, reasons) = phases::primary_defence(calc, defence, quantity, None)?;
        set(calc, field, value, reasons);
    }
    let block = phases::attack_block(calc, None, Reasons::new())?;
    set(calc, MinionField::BlockChance, block.value, block.reasons);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PoB's exporter writes multipliers with Lua's `tostring`.
    #[test]
    fn lua_number_should_keep_14_significant_digits() {
        assert_eq!(lua_number(0.4 * 50.0 / 100.0), 0.2);
        assert_eq!(lua_number(105.0 / 100.0), 1.05);
        assert_eq!(lua_number(1.0 / 3.0), 0.333_333_333_333_33);
    }

    /// The table is sorted (it is searched), and the stats a minion field
    /// reads map as PoB's `Data/Minions.lua` lines read; every other
    /// stat, PoB's comments, has no effect.
    #[test]
    fn the_minion_mapping_should_follow_pobs_minion_lines() {
        assert!(
            MINION_STATS
                .windows(2)
                .all(|w| matches!(w, [a, b] if a.0 < b.0))
        );
        let effect = |s: &str| match MinionMapping.map(&StatId::new(s)) {
            StatMap::Effects(e) => e.first().map(|t| (t.quantity, t.op)),
            _ => None,
        };
        assert_eq!(
            effect("base_maximum_energy_shield"),
            Some((Quantity::EnergyShield, Op::Base))
        );
        assert_eq!(
            effect("physical_damage_reduction_rating_+%"),
            Some((Quantity::Armour, Op::Increased))
        );
        assert_eq!(
            effect("set_maximum_life_is_one"),
            Some((Quantity::Life, Op::Override))
        );
        assert_eq!(
            effect("monster_base_block_%"),
            Some((Quantity::BlockChance, Op::Base))
        );
        for s in [
            "set_item_drop_slots",
            "stun_threshold_+%",
            "monster_no_talismans",
        ] {
            assert!(
                matches!(MinionMapping.map(&StatId::new(s)), StatMap::NoEffect),
                "{s}"
            );
        }
    }

    /// Every `#mod` extra of `Minions.txt` has a reading.
    #[test]
    fn every_minion_template_mod_should_be_read() {
        for m in super::super::pob_minions::MINIONS {
            if m.template != pob_minion::Template::Minions {
                continue;
            }
            for x in m.mods {
                assert!(
                    TEMPLATE_MODS.iter().any(|(t, _)| *t == x.text),
                    "{}: {}",
                    m.name,
                    x.text
                );
            }
        }
    }

    fn ty(es: i64, armour: i64, evasion: i64, alt1: bool, alt2: bool) -> MinionType {
        MinionType {
            id: "T".to_owned(),
            accuracy: 100,
            armour,
            evasion,
            energy_shield: es,
            damage_spread: 20,
            resistances: [40, 40, 40, 20],
            alt_life1: alt1,
            alt_life2: alt2,
            damage_ignores_attack_speed: false,
            player_minion: true,
        }
    }

    /// `initMinionModDB`: life on the ally table (or an AltLife one),
    /// floored; energy shield on the ally table, only for a type with some;
    /// armour and evasion rounded, the whole table for a zero multiplier.
    #[test]
    fn base_numbers_should_follow_init_minion_mod_db() {
        let lives = crate::input::MonsterLife {
            life: 1000,
            ally: 400,
            alt1: Some(300),
            alt2: 200,
        };
        // Life 1.05: floor(400 x 1.05) = 420; no ES; `or 1`.
        let n = base_numbers(105, &ty(0, 0, 0, false, false), lives, 1001.0, 501).unwrap();
        assert_eq!(n.life, 420.0);
        assert_eq!(n.energy_shield, None);
        assert_eq!((n.armour, n.evasion), (1001.0, 501.0));
        // ES 50 (0.2): floor(400 x 1.05 x 0.2) = 84; armour x 0.5 rounds
        // half up; evasion x 0.33.
        let n = base_numbers(105, &ty(50, 50, 33, false, false), lives, 1001.0, 501).unwrap();
        assert_eq!(n.energy_shield, Some(84.0));
        assert_eq!(n.armour, 501.0);
        assert_eq!(n.evasion, 165.0);
        // AltLife1 and AltLife2 (the second wins); ES stays on the ally table.
        let n = base_numbers(100, &ty(50, 0, 0, true, false), lives, 1.0, 1).unwrap();
        assert_eq!((n.life, n.energy_shield), (300.0, Some(80.0)));
        let n = base_numbers(100, &ty(0, 0, 0, true, true), lives, 1.0, 1).unwrap();
        assert_eq!(n.life, 200.0);
        let no_alt1 = crate::input::MonsterLife {
            alt1: None,
            ..lives
        };
        assert_eq!(
            base_numbers(100, &ty(0, 0, 0, true, false), no_alt1, 1.0, 1),
            None
        );
    }

    /// The item skills are sorted (searched) and hold Holy Strike and the
    /// enchantments (P6-7b3a review).
    #[test]
    fn item_skills_should_be_pobs_and_sorted() {
        assert!(ITEM_SKILLS.windows(2).all(|w| matches!(w, [a, b] if a < b)));
        for id in [
            "HolyStrike",
            "AnimateGuardiansWeapon",
            "ManifestDancingDervishes",
        ] {
            assert!(ITEM_SKILLS.binary_search(&id).is_ok(), "{id}");
        }
    }

    /// The level: `minionLevel` skill data, else the game's level stat,
    /// clamped; a support's minion or a level from others is not known.
    #[test]
    fn the_level_should_be_the_games_stat_clamped() {
        let stats = |v: &[(&str, i64)]| -> Vec<(String, i64)> {
            v.iter().map(|(s, n)| ((*s).to_owned(), *n)).collect()
        };
        assert_eq!(level(&stats(&[(LEVEL_STAT, 66)]), None, true), Some(66));
        assert_eq!(level(&stats(&[(LEVEL_STAT, 120)]), None, true), Some(100));
        assert_eq!(level(&stats(&[(LEVEL_STAT, 66)]), None, false), None);
        assert_eq!(level(&stats(&[]), None, true), None);
        assert_eq!(
            level(
                &stats(&[
                    (LEVEL_STAT, 66),
                    ("display_skill_minions_level_is_corpse_level", 1)
                ]),
                None,
                true
            ),
            None
        );
    }
}
