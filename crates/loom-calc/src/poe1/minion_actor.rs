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
//! - a skill that makes minions without a list, or Raise Spectre with an
//!   empty spectre library: `minion-list`;
//! - a minion PoB writes by hand (no variety) or one the data lacks:
//!   `minion-data`;
//! - a minion with items (`minionHasItemSet`, `minionUses`,
//!   `minionUseBowAndQuiver`, `minionUseMainHandWeapon`: its defences come
//!   from items, not modelled): `minion-items`;
//! - a hostile minion (`#hostile`): `hostile-minion`;
//! - a level PoB takes from a support's level requirement, or from the
//!   enemy when the enemy model has a problem: `minion-level`.
//!
//! **Spectres** (RV-12; `CalcActiveSkill.lua` 659 to 667, 717, 720): a
//! skill whose `minionList` is empty in PoB's data (Raise Spectre and its
//! transfigured form, [`SPECTRE_SKILLS`]) takes the build's spectre library
//! in its place ([`spectre_library`]: the `Spectre` entries the build file
//! keeps, each a monster variety). A spectre's life comes from the
//! monsters' table (`monsterLifeTable`), not the allies', unless its type
//! has an `AltLife` scaling, and its weapon damage from `monsterDamageTable`.
//! Its level is the gem's `raised_spectre_level` ([`SPECTRE_LEVEL_STAT`],
//! PoB's stat map: `skill("minionLevel", nil)`).
//!
//! **Level** (`CalcActiveSkill.lua` 706 to 711), clamped to 1 to 100: the
//! enemy's level when the skill has `display_skill_minions_level_is_corpse_level`;
//! the character's, at most the stat's value, with
//! `minion_actor_level_is_user_level_up_to_maximum`; else
//! `skillData.minionLevel`, else the gem level's level requirement. For the
//! last the game's `base_display_minion_actor_level` stat is read first
//! (stat-ID first) and the requirement, carried since data schema 5
//! (`EffectLevel::level_requirement`), after. The stat equals PoB's level
//! requirement at 1,822 of PoB's 1,861 minion gem levels; the 39 others are
//! Vaal Summon Skeletons', where the game's stat is lower (DIV-0111 to
//! DIV-0113, DIV-0174; the maintainer's in-game check is open).
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
//! **The character's minion modifiers** (P6-7b3b; `CalcPerform.lua`
//! `addMinionModifiers`, 1213 to 1219, called at 1918 with the main skill's
//! `skillModList`): every unconditional [`Poe1Quantity::Minion`] effect of
//! the character whose kind a minion field reads
//! ([`super::minions::MinionValue::on_minion`]), and every [`Poe1Quantity::GolemMinion`]
//! one when the main skill is a golem skill, is added to the minion with
//! its op, value, and source ([`minion_modifiers`]). The main skill's own
//! gems (the active one and the supports that apply) are read here from
//! their stats through the gem mapping ([`gem_modifiers`]: "Minions have
//! #% more Life" of Minion Life, Stone Golem of Safeguarding's "50% less
//! Life"), since the character's store holds them only when the skill is
//! also a hit or damage over time the engine computes; their effects in
//! the character's store are skipped, so none counts twice.
//!
//! **Auras and buffs** (P6-7b4): the character's auras, and buffs PoB
//! applies to minions, with their rows scaled and merged as PoB's
//! `minionBuffs` ([`super::gems::minion_buffs`]); one the engine does not
//! model is a `minion-buffs` reason.
//!
//! **`extraSkillModList`:** PoB adds the skill's to the minion
//! (`initMinionModDB` 1149 to 1151): "Socketed Gems have ..." item lines,
//! which the engine maps to nothing, so each is already the character's
//! `unmapped-stat` reason, which every minion field carries.
//!
//! Not yet, each a reason: a conditional minion modifier.
//! Keystones granted to the minion (`Keystone` mods on it, 1920 to 1924),
//! `StrengthAddedToMinions` (1207), and `ResistanceAddedToMinions`
//! (`CalcDefence.lua` 648) come from stats the engine does not map: each is
//! already an `unmapped-stat` reason of the character.

use loom_data::bundle::poe1::{LifeScaling, MinionType, MinionVariety};
use loom_domain::{StatId, StatLine, StatValue};

use super::mapping::{Poe1Mapping, SkillMapping};

#[cfg(test)]
use super::minions::MinionValue;
use super::phases::{self, Valued};
use super::pob_minion::{self, PobMinion};
use super::pob_skill::{self, PobSkill};
use super::{Defence, Poe1Quantity, gem_levels::GemLevels, skill, states};
use crate::condition::{ConditionKey, Conditions, Flags};
use crate::engine::{Actor, ActorState, Calc};
use crate::error::CalcError;
use crate::ingest::{EffectTemplate, Ingest, StatMap, StatMapping};
use crate::input::CalcInput;
use crate::math;
use crate::quantity::{Bound, DamageType, Op, Quantity};
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

/// The stat that sets the minion's level to the enemy's (`SkillStatMap.lua`
/// 307, `minionLevelIsEnemyLevel`).
const LEVEL_IS_ENEMY_LEVEL: &str = "display_skill_minions_level_is_corpse_level";
/// The stat that sets the minion's level to the character's, at most its
/// value (`SkillStatMap.lua` 310, `minionLevelIsPlayerLevel`: PoB's
/// `mergeLevelMod` puts the stat's value in the skill data).
const LEVEL_IS_PLAYER_LEVEL: &str = "minion_actor_level_is_user_level_up_to_maximum";

/// What the minion's level reads besides the gem's stats (P6, gem level
/// data).
#[derive(Debug, Clone, Copy)]
struct LevelSources {
    /// The character's level (PoB's `build.characterLevel`, 1 to 100).
    character: i64,
    /// The enemy's level, `None` when the enemy model has a problem.
    enemy: Option<i64>,
    /// The gem level's character level requirement (data schema 5; 0 when
    /// the bundle is older or the level has none).
    requirement: i64,
}

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
    let spectre = SPECTRE_SKILLS.contains(&skill.effect().id.as_str());
    let library = spectre_library(build);
    let own: &[&str] = if spectre {
        &library
    } else {
        pob.map_or(&[], |p| p.minions)
    };
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
        Chosen {
            name,
            own: own.contains(&name),
            spectre,
        },
    ))
}

/// The minion picked ([`minion`]).
#[derive(Debug, Clone, Copy)]
struct Chosen<'a> {
    /// PoB's name of it: a short name, or a spectre's variety ID.
    name: &'a str,
    /// Whether it is from the skill's own list (not one a support adds).
    own: bool,
    /// Whether it is a spectre.
    spectre: bool,
}

/// The passive stats that give your auras' allies something beside the
/// aura (Commander of Darkness: PoB's `ExtraAura` and `ExtraAuraEffect`
/// lists, `CalcPerform.lua`, "Check for extra auras"): the minion takes
/// them in PoB, and the engine gives them to the character only.
const ALLY_AURA_STATS: [&str; 3] = [
    "auras_grant_attack_and_cast_speed_+%_to_you_and_your_allies",
    "damage_+%_for_you_and_allies_affected_by_your_auras",
    "elemental_resistances_+%_for_you_and_allies_affected_by_your_auras",
];

/// A reason on the minion for each allocated passive with one of
/// [`ALLY_AURA_STATS`] (RV-12: found on the spectre fixture, where PoB's
/// minion has Commander of Darkness' 30% increased damage and 3%
/// increased speed and the engine's had neither, without a word).
fn ally_aura_stats(input: &CalcInput<'_>, ingest: &mut Ingest<'_>) {
    let Some(spec) = super::collect::active_spec(input.build) else {
        return;
    };
    for node in spec.nodes.value() {
        let has = super::collect::node_stats(input, node.0).is_some_and(|stats| {
            stats
                .iter()
                .any(|s| ALLY_AURA_STATS.contains(&s.stat.as_str()))
        });
        if has {
            ingest.reason(UnsupportedReason::UnmodelledSkill {
                source: Source::Passive { node: *node },
                what: "minion-buffs",
            });
        }
    }
}

/// The config check that puts the minions on full life.
pub const FULL_LIFE_INPUT: &str = "minionsConditionFullLife";

/// Whether the active config set has [`FULL_LIFE_INPUT`] on.
fn full_life_input(build: &loom_domain::Build) -> bool {
    super::collect::active_config(build).is_some_and(|set| {
        matches!(
            super::config::last_input(set, FULL_LIFE_INPUT, loom_domain::ConfigInputKind::Input),
            Some(loom_domain::ConfigValue::Boolean(true))
        )
    })
}

/// Whether the minion model reads `input` ([`FULL_LIFE_INPUT`], a check).
#[must_use]
pub fn is_input(input: &str, value: &loom_domain::ConfigValue) -> bool {
    input == FULL_LIFE_INPUT && matches!(value, loom_domain::ConfigValue::Boolean(_))
}

/// The skills that raise a spectre: every skill of PoB v2.67.2 whose
/// `minionList` is there and empty (`Data/Skills/act_int.lua` 15008 and
/// 15108), so the build's spectre library takes its place
/// (`CalcActiveSkill.lua` 661 to 667). A test checks them against PoB's
/// skill table.
pub const SPECTRE_SKILLS: [&str; 2] = ["RaiseSpectre", "RaiseSpectreAltX"];

/// The gem stat a spectre's level comes from (PoB's stat map of Raise
/// Spectre, `Data/Skills/act_int.lua` 15011: `skill("minionLevel", nil)`).
pub const SPECTRE_LEVEL_STAT: &str = "raised_spectre_level";

/// The build's spectre library, in order (PoB's `build.spectreList`,
/// `Modules/Build.lua`, `Load`: each `Spectre` child of the `Build`
/// element with an `id`): the monster varieties the build's author picked
/// spectres from. A build file keeps them among the character's untyped
/// children.
#[must_use]
pub fn spectre_library(build: &loom_domain::Build) -> Vec<&str> {
    build
        .character
        .children
        .iter()
        .filter_map(|c| match c {
            loom_domain::OpaqueNode::Element {
                name, attributes, ..
            } if name == "Spectre" => skill::attribute(attributes, "id"),
            _ => None,
        })
        .collect()
}

/// The minion `name`'s state (module docs).
fn collect(
    input: &CalcInput<'_>,
    player: &ModStore,
    skill: &skill::MainSkill<'_>,
    pob: Option<&PobSkill>,
    source: &Source,
    chosen: Chosen<'_>,
) -> Result<ActorState, UnsupportedReason> {
    let Chosen { name, own, spectre } = chosen;
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
    let enemy = super::enemy::enemy(input.build, data);
    let sources = LevelSources {
        character: input
            .build
            .character
            .level
            .as_ref()
            .map_or(1, |l| i64::from(*l.value()))
            .clamp(1, 100),
        enemy: enemy.problems.is_empty().then_some(enemy.level),
        requirement: skill::effect_level(gem.effect, gem.level).map_or(0, |l| l.level_requirement),
    };
    let level = level(&instance.stats, pob, own, sources)
        .ok_or_else(|| not_computed(source, "minion-level"))?;

    let mapping = MinionMapping;
    let mut ingest = Ingest::new(&mapping);
    minion_modifiers(input, player, skill, &mut ingest);
    let totals = super::skill_modifiers::Totals::from_effects(player.effects());
    // A buff or aura effect modifier the totals do not read scales the
    // minion's buffs as the character's (P6-7b3b review).
    if totals.unread {
        ingest.reason(UnsupportedReason::UnmodelledSkill {
            source: Source::Rule("skill-modifiers"),
            what: "unread-skill-modifier",
        });
    }
    super::gems::minion_buffs(input, &levels_of(player), &totals, &mut ingest);
    ally_aura_stats(input, &mut ingest);
    gem_modifiers(input, skill, &mut ingest);
    base_values(input, variety, ty, level, spectre, &mut ingest)
        .ok_or_else(|| not_computed(source, "minion-data"))?;
    mod_list(input, minions, variety, &mut ingest);
    let mut states = std::collections::BTreeMap::new();
    template_mods(template, &mut ingest, &mut states);
    // The minion's own hit (P6-7c, [`super::minion_hit`]): its main skill,
    // its monster weapon, and PoB's offence mods of `initMinionModDB`.
    let own_skill = minion_skill(input, skill, variety, template);
    if own_skill.is_some() {
        weapon(input, variety, ty, level, spectre, &mut ingest);
        support_modifiers(input, skill, &mut ingest);
    }
    // A computed minion skill's own stats (P6-7c3).
    if let Some(id) = own_skill.as_deref().filter(|id| {
        super::minion_hit::VETTED_SPELLS.contains(id) || super::minion_hit::VETTED.contains(id)
    }) {
        own_skill_stats(input, id, level, &mut ingest);
    }
    ingest.effect(base(
        Quantity::Poe1(Poe1Quantity::CannotBeEvaded),
        Op::Flag,
        1.0,
    ));
    // The enemy is shared (PoB's one `enemyDB`): its modifiers and the
    // configured enemy conditions reach the minion's hits too.
    enemy_effects(input, player, &mut ingest);
    // Every enemy state the rules settle (P6-7c review), not only the
    // config's checks.
    for (key, value) in states::states(input) {
        if key.as_str().starts_with(ENEMY_STATE) {
            states.insert(key, value);
        }
    }
    // "Are your Minions always on Full Life?" (`ConfigOptions.lua` 198:
    // a `MinionModifier` of `Condition:FullLife`; RV-12).
    if full_life_input(input.build) {
        states.insert(ConditionKey::new(super::conditions::FULL_LIFE), true);
    }
    // The minion holds no weapon (PoB's `weaponData2` is empty without
    // items): it never dual wields.
    states.insert(ConditionKey::new(states::DUAL_WIELDING), false);
    let (store, global) = ingest.finish();
    Ok(ActorState {
        store,
        global,
        states,
        skill_sources: std::collections::BTreeSet::new(),
        skill: own_skill,
        weapon: variety
            .main_hand
            .as_deref()
            .and_then(weapon_class)
            .map(str::to_owned),
    })
}

/// The weapon class of a minion's main hand as PoB's exporter keeps it
/// (`itemClassMap`, `Export/Scripts/minions.lua` 76 to 93, then
/// `weaponTypeInfo`): the item class whose flags
/// [`super::hit::weapon_type_flags`] gives; `None` (unarmed) for a class
/// the map leaves out or maps to `None`.
fn weapon_class(game_class: &str) -> Option<&'static str> {
    Some(match game_class {
        "Claw" => "Claw",
        "Dagger" => "Dagger",
        "Wand" => "Wand",
        "One Hand Sword" | "Thrusting One Hand Sword" => "One Hand Sword",
        "One Hand Axe" => "One Hand Axe",
        "One Hand Mace" | "Sceptre" => "One Hand Mace",
        "Bow" => "Bow",
        "Fishing Rod" => "FishingRod",
        "Staff" => "Staff",
        "Two Hand Sword" => "Two Hand Sword",
        "Two Hand Axe" => "Two Hand Axe",
        "Two Hand Mace" => "Two Hand Mace",
        "Shield" => "Shield",
        _ => return None,
    })
}

/// PoB's supports that never reach a minion's skill (`calcLib.canGrantedEffectSupportActiveSkill`, `CalcTools.lua` 113 to 130): a trigger support (`isTrigger`: the minion is not the player), or one that names weapon types (the minion skills have none). From `Data/Skills/sup_*.lua` (P6-7c2), sorted.
pub const NO_MINION_SUPPORTS: &[&str] = &[
    "SupportArrowNova",
    "SupportAwakenedArrowNova",
    "SupportBallistaTotem",
    "SupportBarrage",
    "SupportCastOnCritTriggered",
    "SupportCastOnCritTriggeredPlus",
    "SupportCastOnDeath",
    "SupportCastOnMeleeKillTriggered",
    "SupportCastOnWardBreak",
    "SupportCastWhenDamageTaken",
    "SupportCastWhenStunned",
    "SupportCastWhileChannellingTriggered",
    "SupportCastWhileChannellingTriggeredPlus",
    "SupportCloseCombat",
    "SupportCrystalfall",
    "SupportCurseOnHitCurse",
    "SupportCurseOnHitCursePlus",
    "SupportGreaterKineticInstability",
    "SupportKineticInstability",
    "SupportManaforgedArrows",
    "SupportMarkOnHit",
    "SupportNightblade",
    "SupportSacredWisps",
    "SupportShockwave",
    "SupportTrauma",
    "SupportVoidShockwave",
    "SupportVoidstorm",
];

/// PoB's `notMinionStat` lists of supports (`Modules/Data.lua` 81 to 90: a minion's skill does not read them), from `Data/Skills/sup_*.lua` (P6-7c2), sorted by support.
pub const NOT_MINION_STATS: &[(&str, &[&str])] = &[
    (
        "SupportAwakenedCastWhileChannelling",
        &["cast_while_channelling_time_ms"],
    ),
    (
        "SupportAwakenedSpellCascade",
        &[
            "support_spell_cascade_area_of_effect_+%_final",
            "support_spell_cascade_damage_+%_final",
        ],
    ),
    (
        "SupportAwakenedUnleash",
        &[
            "support_anticipation_rapid_fire_count",
            "support_anticipation_charge_gain_interval_ms",
        ],
    ),
    ("SupportBallistaTotem", &["totem_support_gem_level"]),
    (
        "SupportBloodsoakedBanner",
        &[
            "support_bloodstained_banner_corrupted_blood_base_physical_damage_to_deal_per_minute_per_valour",
        ],
    ),
    (
        "SupportCastWhenDamageTaken",
        &["cast_on_damage_taken_threshold"],
    ),
    ("SupportCastWhenStunned", &["cast_on_stunned_%"]),
    (
        "SupportCastWhileChannelling",
        &["cast_while_channelling_time_ms"],
    ),
    ("SupportCooldownRecovery", &["base_cooldown_speed_+%"]),
    ("SupportCursedGround", &["base_hex_zone_skill_duration_ms"]),
    (
        "SupportFocusedBallista",
        &[
            "support_focused_ballista_totem_attack_speed_+%_final",
            "support_focused_ballista_totem_damage_+%_final",
        ],
    ),
    (
        "SupportFocusedChannelling",
        &["support_focus_channel_damage_+%_final_per_second_channelling_up_to_60%"],
    ),
    (
        "SupportFrostmage",
        &["frostmage_gain_cold_damage_%_of_max_reserved_mana"],
    ),
    (
        "SupportGluttony",
        &["support_gluttony_fire_x_additional_projectiles_in_nova_while_affected_by_disgorge"],
    ),
    (
        "SupportGreaterAncestralCall",
        &["extra_target_targeting_distance_+%"],
    ),
    (
        "SupportGreaterSpellCascade",
        &[
            "support_spell_cascade_area_of_effect_+%_final",
            "support_spell_cascade_damage_+%_final",
        ],
    ),
    (
        "SupportGreaterUnleash",
        &["support_anticipation_charge_gain_interval_ms"],
    ),
    (
        "SupportHextoad",
        &[
            "base_hex_zone_skill_duration_ms",
            "hex_zone_trigger_hextoad_every_x_ms",
        ],
    ),
    (
        "SupportInfusedChannelling",
        &["support_storm_barrier_damage_+%_final"],
    ),
    (
        "SupportInspiration",
        &[
            "support_inspiration_mana_cost_+%_final",
            "lose_all_righteous_charges_on_mana_use_threshold",
            "elemental_damage_+%_final_per_righteous_charge",
            "critical_strike_chance_+%_per_righteous_charge",
        ],
    ),
    (
        "SupportIntensify",
        &["support_spell_boost_area_damage_+%_final_per_charge"],
    ),
    (
        "SupportLifetap",
        &[
            "support_lifetap_damage_+%_final_while_buffed",
            "support_lifetap_spent_life_threshold",
        ],
    ),
    (
        "SupportMachinations",
        &["physical_damage_%_to_add_as_chaos_per_keystone"],
    ),
    (
        "SupportOverheat",
        &["support_overheat_ailment_damage_+%_final_per_3_fortification_consumed"],
    ),
    (
        "SupportOverloadedIntensity",
        &[
            "additional_critical_strike_chance_per_overloaded_intensity",
            "critical_strike_multiplier_+_per_overloaded_intensity",
        ],
    ),
    (
        "SupportPinpoint",
        &["support_greater_projectile_intensity_projectile_damage_+%_final"],
    ),
    (
        "SupportSacrifice",
        &[
            "support_sacrifice_sacrifice_%_of_current_life",
            "support_sacrifice_gain_%_of_sacrificed_life_as_added_chaos_damage",
        ],
    ),
    ("SupportSecondWind", &["base_cooldown_speed_+%"]),
    (
        "SupportSpellCascade",
        &[
            "support_spell_cascade_area_of_effect_+%_final",
            "support_spell_cascade_damage_+%_final",
        ],
    ),
    ("SupportSpellTotem", &["totem_support_gem_level"]),
    (
        "SupportSpellblade",
        &[
            "support_spell_weapon_damage_gain_%_of_one_hand_melee_weapon_damage_as_added_spell_damage",
        ],
    ),
    (
        "SupportTransfusion",
        &["random_nearby_minion_gain_a_soul_eater_soul_on_X_life_spent"],
    ),
    (
        "SupportTrauma",
        &[
            "trauma_strike_self_damage_per_trauma",
            "attack_minimum_added_physical_damage_with_weapons_per_trauma",
            "attack_maximum_added_physical_damage_with_weapons_per_trauma",
            "support_trauma_stun_duration_+%_per_trauma",
            "support_trauma_base_duration_ms",
        ],
    ),
    (
        "SupportTrinity",
        &["damage_penetrates_%_elemental_resistances_while_all_resonance_is_25"],
    ),
    (
        "SupportUnholyTrinity",
        &[
            "damage_+%_final_per_10_lowest_unholy_resonance",
            "hits_ignore_enemy_monster_physical_damage_reduction_%_chance_while_all_unholy_resonance_is_25",
            "hits_ignore_enemy_monster_lightning_and_chaos_resistance_%_chance_while_all_unholy_resonance_is_25",
        ],
    ),
    (
        "SupportUnleash",
        &["support_anticipation_charge_gain_interval_ms"],
    ),
];

/// The summoning skill's supports on the minion's skills (P6-7c2;
/// `CalcActiveSkill.lua` 903: `createActiveSkill(activeEffect,
/// activeSkill.supportList, minion, ...)`): PoB checks a support against
/// the summoning skill's types and its minion types (`CalcTools.lua` 104 to
/// 105), as the summoning skill's own supports already are, so each of
/// them reaches the minion's skill but those of [`NO_MINION_SUPPORTS`].
/// Its stats go into the minion skill's list through the gem mapping
/// ([`SupportOnMinion`]): its minion modifiers there apply to the minion's
/// own minions (none), so they are left to [`gem_modifiers`], and its
/// `notMinionStat` stats ([`NOT_MINION_STATS`]) are not read.
fn support_modifiers(
    input: &CalcInput<'_>,
    summon: &skill::MainSkill<'_>,
    ingest: &mut Ingest<'_>,
) {
    let group = summon.group.saturating_sub(1);
    for r in &summon.supports {
        let id = r.effect.id.as_str();
        if NO_MINION_SUPPORTS.binary_search(&id).is_ok() {
            continue;
        }
        let instance = skill::instance(r.effect, r.level, r.quality, input.data);
        let source = Source::Gem { group, gem: r.gem };
        for p in &instance.problems {
            ingest.reason(UnsupportedReason::UnmodelledSkill {
                source: source.clone(),
                what: p.name(),
            });
        }
        let lines: Vec<StatLine> = instance
            .stats
            .iter()
            .map(|(stat, value)| StatLine {
                stat: StatId::new(stat.as_str()),
                value: StatValue::Exact {
                    min: *value,
                    max: *value,
                },
            })
            .collect();
        ingest.stats_with(&SupportOnMinion { support: id }, &source, &lines);
    }
}

/// The minion skill's own stats (P6-7c3): PoB builds the skill at level 1
/// with the minion as its actor, so an effectiveness-scaled stat (its base
/// damage) is PoB's value at the minion's level
/// ([`skill::instance_for_actor`]). They join the skill's list
/// ([`super::minion_hit::SKILL_RULE`]) through the mapping a support's
/// stats take on a minion's skill ([`SupportOnMinion`]; a skill has no
/// `notMinionStat` list PoB reads, `Modules/Data.lua` 82).
fn own_skill_stats(input: &CalcInput<'_>, id: &str, level: i64, ingest: &mut Ingest<'_>) {
    let source = Source::Rule(super::minion_hit::SKILL_RULE);
    let Some(effect) = input.data.granted_effect(id) else {
        ingest.reason(not_computed(&source, "minion-skill"));
        return;
    };
    let instance = skill::instance_for_actor(effect, 1, level, input.data);
    for p in &instance.problems {
        ingest.reason(UnsupportedReason::UnmodelledSkill {
            source: source.clone(),
            what: p.name(),
        });
    }
    let lines: Vec<StatLine> = instance
        .stats
        .iter()
        .map(|(stat, value)| StatLine {
            stat: StatId::new(stat.as_str()),
            value: StatValue::Exact {
                min: *value,
                max: *value,
            },
        })
        .collect();
    ingest.stats_with(&OwnSkillMapping { skill: id }, &source, &lines);
}

/// The mapping of a minion skill's own stats ([`own_skill_stats`]): PoB's
/// own stat map of the skill ([`super::minion_hit::OWN_STATS`], P6-7c3c),
/// then the mapping a support's stats take on a minion's skill.
struct OwnSkillMapping<'a> {
    skill: &'a str,
}

impl StatMapping for OwnSkillMapping<'_> {
    fn map(&self, stat: &StatId) -> StatMap {
        match super::minion_hit::own_stat(self.skill, stat.as_str()) {
            Some(template) => StatMap::Effects(vec![template]),
            None => SupportOnMinion {
                support: self.skill,
            }
            .map(stat),
        }
    }
}

/// The gem mapping of a support's stats on a minion's skill
/// ([`support_modifiers`]), and of the minion skill's own
/// ([`own_skill_stats`]).
struct SupportOnMinion<'a> {
    support: &'a str,
}

impl StatMapping for SupportOnMinion<'_> {
    fn map(&self, stat: &StatId) -> StatMap {
        let not_minion = NOT_MINION_STATS
            .binary_search_by(|(id, _)| (*id).cmp(self.support))
            .ok()
            .and_then(|i| NOT_MINION_STATS.get(i))
            .is_some_and(|(_, stats)| stats.contains(&stat.as_str()));
        if not_minion {
            return StatMap::NoEffect;
        }
        match SkillMapping.map(stat) {
            StatMap::Effects(templates) => {
                let kept: Vec<EffectTemplate> = templates
                    .into_iter()
                    .filter(|t| {
                        // The enemy's quantities come with the enemy's
                        // effects ([`enemy_effects`]): never twice.
                        !is_enemy_quantity(t.quantity)
                            && !matches!(
                                t.quantity,
                                Quantity::Poe1(
                                    Poe1Quantity::Minion(_)
                                        | Poe1Quantity::GolemMinion(_)
                                        | Poe1Quantity::MinionToYou(..)
                                )
                            )
                    })
                    .collect();
                if kept.is_empty() {
                    StatMap::NoEffect
                } else {
                    StatMap::Effects(kept)
                }
            }
            other => other,
        }
    }
}

/// The prefix of the enemy's conditions ([`super::conditions`]).
const ENEMY_STATE: &str = "Enemy:";

/// The source of the minion's monster weapon (P6-7c).
pub const WEAPON_RULE: &str = "minion-weapon";

/// The minion's main skill (`CalcActiveSkill.lua` 873 to 922): its skill
/// list (the variety's skills, then the template's, each PoB's data has),
/// the gem's `skillMinionSkill` (1-based, clamped) or the first; PoB's
/// default attack when the list is empty.
fn minion_skill(
    input: &CalcInput<'_>,
    summon: &skill::MainSkill<'_>,
    variety: &MinionVariety,
    template: &PobMinion,
) -> Option<String> {
    let mut list: Vec<&str> = variety
        .skills
        .iter()
        .map(String::as_str)
        .chain(template.skills.iter().copied())
        .filter(|id| pob_skill::skill(id).is_some())
        .collect();
    // PoB's `ExtraMinionSkill` from the summoning skill's gems' stat maps,
    // appended in order (`CalcActiveSkill.lua` 880 to 884; P6-7c review).
    // From the character's lines it comes through stats the engine does
    // not map: the character's reasons, which the minion's fields carry.
    for r in std::iter::once(&summon.active_gem).chain(&summon.supports) {
        let instance = skill::instance(r.effect, r.level, r.quality, input.data);
        for (stat, value) in &instance.stats {
            if *value == 0 {
                continue;
            }
            if let Some(&(_, extra)) = EXTRA_MINION_SKILLS.iter().find(|(s, _)| s == stat) {
                list.push(extra);
            }
        }
    }
    if list.is_empty() {
        return Some("Melee".to_owned());
    }
    let wanted = skill::active_groups(input.build)
        .ok()
        .and_then(|groups| groups.get(summon.group.checked_sub(1)?).copied())
        .and_then(|g| g.gems.typed().nth(summon.active_gem.gem))
        .and_then(|g| skill::attribute(&g.attributes, "skillMinionSkill"))
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1);
    let index = wanted.clamp(1, list.len()) - 1;
    list.get(index).map(|s| (*s).to_owned())
}

/// The gem stats whose PoB stat map adds an `ExtraMinionSkill`, and the
/// skill (`Data/Skills/sup_int.lua` 3822 to 3825, Infernal Legion;
/// `Data/Skills/other.lua` 4697 to 4700, the Radiant Sentinel).
const EXTRA_MINION_SKILLS: [(&str, &str); 2] = [
    (
        "radiant_sentinel_minion_fire_%_of_life_to_deal_nearby_per_minute",
        "GuardianSentinelFireAura",
    ),
    (
        "support_minion_instability_minion_base_fire_area_damage_per_minute",
        "InfernalLegion",
    ),
];

/// PoB's `damageFixup` of the exporter (`minions.lua` 185 to 191): a
/// variety mod's ID and its fixup.
const DAMAGE_FIXUPS: [(&str, f64); 3] = [
    ("MonsterSpeedAndDamageFixupSmall", 0.11),
    ("MonsterSpeedAndDamageFixupLarge", 0.22),
    ("MonsterSpeedAndDamageFixupComplete", 0.33),
];

/// The minion's monster weapon (`CalcActiveSkill.lua` 719 to 752, module
/// docs of [`super::minion_hit`]), as the weapon quantities an item gives
/// the character, and PoB's `damageFixup` (488 to 490: `Damage` MORE
/// `-100 x fixup` and `Speed` MORE `100 x fixup` for attacks). The
/// multipliers are the numbers PoB's exporter wrote ([`lua_number`]); the
/// table's value is the game's single-precision number, as PoB's.
fn weapon(
    input: &CalcInput<'_>,
    variety: &MinionVariety,
    ty: &MinionType,
    level: i64,
    spectre: bool,
    ingest: &mut Ingest<'_>,
) {
    use super::{Hand, WeaponValue};
    let source = Source::Rule(WEAPON_RULE);
    let effect = |quantity, op, value, all| Effect {
        quantity,
        op,
        value,
        conditions: Conditions::none().with_all(all),
        source: source.clone(),
    };
    // A spectre's damage is a monster's, not an ally's (`CalcActiveSkill.lua`
    // 720).
    let table = if spectre {
        input.data.monster_damage(level)
    } else {
        input.data.monster_ally_damage(level)
    };
    let Some(table) = table else {
        ingest.reason(UnsupportedReason::UnmodelledSkill {
            source: source.clone(),
            what: "minion-weapon",
        });
        return;
    };
    #[expect(clippy::cast_precision_loss, reason = "game integers, exact as f64")]
    let f = |v: i64| v as f64;
    let damage_multiplier = lua_number(f(variety.damage) / 100.0);
    let attack_time = lua_number(f(variety.attack_time) / 1000.0);
    let spread = lua_number(f(ty.damage_spread) / 100.0);
    let mut damage = table * damage_multiplier;
    if !ty.damage_ignores_attack_speed {
        damage *= attack_time;
    }
    let weapon = |v| Hand::Main.weapon(v);
    let none = crate::condition::Flags::NONE;
    for (bound, value) in [
        (
            crate::quantity::Bound::Min,
            math::round_half_up(damage * (1.0 - spread)),
        ),
        (
            crate::quantity::Bound::Max,
            math::round_half_up(damage * (1.0 + spread)),
        ),
    ] {
        if value != 0.0 {
            ingest.effect(effect(
                weapon(WeaponValue::Damage(DamageType::Physical, bound)),
                Op::Base,
                value,
                none,
            ));
        }
    }
    if attack_time > 0.0 {
        ingest.effect(effect(
            weapon(WeaponValue::AttackRate),
            Op::Base,
            1.0 / attack_time,
            none,
        ));
    }
    ingest.effect(effect(weapon(WeaponValue::CritChance), Op::Base, 5.0, none));
    for (id, fixup) in DAMAGE_FIXUPS {
        if variety.mods.iter().any(|m| m == id) {
            let attack = crate::condition::Flags::ATTACK;
            ingest.effect(effect(Quantity::Damage, Op::More, -100.0 * fixup, attack));
            ingest.effect(effect(Quantity::Speed, Op::More, 100.0 * fixup, attack));
        }
    }
}

/// The enemy's modifiers the character's skills and the config gave (the
/// enemy is shared: PoB's curses, debuffs, exposure, and configured enemy
/// conditions reach the minion's hits; P6-7c): every effect on an enemy
/// quantity, and every effect of a curse or debuff skill (a curse's rows
/// on the character's added damage and crit are the enemy's `Self...` mods
/// in PoB, [`super::gems`]), copied with their sources.
fn enemy_effects(input: &CalcInput<'_>, player: &ModStore, ingest: &mut Ingest<'_>) {
    let groups = skill::active_groups(input.build).ok();
    let enemy_skill = |source: &Source| -> bool {
        let Source::Gem { group, gem } = source else {
            return false;
        };
        let Some(g) = groups.as_ref().and_then(|gs| gs.get(*group)) else {
            return false;
        };
        let Some(gem) = g.gems.typed().nth(*gem) else {
            return false;
        };
        skill::gem_effects(gem, input.data)
            .unwrap_or_default()
            .iter()
            .any(|e| {
                matches!(
                    super::gems::kind(&e.id),
                    Some(super::gems::Kind::Curse { .. } | super::gems::Kind::Debuff { .. })
                )
            })
    };
    for e in player.effects() {
        if is_enemy_quantity(e.quantity) || enemy_skill(&e.source) {
            ingest.effect(e.clone());
        }
    }
}

/// A quantity of the enemy's (the shared enemy, P6-7c).
const fn is_enemy_quantity(q: Quantity) -> bool {
    matches!(
        q,
        Quantity::Poe1(
            Poe1Quantity::EnemyDamageTaken
                | Poe1Quantity::EnemyTypeDamageTaken(_)
                | Poe1Quantity::EnemyResistance(_)
                | Poe1Quantity::SkillExposure(_)
                | Poe1Quantity::EnemyEvasion
                | Poe1Quantity::EnemyAilmentDuration
                | Poe1Quantity::EnemyBuffExpireFaster
                | Poe1Quantity::EnemyPhysicalDamageReduction
                | Poe1Quantity::EnemyProjectileDamageTaken
                | Poe1Quantity::EnemyTrapMineDamageTaken
                | Poe1Quantity::EnemyProjectileAttackDamageTaken
                | Poe1Quantity::ChillDamageTaken(_)
                | Poe1Quantity::ConsecratedGroundDamageTaken
                | Poe1Quantity::ChillSecondsDamageTaken
        )
    )
}

/// The minion's level (module docs; `CalcActiveSkill.lua` 707 to 711),
/// clamped to 1 to 100, in PoB's order: the enemy's level with
/// [`LEVEL_IS_ENEMY_LEVEL`]; the character's, at most the stat's value,
/// with [`LEVEL_IS_PLAYER_LEVEL`]; PoB's `minionLevel`; else the gem
/// level's. For the last the game's [`LEVEL_STAT`] comes first and the
/// level requirement (data schema 5) after: they differ only at Vaal
/// Summon Skeletons, where the game's stat is lower (DIV-0111 to
/// DIV-0113, DIV-0174: the in-game check is open). `None` when the level
/// comes from elsewhere (a support's minion) or is not known.
fn level(
    stats: &[(String, i64)],
    pob: Option<&PobSkill>,
    own: bool,
    sources: LevelSources,
) -> Option<i64> {
    let get = |id: &str| stats.iter().find(|(s, _)| s == id).map(|(_, v)| *v);
    if !own {
        return None;
    }
    if get(LEVEL_IS_ENEMY_LEVEL).is_some_and(|v| v != 0) {
        return sources.enemy.map(|l| l.clamp(1, 100));
    }
    if let Some(cap) = get(LEVEL_IS_PLAYER_LEVEL).filter(|v| *v != 0) {
        return Some(sources.character.min(cap).clamp(1, 100));
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "PoB's minion levels are small integers"
    )]
    let from_data = pob
        .and_then(|p| p.skill_data("minionLevel"))
        .and_then(pob_skill::Value::num)
        .map(|n| n as i64);
    let requirement = (sources.requirement > 0).then_some(sources.requirement);
    let level = from_data
        .or_else(|| get(SPECTRE_LEVEL_STAT))
        .or_else(|| get(LEVEL_STAT))
        .or(requirement)?;
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
    spectre: bool,
) -> Option<BaseNumbers> {
    #[expect(clippy::cast_precision_loss, reason = "game integers, exact as f64")]
    let f = |v: i64| v as f64;
    let life_mult = lua_number(f(life) / 100.0);
    let table = match ty.life_scaling() {
        Some(LifeScaling::AltLife1) => lives.alt1?,
        Some(LifeScaling::AltLife2) => lives.alt2,
        // A spectre is a monster (`CalcActiveSkill.lua` 717).
        None if spectre => lives.life,
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
    spectre: bool,
    ingest: &mut Ingest<'_>,
) -> Option<()> {
    let data = input.data;
    let n = base_numbers(
        variety.life,
        ty,
        data.monster_life(level)?,
        super::enemy::monster_armour(data, level)?,
        data.monster_level(level)?.evasion,
        spectre,
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

/// No flag on a minion stat's effect.
const NO_FLAGS: Flags = Flags::NONE;

/// How a monster stat reads on a minion ([`MinionMapping`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MinionStat {
    /// An effect a minion field reads: its quantity, op, the flags of PoB's
    /// mod (P6-7c: the minion's hit reads them), and the state it needs.
    Effect(Quantity, Op, Flags, Option<&'static str>),
    /// PoB maps it to a mod no minion field reads (damage taken by the
    /// minion, immunities, stun, leech, regeneration, charges).
    Later,
}

/// The stats PoB's exporter turns into mods on the skills' minions (the
/// skill stat map; `Data/Minions.lua`'s `mod(...)` lines), sorted, and how
/// they read; every other stat is a comment in PoB, with no effect.
pub const MINION_STATS: [(&str, MinionStat); 29] = [
    // P6-7c, the minion's hit (PoB `SkillStatMap.lua`): `Speed` MORE with
    // `ModFlag.Attack` (1567).
    (
        "active_skill_attack_speed_+%_final",
        MinionStat::Effect(Quantity::Speed, Op::More, Flags::ATTACK, None),
    ),
    // `PhysicalMax` BASE with `KeywordFlag.Attack` (1600), as the
    // character's row.
    (
        "attack_maximum_added_physical_damage",
        MinionStat::Effect(
            Quantity::AddedDamage(DamageType::Physical, Bound::Max),
            Op::Base,
            Flags::ATTACK,
            None,
        ),
    ),
    (
        "attack_minimum_added_physical_damage",
        MinionStat::Effect(
            Quantity::AddedDamage(DamageType::Physical, Bound::Min),
            Op::Base,
            Flags::ATTACK,
            None,
        ),
    ),
    ("base_block_%_damage_taken", MinionStat::Later),
    ("base_cannot_be_damaged", MinionStat::Later),
    ("base_cannot_be_stunned", MinionStat::Later),
    (
        "base_maximum_energy_shield",
        MinionStat::Effect(Quantity::EnergyShield, Op::Base, NO_FLAGS, None),
    ),
    // `PhysicalDamageConvertToChaos` BASE.
    (
        "base_physical_damage_%_to_convert_to_chaos",
        MinionStat::Effect(
            Quantity::Poe1(Poe1Quantity::Conversion(
                DamageType::Physical,
                DamageType::Chaos,
            )),
            Op::Base,
            NO_FLAGS,
            None,
        ),
    ),
    // `PhysicalDamageConvertToCold` BASE.
    (
        "base_physical_damage_%_to_convert_to_cold",
        MinionStat::Effect(
            Quantity::Poe1(Poe1Quantity::Conversion(
                DamageType::Physical,
                DamageType::Cold,
            )),
            Op::Base,
            NO_FLAGS,
            None,
        ),
    ),
    // `PhysicalDamageConvertToFire` BASE.
    (
        "base_physical_damage_%_to_convert_to_fire",
        MinionStat::Effect(
            Quantity::Poe1(Poe1Quantity::Conversion(
                DamageType::Physical,
                DamageType::Fire,
            )),
            Op::Base,
            NO_FLAGS,
            None,
        ),
    ),
    (
        "block_while_dual_wielding_%",
        MinionStat::Effect(
            Quantity::BlockChance,
            Op::Base,
            NO_FLAGS,
            Some(states::DUAL_WIELDING),
        ),
    ),
    ("cannot_be_knocked_back", MinionStat::Later),
    // `Speed` MORE with `ModFlag.Attack` while dual wielding (1570): the
    // minion never dual wields.
    (
        "dual_wield_inherent_attack_speed_+%_final",
        MinionStat::Effect(
            Quantity::Speed,
            Op::More,
            Flags::ATTACK,
            Some(states::DUAL_WIELDING),
        ),
    ),
    (
        "life_leech_from_physical_attack_damage_permyriad",
        MinionStat::Later,
    ),
    ("life_regeneration_rate_per_minute_%", MinionStat::Later),
    // `PhysicalMax` BASE with `ModFlag.Weapon` and `KeywordFlag.Attack`
    // (1609): a minion with a weapon type.
    (
        "main_hand_local_maximum_added_physical_damage",
        MinionStat::Effect(
            Quantity::AddedDamage(DamageType::Physical, Bound::Max),
            Op::Base,
            Flags::WEAPON.union(Flags::ATTACK),
            None,
        ),
    ),
    // `PhysicalMin` BASE with `ModFlag.Weapon` and `KeywordFlag.Attack`
    // (1609): a minion with a weapon type.
    (
        "main_hand_local_minimum_added_physical_damage",
        MinionStat::Effect(
            Quantity::AddedDamage(DamageType::Physical, Bound::Min),
            Op::Base,
            Flags::WEAPON.union(Flags::ATTACK),
            None,
        ),
    ),
    (
        "monster_base_block_%",
        MinionStat::Effect(Quantity::BlockChance, Op::Base, NO_FLAGS, None),
    ),
    ("monster_inherent_damage_taken_+%_final", MinionStat::Later),
    (
        "physical_damage_reduction_rating_+%",
        MinionStat::Effect(Quantity::Armour, Op::Increased, NO_FLAGS, None),
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
        MinionStat::Effect(Quantity::Life, Op::Override, NO_FLAGS, None),
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
                Some(MinionStat::Effect(quantity, op, flags, state)) => {
                    let conditions = state.map_or_else(Conditions::none, |key| {
                        Conditions::none().with_state(ConditionKey::new(key), true)
                    });
                    let conditions = conditions.with_all(flags);
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

/// The character's minion modifiers on the minion (module docs): each
/// unconditional effect of a kind a minion field reads is added; a
/// conditional one is a `minion-modifier` reason (its condition would be
/// the character's, or the minion's, as PoB's tags say: not modelled).
fn minion_modifiers(
    input: &CalcInput<'_>,
    player: &ModStore,
    skill: &skill::MainSkill<'_>,
    ingest: &mut Ingest<'_>,
) {
    let golem = skill.types.contains("Golem");
    // The character's settled conditions a minion modifier's own condition
    // reads (P6-11cm). PoB's minion modifiers see the main skill's
    // conditions too: Summon Raging Spirit without the check gets "if
    // you've used a Minion Skill Recently" (fixture
    // `minion-summon-raging-spirit-used-minion-skill-witch.xml`).
    let settled = states::states(input);
    // An unsettled minion modifier of the character is the minion's reason
    // when the minion reads it.
    // Every one reaches the minion's reasons, those of kinds no minion field
    // reads yet too (regeneration, movement, cooldown: kept visible, ADR-015;
    // P6-7b3b review), but a golem one for a skill that is not a golem.
    for (quantities, reason) in player.elsewhere() {
        if quantities.iter().any(|q| applies_to(golem, *q)) {
            ingest.indeterminate(reason.clone());
        }
    }
    let main_group = skill.group.saturating_sub(1);
    for e in player.effects() {
        // The main skill's own gems: [`gem_modifiers`].
        if matches!(e.source, Source::Gem { group, .. } if group == main_group) {
            continue;
        }
        let Some((quantity, flags)) = on_minion(golem, e.quantity) else {
            continue;
        };
        // A condition of the character only (P6-11cm): settled, it applies
        // or not; any other condition, or an unsettled one, is a reason.
        let own_only = e.conditions.all.is_empty() && e.conditions.any.is_empty();
        let applies = if own_only {
            e.conditions
                .states
                .iter()
                .map(|(key, want)| settled.get(key).map(|v| v == want))
                .try_fold(true, |all, v| v.map(|v| all && v))
        } else {
            None
        };
        match applies {
            Some(true) => {}
            Some(false) => continue,
            None => {
                ingest.reason(UnsupportedReason::UnmodelledSkill {
                    source: e.source.clone(),
                    what: "minion-modifier",
                });
                continue;
            }
        }
        ingest.effect(Effect {
            quantity,
            op: e.op,
            value: e.value,
            conditions: Conditions::none().with_all(flags),
            source: e.source.clone(),
        });
    }
}

/// Whether a minion modifier of the character applies to this minion at
/// all (a golem one only for a golem skill), whether or not a minion field
/// reads its kind.
fn applies_to(golem: bool, quantity: Quantity) -> bool {
    match quantity {
        Quantity::Poe1(Poe1Quantity::Minion(_)) => true,
        Quantity::Poe1(Poe1Quantity::GolemMinion(_)) => golem,
        _ => false,
    }
}

/// The minion's quantity of a minion modifier of the character, with the
/// flags its effects take on, when the minion reads it: a
/// [`Poe1Quantity::GolemMinion`] one only for a golem skill (`golem`: the
/// main skill has the `Golem` type).
fn on_minion(golem: bool, quantity: Quantity) -> Option<(Quantity, crate::condition::Flags)> {
    match quantity {
        Quantity::Poe1(Poe1Quantity::Minion(v)) => v.on_minion(),
        Quantity::Poe1(Poe1Quantity::GolemMinion(v)) if golem => v.on_minion(),
        _ => None,
    }
}

/// How the gem mapping reads a stat PoB's stat map sends to the minion
/// ([`gem_modifiers`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GemMinionStat {
    /// As minion modifiers, applied.
    Applied,
    /// Not at all: an `unmapped-stat` reason on the minion.
    Unmapped,
    /// As no effect, or as the character's: lost on the minion, a
    /// `minion-modifier` reason.
    Lost,
}

/// How the gem mapping reads `stat` ([`GemMinionStat`]).
fn gem_minion_stat(stat: &str) -> GemMinionStat {
    match SkillMapping.map(&StatId::new(stat)) {
        StatMap::Effects(e)
            if e.iter()
                .all(|t| super::minions::is_minion_quantity(t.quantity)) =>
        {
            GemMinionStat::Applied
        }
        StatMap::Unmapped => GemMinionStat::Unmapped,
        _ => GemMinionStat::Lost,
    }
}

/// The main skill's own gems' minion modifiers (module docs): each stat
/// of the active gem and of the supports that apply that the gem mapping
/// reads as minion modifiers ([`on_minion`]), with the gem as source. A
/// stat PoB's stat map sends to the minion ([`PobSkill::minion_stats`])
/// that the gem mapping does not read is a reason ([`GemMinionStat`]).
fn gem_modifiers(input: &CalcInput<'_>, skill: &skill::MainSkill<'_>, ingest: &mut Ingest<'_>) {
    let golem = skill.types.contains("Golem");
    let group = skill.group.saturating_sub(1);
    for gem in std::iter::once(&skill.active_gem).chain(&skill.supports) {
        let source = Source::Gem {
            group,
            gem: gem.gem,
        };
        let pob = pob_skill::skill(&gem.effect.id);
        let instance = skill::instance(gem.effect, gem.level, gem.quality, input.data);
        for (stat, value) in &instance.stats {
            let to_minion = *value != 0
                && pob.is_some_and(|p| p.minion_stats.binary_search(&stat.as_str()).is_ok());
            match (to_minion, gem_minion_stat(stat)) {
                (true, GemMinionStat::Unmapped) => {
                    ingest.reason(UnsupportedReason::UnmappedStat {
                        stat: StatId::new(stat.as_str()),
                        source: source.clone(),
                    });
                    continue;
                }
                (true, GemMinionStat::Lost) => {
                    ingest.reason(UnsupportedReason::UnmodelledSkill {
                        source: source.clone(),
                        what: "minion-modifier",
                    });
                    continue;
                }
                _ => {}
            }
            let StatMap::Effects(templates) = SkillMapping.map(&StatId::new(stat.as_str())) else {
                continue;
            };
            for t in templates {
                let Some((quantity, flags)) = on_minion(golem, t.quantity) else {
                    continue;
                };
                // As for the character's (`minion_modifiers`): a
                // conditional minion modifier is a reason, not evaluated
                // against the minion's states (P6-7b3b review).
                if t.conditions != Conditions::none() {
                    ingest.reason(UnsupportedReason::UnmodelledSkill {
                        source: source.clone(),
                        what: "minion-modifier",
                    });
                    continue;
                }
                #[expect(clippy::cast_precision_loss, reason = "gem stat values are small")]
                let value = *value as f64 * t.scale / t.divisor;
                ingest.effect(Effect {
                    quantity,
                    op: t.op,
                    value,
                    conditions: Conditions::none().with_all(flags),
                    source: source.clone(),
                });
            }
        }
    }
}

/// The gem levels the character's effects give (as [`minion`] reads them).
fn levels_of(player: &ModStore) -> GemLevels {
    GemLevels::from_effects(player.effects())
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
        // "Minions gain #% of Maximum Life as Extra Maximum Energy Shield"
        // (RV-12; `CalcDefence.lua` 1023 to 1036, the character's row).
        let mut life_reasons = Reasons::new();
        let row = if matches!(defence, Defence::EnergyShield) {
            phases::life_as_energy_shield(calc, None, false, &mut life_reasons)?
        } else {
            None
        };
        let (value, mut reasons) = phases::primary_defence(calc, defence, quantity, None, row)?;
        if row.is_some() {
            reasons.extend(&life_reasons);
        }
        set(calc, field, value, reasons);
    }
    let block = phases::attack_block(calc, None, Reasons::new())?;
    set(calc, MinionField::BlockChance, block.value, block.reasons);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P6-7b3b: a minion modifier reaches the minion's quantity; a golem
    /// one only for a golem skill; a kind no minion field reads, and any
    /// other quantity, reach nothing.
    /// P6-7b3b review: an unsettled value of a kind no minion field reads
    /// still reaches the minion; a golem one only a golem.
    #[test]
    fn applies_to_should_keep_every_kind_but_golem_for_others() {
        let minion = |v| Quantity::Poe1(Poe1Quantity::Minion(v));
        let golem = |v| Quantity::Poe1(Poe1Quantity::GolemMinion(v));
        assert!(applies_to(false, minion(MinionValue::LifeRegen)));
        assert!(applies_to(false, minion(MinionValue::Life)));
        assert!(!applies_to(false, golem(MinionValue::Life)));
        assert!(applies_to(true, golem(MinionValue::Life)));
        assert!(!applies_to(true, Quantity::Life));
    }

    #[test]
    fn on_minion_should_filter_golem_modifiers() {
        let minion = |v| Quantity::Poe1(Poe1Quantity::Minion(v));
        let golem = |v| Quantity::Poe1(Poe1Quantity::GolemMinion(v));
        let none = crate::condition::Flags::NONE;
        assert_eq!(
            on_minion(false, minion(MinionValue::Life)),
            Some((Quantity::Life, none))
        );
        assert_eq!(on_minion(false, golem(MinionValue::Life)), None);
        assert_eq!(
            on_minion(true, golem(MinionValue::Life)),
            Some((Quantity::Life, none))
        );
        assert_eq!(
            on_minion(true, golem(MinionValue::Resistance(DamageType::Cold))),
            Some((Quantity::Resistance(DamageType::Cold), none))
        );
        // P6-7c: damage and speeds reach the minion's hit.
        assert_eq!(
            on_minion(true, minion(MinionValue::Damage)),
            Some((Quantity::Damage, none))
        );
        assert_eq!(
            on_minion(false, minion(MinionValue::AttackSpeed)),
            Some((Quantity::Speed, crate::condition::Flags::ATTACK))
        );
        assert_eq!(on_minion(true, minion(MinionValue::LifeRegen)), None);
        assert_eq!(on_minion(true, Quantity::Life), None);
    }

    /// P6-7b3b: a gem's minion stat the gem mapping reads as a minion
    /// modifier is applied; one it leaves unmapped, or reads as no effect,
    /// is a reason.
    #[test]
    fn gem_minion_stats_should_follow_the_gem_mapping() {
        assert_eq!(
            gem_minion_stat("active_skill_minion_life_+%_final"),
            GemMinionStat::Applied
        );
        assert_eq!(gem_minion_stat("minion_damage_+%"), GemMinionStat::Applied);
        // P6-11cs: a row since.
        assert_eq!(
            gem_minion_stat("minion_critical_strike_multiplier_+"),
            GemMinionStat::Applied
        );
        assert_eq!(
            gem_minion_stat("minion_grant_puppet_master_buff_to_parent_on_hit_%"),
            GemMinionStat::Unmapped
        );
        assert_eq!(
            gem_minion_stat("active_skill_area_of_effect_+%_final"),
            GemMinionStat::Lost
        );
    }

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
        let n = base_numbers(105, &ty(0, 0, 0, false, false), lives, 1001.0, 501, false).unwrap();
        assert_eq!(n.life, 420.0);
        assert_eq!(n.energy_shield, None);
        assert_eq!((n.armour, n.evasion), (1001.0, 501.0));
        // ES 50 (0.2): floor(400 x 1.05 x 0.2) = 84; armour x 0.5 rounds
        // half up; evasion x 0.33.
        let n = base_numbers(
            105,
            &ty(50, 50, 33, false, false),
            lives,
            1001.0,
            501,
            false,
        )
        .unwrap();
        assert_eq!(n.energy_shield, Some(84.0));
        assert_eq!(n.armour, 501.0);
        assert_eq!(n.evasion, 165.0);
        // AltLife1 and AltLife2 (the second wins); ES stays on the ally table.
        let n = base_numbers(100, &ty(50, 0, 0, true, false), lives, 1.0, 1, false).unwrap();
        assert_eq!((n.life, n.energy_shield), (300.0, Some(80.0)));
        let n = base_numbers(100, &ty(0, 0, 0, true, true), lives, 1.0, 1, false).unwrap();
        assert_eq!(n.life, 200.0);
        let no_alt1 = crate::input::MonsterLife {
            alt1: None,
            ..lives
        };
        assert_eq!(
            base_numbers(100, &ty(0, 0, 0, true, false), no_alt1, 1.0, 1, false),
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

    /// The level (`CalcActiveSkill.lua` 707 to 711): the enemy's with the
    /// corpse stat (unknown with an enemy problem), the character's at most
    /// the cap with the player-level stat, else `minionLevel` skill data,
    /// the game's level stat, then the level requirement (data schema 5);
    /// clamped; a support's minion is not known.
    #[test]
    fn the_level_should_follow_pobs_order() {
        let stats = |v: &[(&str, i64)]| -> Vec<(String, i64)> {
            v.iter().map(|(s, n)| ((*s).to_owned(), *n)).collect()
        };
        let src = LevelSources {
            character: 90,
            enemy: Some(83),
            requirement: 70,
        };
        let none = LevelSources {
            requirement: 0,
            ..src
        };
        assert_eq!(
            level(&stats(&[(LEVEL_STAT, 66)]), None, true, src),
            Some(66)
        );
        assert_eq!(
            level(&stats(&[(LEVEL_STAT, 120)]), None, true, src),
            Some(100)
        );
        assert_eq!(level(&stats(&[(LEVEL_STAT, 66)]), None, false, src), None);
        // The requirement when the game's stat is absent.
        assert_eq!(level(&stats(&[]), None, true, src), Some(70));
        assert_eq!(level(&stats(&[]), None, true, none), None);
        // The enemy's level with the corpse stat.
        let corpse = stats(&[(LEVEL_STAT, 66), (LEVEL_IS_ENEMY_LEVEL, 1)]);
        assert_eq!(level(&corpse, None, true, src), Some(83));
        let no_enemy = LevelSources { enemy: None, ..src };
        assert_eq!(level(&corpse, None, true, no_enemy), None);
        // The character's, at most the stat's value.
        let player = stats(&[(LEVEL_STAT, 66), (LEVEL_IS_PLAYER_LEVEL, 85)]);
        assert_eq!(level(&player, None, true, src), Some(85));
        let low = LevelSources {
            character: 40,
            ..src
        };
        assert_eq!(level(&player, None, true, low), Some(40));
    }

    /// PoB's exporter's class map (`minions.lua` 76 to 93): a class it
    /// leaves out is an empty hand.
    /// RV-12: the skills that raise a spectre are the ones whose minion
    /// list PoB's data leaves empty though they make minions.
    #[test]
    fn spectre_skills_should_be_pobs_skills_with_an_empty_minion_list() {
        for id in SPECTRE_SKILLS {
            let pob = pob_skill::skill(id).expect(id);
            assert!(pob.minions.is_empty(), "{id}");
        }
        // No other skill of PoB's table is named as a spectre skill by PoB
        // (`CalcPerform.lua` 2161: `^RaiseSpectre`).
        for skill in super::super::pob_skills::SKILLS {
            assert_eq!(
                skill.id.starts_with("RaiseSpectre"),
                SPECTRE_SKILLS.contains(&skill.id),
                "{}",
                skill.id
            );
        }
    }

    /// RV-12: the spectre library is the build file's `Spectre` entries,
    /// in order; anything else among the character's children is not one.
    #[test]
    fn the_spectre_library_should_be_the_builds_spectre_entries() {
        use loom_domain::{Attribute, Build, Game, OpaqueNode};
        let entry = |name: &str, id: &str| OpaqueNode::Element {
            name: name.to_owned(),
            attributes: vec![Attribute::new("id", id)],
            children: Vec::new(),
        };
        let mut b = Build::new(Game::Poe1);
        assert!(spectre_library(&b).is_empty());
        b.character.children = vec![
            entry("PlayerStat", "Life"),
            entry("Spectre", "Metadata/Monsters/A"),
            OpaqueNode::Text("x".to_owned()),
            entry("Spectre", "Metadata/Monsters/B"),
        ];
        assert_eq!(
            spectre_library(&b),
            ["Metadata/Monsters/A", "Metadata/Monsters/B"]
        );
    }

    /// RV-12: a spectre's life comes from the monsters' table, an `AltLife`
    /// type's from its own as before; its energy shield from the allies'.
    #[test]
    fn a_spectres_life_should_come_from_the_monsters_table() {
        let lives = crate::input::MonsterLife {
            life: 3000,
            ally: 1000,
            alt1: Some(5000),
            alt2: 7000,
        };
        let plain = ty(0, 0, 0, false, false);
        let minion = base_numbers(100, &plain, lives, 1.0, 1, false).unwrap();
        let spectre = base_numbers(100, &plain, lives, 1.0, 1, true).unwrap();
        assert!((minion.life - 1000.0).abs() < f64::EPSILON);
        assert!((spectre.life - 3000.0).abs() < f64::EPSILON);
        let alt = ty(0, 0, 0, true, false);
        let a = base_numbers(100, &alt, lives, 1.0, 1, false).unwrap();
        let b = base_numbers(100, &alt, lives, 1.0, 1, true).unwrap();
        assert!((a.life - b.life).abs() < f64::EPSILON);
    }

    /// RV-12: a spectre's level is the gem's `raised_spectre_level`.
    #[test]
    fn a_spectres_level_should_be_the_gems_stat() {
        let sources = LevelSources {
            character: 92,
            enemy: Some(84),
            requirement: 70,
        };
        let stats = vec![(SPECTRE_LEVEL_STAT.to_owned(), 79)];
        assert_eq!(level(&stats, None, true, sources), Some(79));
        // Not from the skill's own list (a support's minion): not known.
        assert_eq!(level(&stats, None, false, sources), None);
    }

    #[test]
    fn weapon_class_should_follow_pobs_class_map() {
        assert_eq!(
            weapon_class("Thrusting One Hand Sword"),
            Some("One Hand Sword")
        );
        assert_eq!(weapon_class("Sceptre"), Some("One Hand Mace"));
        assert_eq!(weapon_class("Staff"), Some("Staff"));
        assert_eq!(weapon_class("Unarmed"), None);
        assert_eq!(weapon_class("Warstaff"), None);
        for class in [
            "Claw",
            "Dagger",
            "Wand",
            "One Hand Sword",
            "Bow",
            "Two Hand Axe",
        ] {
            let ours = weapon_class(class).expect(class);
            assert!(
                super::super::hit::weapon_type_flags(ours).is_some(),
                "{class}"
            );
        }
    }

    /// P6-7c2: the support tables are sorted for their searches.
    #[test]
    fn support_tables_should_be_sorted() {
        assert!(NO_MINION_SUPPORTS.windows(2).all(|w| w[0] < w[1]));
        assert!(NOT_MINION_STATS.windows(2).all(|w| w[0].0 < w[1].0));
    }

    /// P6-7c2: a support's stats on the minion's skill keep its hit rows,
    /// leave its minion modifiers to the minion step, and skip its
    /// `notMinionStat` stats.
    #[test]
    fn a_support_on_the_minion_should_keep_its_hit_rows_only() {
        let melee = SupportOnMinion {
            support: "SupportMeleePhysicalDamage",
        };
        assert!(matches!(
            melee.map(&StatId::new("support_melee_physical_damage_+%_final")),
            StatMap::Effects(e) if e.iter().all(|t| t.quantity == Quantity::Damage)
        ));
        let minion_damage = SupportOnMinion {
            support: "SupportMinionDamage",
        };
        assert_eq!(
            minion_damage.map(&StatId::new("support_minion_damage_+%_final")),
            StatMap::NoEffect
        );
        let second_wind = SupportOnMinion {
            support: "SupportSecondWind",
        };
        assert_eq!(
            second_wind.map(&StatId::new("base_cooldown_speed_+%")),
            StatMap::NoEffect
        );
    }
}
