//! Named conditions the config settles (Phase 3 plan, step 8; P3-8n):
//! PoB's condition checks that only modifiers read, settled from the
//! active config set as [`super::states`] settles the weapon conditions.
//!
//! **PoB v2.67.2** (`Modules/ConfigOptions.lua`): each check option's
//! `apply` adds a `Condition:<name>` flag (the enemy's for an enemy
//! condition), conditioned on `Condition:Combat` or `Condition:Effective`,
//! both on in PoB's main view, and `BuildModList` applies a check only
//! when it is on (`Classes/ConfigTab.lua`). So an unset or `false` check
//! is a settled `false` (none of these is a `defaultState = true` option,
//! which PoB saves only when off: those are `config::DEFAULT_ON`):
//! - `conditionShockedEnemyRecently`: `ShockedEnemyRecently` (1340);
//! - `conditionShatteredEnemyRecently`: `ShatteredEnemyRecently` (1331);
//! - `conditionAtCloseRange`: `AtCloseRange` (1765);
//! - `conditionBlockedRecently`, `conditionBlockedAttackRecently`,
//!   `conditionBlockedSpellRecently`: `BlockedRecently`, set by any of the
//!   three (1410 to 1420; P6-11g), and the last two's own
//!   `BlockedAttackRecently` and `BlockedSpellRecently`;
//! - `conditionOnBurningGround`: `OnBurningGround` (1163; P6-11w; its
//!   implied `Burning` is the UI's alone);
//! - `conditionFullLife`: `FullLife` (174; P6-11ai; Chaos Inoculation's
//!   too, `states`' since P6-11ej);
//! - `conditionEnemyFullLife`: the enemy's `FullLife` (1774);
//! - `conditionEnemyBurning`, `conditionEnemyIgnited`: the enemy's `Burning`
//!   (1849, 1852; P6-11dk): an ignited enemy is burning (`implyCond`, and
//!   `CalcSetup.lua` 72's `Burning` flag from `Ignited`);
//! - `conditionEnemyLowLife`: the enemy's `LowLife` (1777; P6-9e, read by
//!   Punishment's hex, `gems::SKILLS`);
//! - `conditionEnemyRareOrUnique`: the enemy's `RareOrUnique` (2057), which
//!   the `Boss`, `Pinnacle`, and `Uber` boss kinds also set (2095, 2129,
//!   2162; the kind as [`super::enemy`] reads it, PoB's default
//!   `Pinnacle` too);
//! - `buffArcaneSurge`: `Condition:ArcaneSurge` (1002), which
//!   `CalcPerform.lua` (686) turns into `AffectedByArcaneSurge` (the buff's
//!   own effect is [`super::buffs`]');
//! - `conditionFocused`: `Focused` (992; P6-11gk), which no skill or
//!   modifier PoB reads sets;
//! - `conditionUsedMinionSkillRecently`: `UsedMinionSkillRecently` (1526;
//!   P6-11cm), which `CalcPerform.lua` (249) also sets from the main skill
//!   ([`used_minion_skill`]). Its implied `UsedSkillRecently` is the UI's.
//!
//! A value of another type (PoB would read it otherwise) leaves the
//! condition unsettled, so an effect that needs it is a reason, and
//! [`handles`] leaves the input to `collect`'s `unmodelled-config`.
//!
//! The modifiers that read these conditions are mapping rows with a state
//! (`mapping::STATE_ROWS`). No modifier the engine maps sets one of them
//! (PoB's other setters are unmapped stats). A condition a mapped
//! modifier can also set (being on consecrated ground while stationary)
//! is not settled here: its input stays in `config::NAMED_ONLY`.

use std::collections::BTreeMap;

use loom_domain::{ConfigInputKind, ConfigSet, ConfigValue};

use super::config::last_input as last;
use super::enemy::{self, BossKind};
use crate::condition::ConditionKey;
use crate::input::CalcInput;

/// "if you've used a Minion Skill Recently" (PoB's `UsedMinionSkillRecently`;
/// P6-11cm): its check, or the main skill (module docs).
pub const USED_MINION_SKILL_RECENTLY: &str = "UsedMinionSkillRecently";
/// "if you've Attacked Recently" (PoB's `AttackedRecently`; P6-11em): its
/// check (`ConfigOptions.lua` 1485), or a main skill that attacks
/// ([`main_skill_recently`]).
pub const ATTACKED_RECENTLY: &str = "AttackedRecently";
/// "if you've Cast a Spell Recently" (PoB's `CastSpellRecently`; P6-11em):
/// its check (1489), or a main skill that is a spell and no attack.
pub const CAST_SPELL_RECENTLY: &str = "CastSpellRecently";

/// "if you've Consumed a corpse Recently" (PoB's `ConsumedCorpseRecently`;
/// P6-11gp): its check, or the check of a corpse consumed in the past 2
/// seconds (`implyCond`, `ConfigOptions.lua` 1599 to 1603).
pub const CONSUMED_CORPSE_RECENTLY: &str = "ConsumedCorpseRecently";

/// "#% chance to gain Arcane Surge when you Kill an Enemy" (P6-11gs).
pub const ARCANE_SURGE_ON_KILL_STAT: &str = "gain_arcane_surge_on_kill_chance_%";

/// The count of corpses consumed recently (P6-11gp), which sets
/// [`CONSUMED_CORPSE_RECENTLY`] too.
const CORPSES_CONSUMED_INPUT: &str = "multiplierCorpseConsumedRecently";

/// "while on Caustic Ground" (PoB's `OnCausticGround`; P6-11gp): its check
/// (`ConfigOptions.lua` 1157).
pub const ON_CAUSTIC_GROUND: &str = "OnCausticGround";

/// Ngamahu's Flames' Magmatic Strikes (PoB's `NgamahuFlamesAdvance`;
/// P6-11gp): its check (`ConfigOptions.lua` 1469).
pub const NGAMAHU_FLAMES_ADVANCE: &str = "NgamahuFlamesAdvance";

/// "while not Ignited, Frozen or Shocked" (P6-11gp): the engine's key for
/// PoB's condition that none of `Ignited`, `Frozen`, `Shocked` holds
/// (`ModParser.lua`'s `neg` `varList`), settled from their checks
/// ([`settle`]).
pub const NOT_IGNITED_FROZEN_SHOCKED: &str = "NotIgnitedFrozenShocked";

/// The checks that set `Ignited`, `Frozen`, or `Shocked` on you
/// (`ConfigOptions.lua` 1171, 1181, 1193, 1199).
const IGNITED_FROZEN_SHOCKED_INPUTS: [&str; 4] = [
    "conditionFrozen",
    "conditionIgnited",
    "conditionOnShockedGround",
    "conditionShocked",
];

/// "if you've cast Elemental Weakness in the past 10 seconds" (PoB's
/// `SelfCastElementalWeakness`; P6-11gz): [`self_cast`]'s.
pub const SELF_CAST_ELEMENTAL_WEAKNESS: &str = "SelfCastElementalWeakness";
/// "if you've cast Flammability in the past 10 seconds" (P6-11gz).
pub const SELF_CAST_FLAMMABILITY: &str = "SelfCastFlammability";
/// "if you've cast Frostbite in the past 10 seconds" (P6-11gz).
pub const SELF_CAST_FROSTBITE: &str = "SelfCastFrostbite";
/// "if you've cast Conductivity in the past 10 seconds" (P6-11gz).
pub const SELF_CAST_CONDUCTIVITY: &str = "SelfCastConductivity";

/// The state key of a curse cast by you (PoB's `SelfCast<Curse>`, the name
/// without spaces).
#[must_use]
pub fn self_cast(curse: &str) -> String {
    format!("SelfCast{}", curse.replace(' ', ""))
}

/// "while Focused" (PoB's `Focused`; P6-11gk): its check alone
/// (`ConfigOptions.lua` 992).
pub const FOCUSED: &str = "Focused";

/// "if you've used a Life Flask in the past 10 seconds", "during effect of
/// any Life Flask" (PoB's `UsingLifeFlask`; P6-11cq): from the active
/// flasks ([`super::flasks::using_life_flask`]).
pub const USING_LIFE_FLASK: &str = "UsingLifeFlask";

/// "during any Flask Effect" (PoB's `UsingFlask`; P6-11dz): its check
/// `conditionUsingFlask`, or an active flask
/// ([`super::flasks::using_flask`]).
pub const USING_FLASK: &str = "UsingFlask";

/// "while you are affected by a Herald" (PoB's `AffectedByHerald`;
/// P6-11ct): a herald among the skills (`CalcPerform.lua` 2117 to 2120,
/// [`super::gems::herald_and_aura_counts`]).
pub const AFFECTED_BY_HERALD: &str = "AffectedByHerald";

/// The enemy is cursed (PoB's enemy `Cursed`; P6-11cu): its check, or a
/// curse the gem table applies ([`super::gems::curses_enemy`]).
pub const ENEMY_CURSED: &str = "Enemy:Cursed";

/// The enemy is marked (PoB's enemy `Marked`, set only by a mark in a
/// curse slot; P6-11de): [`super::gems::marks_enemy`], unsettled when the
/// slots' priorities decide. No config check sets it.
pub const ENEMY_MARKED: &str = "Enemy:Marked";

/// The enemy is burning (PoB's enemy `Burning`; P6-11dk): its check, or
/// the ignited check (an ignited enemy burns).
pub const ENEMY_BURNING: &str = "Enemy:Burning";

/// The enemy is bleeding (PoB's enemy `Bleeding`, set only by its config
/// check `conditionEnemyBleeding`, `ConfigOptions.lua`; P6-11dp).
pub const ENEMY_BLEEDING: &str = "Enemy:Bleeding";

/// The enemy is blinded (PoB's enemy `Blinded`, from its config check
/// `conditionEnemyBlinded`, `ConfigOptions.lua` 1835; P6-11hr). PoB's
/// other sources are modifiers giving the enemy the condition, unmapped
/// stats, so reasons.
pub const ENEMY_BLINDED: &str = "Enemy:Blinded";

/// The enemy is maimed (PoB's enemy `Maimed`, set only by its config check
/// `conditionEnemyMaimed`, `ConfigOptions.lua` 1825; P6-11ec).
pub const ENEMY_MAIMED: &str = "Enemy:Maimed";

/// The check of [`USED_MINION_SKILL_RECENTLY`].
pub const USED_MINION_SKILL_INPUT: &str = "conditionUsedMinionSkillRecently";

/// "if you have Shocked an Enemy Recently".
pub const SHOCKED_ENEMY_RECENTLY: &str = "ShockedEnemyRecently";
/// "if you've Shattered an Enemy Recently".
pub const SHATTERED_ENEMY_RECENTLY: &str = "ShatteredEnemyRecently";
/// "at Close Range".
pub const AT_CLOSE_RANGE: &str = "AtCloseRange";
/// "while stationary" (PoB's `Condition:Stationary`, set when the config's
/// `conditionStationary` count is above 0; P6-10b).
pub const STATIONARY: &str = "Stationary";
/// "if you've stopped taking Damage Over Time Recently" (P6-10b).
pub const STOPPED_TAKING_DOT_RECENTLY: &str = "StoppedTakingDamageOverTimeRecently";
/// "if you've Blocked Recently" (P6-11g).
pub const BLOCKED_RECENTLY: &str = "BlockedRecently";
/// "if you haven't Blocked Recently": [`BLOCKED_RECENTLY`] negated,
/// unsettled when it is (P6-11eg).
pub const NOT_BLOCKED_RECENTLY: &str = "NotBlockedRecently";
/// "if you haven't dealt a Critical Strike Recently": [`CRIT_RECENTLY`]
/// negated, unsettled when it is (P6-11fb).
pub const NOT_CRIT_RECENTLY: &str = "NotCritRecently";
/// "if you haven't Killed Recently": [`KILLED_RECENTLY`] negated,
/// unsettled when it is (P6-11fb).
pub const NOT_KILLED_RECENTLY: &str = "NotKilledRecently";
/// "if you've Summoned a Totem Recently" (PoB's `SummonedTotemRecently`;
/// P6-11hu): its check (`ConfigOptions.lua` 1109), or a main skill that
/// is a totem (`CalcPerform.lua` 265 to 267, [`summoned_totem`]).
pub const SUMMONED_TOTEM_RECENTLY: &str = "SummonedTotemRecently";
/// "Disable Emperor's Vigilance Bypass" (PoB's `Condition:EVBypass`,
/// `ConfigOptions.lua` 245; P6-11hy).
pub const EV_BYPASS: &str = "EVBypass";
/// [`EV_BYPASS`] negated (P6-11hy): Emperor's Vigilance's lines need it.
pub const NOT_EV_BYPASS: &str = "NotEVBypass";
/// The main skill is used by you: not of the `SummonsTotem`,
/// `RemoteMined`, or `Trapped` type (with its supports' added types),
/// and not the mirage's (P6-11hv; PoB's negated `SkillType` list and
/// negated `usedByMirage` of Ancestral Bond's "You can't deal Damage with
/// Skills yourself", `ModParser.lua` 5252). Settled from the main skill
/// ([`skill_used_by_you`]).
pub const SKILL_USED_BY_YOU: &str = "SkillUsedByYou";
/// "against Enemies that are on Full Life": the enemy's `FullLife`.
pub const ENEMY_FULL_LIFE: &str = "Enemy:FullLife";
/// The enemy is on low life: the enemy's `LowLife` (P6-9e).
pub const ENEMY_LOW_LIFE: &str = "Enemy:LowLife";
/// "against Unique Enemies": the enemy's `RareOrUnique`.
pub const ENEMY_RARE_OR_UNIQUE: &str = "Enemy:RareOrUnique";
/// "while a Rare or Unique Enemy is Nearby" (P6-10b; PoB's enemy
/// `ActorCondition` on either `NearbyRareOrUniqueEnemy` or `RareOrUnique`,
/// `ModParser.lua` 1776): the enemy's [`ENEMY_RARE_OR_UNIQUE`], or the
/// config's count of nearby rare or unique enemies at least 1
/// (`ConfigOptions.lua` 1231).
pub const ENEMY_RARE_OR_UNIQUE_NEARBY: &str = "Enemy:RareOrUniqueNearby";

/// The config's count of nearby rare or unique enemies.
pub const NEARBY_RARE_OR_UNIQUE_INPUT: &str = "multiplierNearbyRareOrUniqueEnemies";
/// "if you've dealt a Critical Strike Recently" (PoB's `CritRecently`,
/// from `conditionCritRecently`, `ConfigOptions.lua` 1240; close-out).
pub const CRIT_RECENTLY: &str = "CritRecently";
/// "while Leeching" (P6-11bv; PoB's `Leeching`: the config's four
/// leeching checks, `ConfigOptions.lua` 1068 to 1081; PoB also sets it
/// for "Life Leech effects are not removed at Full Life", stats that stay
/// reasons, `config::DEPENDS_ON`).
pub const LEECHING: &str = "Leeching";
/// "while Leeching Energy Shield" (P6-11bv).
pub const LEECHING_ENERGY_SHIELD: &str = "LeechingEnergyShield";
/// "while you have Arcane Surge" (PoB's `AffectedByArcaneSurge`).
pub const ARCANE_SURGE: &str = "AffectedByArcaneSurge";

/// "if you've been Hit Recently" (PoB's `BeenHitRecently`, from
/// `conditionBeenHitRecently` or a `multiplierBeenHitRecently` above 0;
/// P6-10b part 13).
pub const BEEN_HIT_RECENTLY: &str = "BeenHitRecently";
/// "if you haven't been Hit Recently": [`BEEN_HIT_RECENTLY`] negated.
pub const NOT_HIT_RECENTLY: &str = "NotBeenHitRecently";
/// "if you've Killed Recently" (PoB's `KilledRecently`, from
/// `conditionKilledRecently`; P6-11p).
pub const KILLED_RECENTLY: &str = "KilledRecently";
/// "while moving" (PoB's `Moving`, from `conditionMoving`; P6-10b part 13).
pub const MOVING: &str = "Moving";
/// "while on Burning Ground" (PoB's `OnBurningGround`, from
/// `conditionOnBurningGround`, `ConfigOptions.lua` 1163; P6-11w).
pub const ON_BURNING_GROUND: &str = "OnBurningGround";
/// "while on Full Life" (PoB's `FullLife`, from `conditionFullLife`,
/// `ConfigOptions.lua` 174; P6-11ai). PoB sets it otherwise only for Chaos
/// Inoculation (`CalcPerform.lua` 79 to 83), which `states` reads from the
/// allocated keystone (P6-11ej).
pub const FULL_LIFE: &str = "FullLife";
/// "when on Low Life" (PoB's `LowLife`, from `conditionLowLife`,
/// `ConfigOptions.lua` 177; P6-11ei). PoB sets it from no other source in
/// its calc modules: life reserved past the threshold (`CalcPerform.lua`
/// 546) reads `LowLifePercentage` for display only.
pub const LOW_LIFE: &str = "LowLife";
/// "while Ignited" (PoB's `Ignited`, from `conditionIgnited`,
/// `ConfigOptions.lua` 1181; P6-11ei). Its implied `Burning` is the UI's.
pub const IGNITED: &str = "Ignited";
/// The count of hits taken recently, which sets [`BEEN_HIT_RECENTLY`]
/// above 0 (`ConfigOptions.lua`).
pub const HIT_RECENTLY_COUNT_INPUT: &str = "multiplierBeenHitRecently";
/// The count of enemies killed recently, which sets [`KILLED_RECENTLY`]
/// above 0 (`ConfigOptions.lua`).
pub const KILLED_RECENTLY_COUNT_INPUT: &str = "multiplierKilledRecently";
/// "if you've used a Warcry Recently" (PoB's `UsedWarcryRecently`, from
/// `conditionUsedWarcryRecently` or a `multiplierWarcryUsedRecently` above
/// 0, `ConfigOptions.lua` 1558, 1612; P6-11ev).
pub const USED_WARCRY_RECENTLY: &str = "UsedWarcryRecently";
/// "if you've Warcried in the past 8 seconds" (PoB's
/// `UsedWarcryInPast8Seconds`; P6-11hz): its check
/// (`ConfigOptions.lua` 1563), or [`USED_WARCRY_RECENTLY`] and its count,
/// which imply it (1558, 1612).
pub const USED_WARCRY_IN_PAST_8_SECONDS: &str = "UsedWarcryInPast8Seconds";
/// The count of warcries used recently, which sets
/// [`USED_WARCRY_RECENTLY`] above 0.
pub const WARCRY_RECENTLY_COUNT_INPUT: &str = "multiplierWarcryUsedRecently";

/// The auras whose `AffectedBy<Name>` condition a row reads (P6-11o),
/// each with its condition: PoB sets it while the aura affects the
/// character (`CalcPerform.lua` 2417 to 2419; a Vaal aura's too, its name
/// without "Vaal "). Settled from the gem table's auras
/// ([`super::gems::affected_by`]).
pub const AFFECTED_BY: [(&str, &str); 7] = [
    ("Anger", "AffectedByAnger"),
    ("Determination", "AffectedByDetermination"),
    ("Grace", "AffectedByGrace"),
    ("Hatred", "AffectedByHatred"),
    ("Precision", "AffectedByPrecision"),
    ("Pride", "AffectedByPride"),
    ("Wrath", "AffectedByWrath"),
];

/// PoB's `Condition:Lifetap`, from its config check `buffLifetap`
/// (`ConfigOptions.lua`; P6-10b part 8: Lifetap's more damage while
/// buffed).
pub const LIFETAP: &str = "Lifetap";

/// PoB's `Condition:ChannellingCyclone`, from its config check
/// `channellingCycloneCheck` (`ConfigOptions.lua` 375; P6-8a: Cyclone's
/// movement speed buff, `poe1::gems`).
pub const CHANNELLING_CYCLONE: &str = "ChannellingCyclone";

/// The check inputs and the condition each sets (module docs), sorted by
/// input (a test checks); an input may set several, and several inputs
/// one: the condition holds when any of its inputs is on (P6-11g).
pub const CHECKS: &[(&str, &str)] = &[
    // P6-11hy: Emperor's Vigilance's bypass disabled.
    ("EVBypass", EV_BYPASS),
    ("buffArcaneSurge", ARCANE_SURGE),
    ("buffLifetap", LIFETAP),
    // P6-11gp: Magmatic Strikes.
    ("buffNgamahuFlamesAdvance", NGAMAHU_FLAMES_ADVANCE),
    ("channellingCycloneCheck", CHANNELLING_CYCLONE),
    ("conditionAtCloseRange", AT_CLOSE_RANGE),
    // P6-11em: attacked recently; the main skill too (`settle`).
    ("conditionAttackedRecently", ATTACKED_RECENTLY),
    ("conditionBeenHitRecently", BEEN_HIT_RECENTLY),
    ("conditionBlockedAttackRecently", "BlockedAttackRecently"),
    ("conditionBlockedAttackRecently", BLOCKED_RECENTLY),
    ("conditionBlockedRecently", BLOCKED_RECENTLY),
    ("conditionBlockedSpellRecently", BLOCKED_RECENTLY),
    ("conditionBlockedSpellRecently", "BlockedSpellRecently"),
    // P6-11em: cast a spell recently; the main skill too (`settle`).
    ("conditionCastSpellRecently", CAST_SPELL_RECENTLY),
    // P6-11gp: a corpse consumed recently (the past 2 seconds implies it).
    (
        "conditionConsumedCorpseInPast2Sec",
        "ConsumedCorpseInPast2Sec",
    ),
    (
        "conditionConsumedCorpseInPast2Sec",
        CONSUMED_CORPSE_RECENTLY,
    ),
    ("conditionConsumedCorpseRecently", CONSUMED_CORPSE_RECENTLY),
    ("conditionCritRecently", CRIT_RECENTLY),
    ("conditionEnemyBleeding", ENEMY_BLEEDING),
    // P6-11hr: blinded, for the rows against a blinded enemy (the blind's
    // less evasion is `damage_taken`'s).
    ("conditionEnemyBlinded", ENEMY_BLINDED),
    ("conditionEnemyBurning", ENEMY_BURNING),
    ("conditionEnemyCursed", ENEMY_CURSED),
    ("conditionEnemyFullLife", ENEMY_FULL_LIFE),
    // P6-11dk: an ignited enemy is burning (`implyCond`).
    ("conditionEnemyIgnited", ENEMY_BURNING),
    ("conditionEnemyLowLife", ENEMY_LOW_LIFE),
    ("conditionEnemyMaimed", ENEMY_MAIMED),
    ("conditionEnemyRareOrUnique", ENEMY_RARE_OR_UNIQUE),
    // P6-11gk: focused.
    ("conditionFocused", FOCUSED),
    ("conditionFullLife", FULL_LIFE),
    // P6-11ei: being ignited.
    ("conditionIgnited", IGNITED),
    ("conditionKilledRecently", KILLED_RECENTLY),
    // P6-11bv: each leeching check sets `Leeching` (PoB's `implyCond`).
    ("conditionLeeching", LEECHING),
    ("conditionLeechingEnergyShield", LEECHING),
    ("conditionLeechingEnergyShield", LEECHING_ENERGY_SHIELD),
    ("conditionLeechingLife", LEECHING),
    ("conditionLeechingLife", "LeechingLife"),
    ("conditionLeechingMana", LEECHING),
    ("conditionLeechingMana", "LeechingMana"),
    // P6-11ei: low life (Pain Attunement, movement speed).
    ("conditionLowLife", LOW_LIFE),
    ("conditionMoving", MOVING),
    ("conditionOnBurningGround", ON_BURNING_GROUND),
    // P6-11gp: on caustic ground.
    ("conditionOnCausticGround", ON_CAUSTIC_GROUND),
    ("conditionShatteredEnemyRecently", SHATTERED_ENEMY_RECENTLY),
    ("conditionShockedEnemyRecently", SHOCKED_ENEMY_RECENTLY),
    (
        "conditionStoppedTakingDamageOverTimeRecently",
        STOPPED_TAKING_DOT_RECENTLY,
    ),
    // P6-11hu: summoned a totem recently; a totem main skill too (`settle`).
    ("conditionSummonedTotemRecently", SUMMONED_TOTEM_RECENTLY),
    (USED_MINION_SKILL_INPUT, USED_MINION_SKILL_RECENTLY),
    // P6-11hz: warcried in the past 8 seconds; implied too (`settle`).
    (
        "conditionUsedWarcryInPast8Seconds",
        USED_WARCRY_IN_PAST_8_SECONDS,
    ),
    // P6-11ev: used a warcry recently; a warcry count too (`settle`).
    ("conditionUsedWarcryRecently", USED_WARCRY_RECENTLY),
    ("conditionUsingFlask", USING_FLASK),
];

/// Whether the main skill makes the character have used a minion skill
/// recently (P6-11cm; PoB's `doActorMisc`, `CalcPerform.lua` 240 to 251):
/// a skill with PoB's `minion` flag and not `permanentMinion`, not
/// triggered, not a trap, mine, or totem. `None` when the main skill is not
/// certain; `Some(false)` without one (PoB's empty main skill).
#[must_use]
pub fn used_minion_skill(input: &CalcInput<'_>) -> Option<bool> {
    main_skill_recently(input).map(|r| r.minion)
}

/// What the main skill makes the character have done recently (PoB's
/// `doActorMisc`, `CalcPerform.lua` 240 to 251), for a skill not
/// triggered and not a trap, mine, or totem: attacked (PoB's `attack`
/// flag), else cast a spell (`spell`), and used a minion skill (`minion`,
/// not `permanentMinion`). `None` when the main skill is not certain; all
/// `false` without one (PoB's empty main skill).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MainSkillRecently {
    /// `AttackedRecently`.
    pub attacked: bool,
    /// `CastSpellRecently`.
    pub cast_spell: bool,
    /// `UsedMinionSkillRecently`.
    pub minion: bool,
}

/// Whether the main skill is a totem (P6-11hu; PoB's `skillFlags.totem`,
/// the skill's types with its supports' added types holding
/// `SummonsTotem`), which sets `SummonedTotemRecently`
/// (`CalcPerform.lua` 265 to 267). `None` when the main skill is not
/// certain; `Some(false)` without one.
#[must_use]
pub fn summoned_totem(input: &CalcInput<'_>) -> Option<bool> {
    use super::skill::{self, SkillProblem};
    let levels = super::gem_levels::GemLevels::none();
    match skill::main_skill(input.build, input.data, &levels) {
        Ok(s) => Some(s.types.contains(skill::TOTEM_TYPE)),
        Err(SkillProblem::NoActiveSkill) => Some(false),
        Err(_) => None,
    }
}

/// [`SKILL_USED_BY_YOU`] for the build's main skill (P6-11hv): `false`
/// for the mirage's calculation, a totem, a mine, or a trap; `None` when
/// the main skill is not certain; `true` without one (PoB's tag passes
/// an empty skill type list).
#[must_use]
pub fn skill_used_by_you(input: &CalcInput<'_>) -> Option<bool> {
    use super::skill::{self, SkillProblem};
    if input.options.mirage.is_some() {
        return Some(false);
    }
    let levels = super::gem_levels::GemLevels::none();
    match skill::main_skill(input.build, input.data, &levels) {
        Ok(s) => Some(
            ![skill::TOTEM_TYPE, skill::MINE_TYPE, skill::TRAP_TYPE]
                .iter()
                .any(|t| s.types.contains(*t)),
        ),
        Err(SkillProblem::NoActiveSkill) => Some(true),
        Err(_) => None,
    }
}

/// [`MainSkillRecently`] for the build's main skill.
#[must_use]
pub fn main_skill_recently(input: &CalcInput<'_>) -> Option<MainSkillRecently> {
    use super::skill::{self, SkillProblem};
    // Gem levels do not decide the skill's types or supports.
    let levels = super::gem_levels::GemLevels::none();
    let skill = match skill::main_skill(input.build, input.data, &levels) {
        Ok(s) => s,
        Err(SkillProblem::NoActiveSkill) => return Some(MainSkillRecently::default()),
        Err(_) => return None,
    };
    let Some(pob) = super::pob_skill::skill(&skill.active_gem.effect.id) else {
        return Some(MainSkillRecently::default());
    };
    let placed = ["Trapped", "RemoteMined", "SummonsTotem"]
        .iter()
        .any(|t| skill.types.contains(*t));
    let triggered = !matches!(
        super::triggers::kind(&skill),
        super::triggers::Kind::NotTriggered
    );
    if placed || triggered {
        return Some(MainSkillRecently::default());
    }
    let attacked = pob.flag("attack");
    Some(MainSkillRecently {
        attacked,
        cast_spell: !attacked && pob.flag("spell"),
        minion: pob.flag("minion") && !pob.flag("permanentMinion"),
    })
}

/// A check's value: `Some(false)` when unset or off, `None` for a value
/// that is not a boolean.
#[must_use]
pub fn check(set: Option<&ConfigSet>, name: &str) -> Option<bool> {
    match set.and_then(|s| last(s, name, ConfigInputKind::Input)) {
        None | Some(ConfigValue::Boolean(false)) => Some(false),
        Some(ConfigValue::Boolean(true)) => Some(true),
        Some(ConfigValue::Number(_) | ConfigValue::String(_)) => None,
    }
}

/// Whether this module settles `input` from `value` (a check as a
/// boolean).
#[must_use]
pub fn handles(input: &str, value: &ConfigValue) -> bool {
    CHECKS.iter().any(|(name, _)| *name == input) && matches!(value, ConfigValue::Boolean(_))
}

/// The conditions the config settles (module docs), into `out`.
pub fn settle(input: &CalcInput<'_>, out: &mut BTreeMap<ConditionKey, bool>) {
    let set = super::collect::active_config(input.build);
    // The stationary count (a number, [`super::buffs::STATIONARY_INPUT`]):
    // above 0 is `Stationary`; a value of another type leaves it unsettled.
    match set.and_then(|s| last(s, super::buffs::STATIONARY_INPUT, ConfigInputKind::Input)) {
        None => {
            out.insert(ConditionKey::new(STATIONARY), false);
        }
        Some(ConfigValue::Number(n)) => {
            out.insert(ConditionKey::new(STATIONARY), n.value().0 > 0.0);
        }
        Some(_) => {}
    }
    let mut seen: BTreeMap<&str, Option<bool>> = BTreeMap::new();
    for &(name, condition) in CHECKS {
        let mut value = check(set, name);
        if condition == ENEMY_RARE_OR_UNIQUE
            && enemy::enemy(input.build, input.data).kind != BossKind::None
        {
            value = value.map(|_| true);
        }
        // An Arcane Surge Support's buff gives Arcane Surge too
        // (`poe1::gems::arcane_surge_support`; close-out).
        if condition == ARCANE_SURGE {
            // Gem levels do not decide which supports apply.
            let levels = super::gem_levels::GemLevels::none();
            if super::gems::arcane_surge_support(input, &levels) {
                value = Some(true);
            }
        }
        // Any of a condition's inputs on sets it; an unreadable input leaves
        // it unsettled unless another sets it (P6-11g).
        let key = ConditionKey::new(condition);
        let seen_before = seen.get(condition).copied().flatten().is_some();
        let merged = match (seen.get(condition).copied(), value) {
            (None, v) => v,
            (Some(Some(true)), _) | (_, Some(true)) => Some(true),
            (Some(None), _) | (_, None) => None,
            (Some(Some(false)), Some(false)) => Some(false),
        };
        seen.insert(condition, merged);
        match merged {
            Some(v) => {
                out.insert(key, v);
            }
            // Only an entry this loop set earlier (P6-7c review).
            None if seen_before => {
                out.remove(&key);
            }
            None => {}
        }
    }
    // Arcane Surge on kill (P6-11gs; `Condition:ArcaneSurge` with
    // `KilledRecently`): with the stat, killed recently sets Arcane Surge.
    if out.get(&ConditionKey::new(KILLED_RECENTLY)) == Some(&true)
        && super::states::build_has_stat(input, ARCANE_SURGE_ON_KILL_STAT)
    {
        out.insert(ConditionKey::new(ARCANE_SURGE), true);
    }
    // The count of corpses consumed recently also sets the condition
    // (P6-11gp; its `implyCond` and `Condition:ConsumedCorpseRecently` mod,
    // `ConfigOptions.lua` 1605 to 1608): a count above 0 settles it true,
    // a value of another type leaves it unsettled.
    match set.and_then(|s| last(s, CORPSES_CONSUMED_INPUT, ConfigInputKind::Input)) {
        Some(ConfigValue::Number(n)) if n.value().0 > 0.0 => {
            out.insert(ConditionKey::new(CONSUMED_CORPSE_RECENTLY), true);
        }
        None | Some(ConfigValue::Number(_)) => {}
        Some(_) => {
            out.remove(&ConditionKey::new(CONSUMED_CORPSE_RECENTLY));
        }
    }
    // The curses cast by you (P6-11gz): each of the gem table's curses,
    // true when cast by you, false otherwise; unsettled when the skills are
    // not certain.
    if let Some(cast) = super::gems::self_cast_curses(input) {
        for name in super::gems::curse_names() {
            out.insert(ConditionKey::new(self_cast(name)), cast.contains(name));
        }
    }
    // Not ignited, frozen, or shocked (P6-11gp): every check off settles it
    // true, any on false; a value of another type leaves it unsettled.
    let checks: Vec<Option<bool>> = IGNITED_FROZEN_SHOCKED_INPUTS
        .iter()
        .map(|name| check(set, name))
        .collect();
    if checks.contains(&Some(true)) {
        out.insert(ConditionKey::new(NOT_IGNITED_FROZEN_SHOCKED), false);
    } else if checks.iter().all(Option::is_some) {
        out.insert(ConditionKey::new(NOT_IGNITED_FROZEN_SHOCKED), true);
    }
    // A minion skill used recently (P6-11cm): the check or the main skill.
    // Attacked and cast a spell recently (P6-11em): the same.
    let recently = main_skill_recently(input);
    for (name, from_skill) in [
        (USED_MINION_SKILL_RECENTLY, recently.map(|r| r.minion)),
        (ATTACKED_RECENTLY, recently.map(|r| r.attacked)),
        (CAST_SPELL_RECENTLY, recently.map(|r| r.cast_spell)),
    ] {
        let key = ConditionKey::new(name);
        if out.get(&key).copied() == Some(false) {
            match from_skill {
                Some(false) => {}
                Some(true) => {
                    out.insert(key, true);
                }
                None => {
                    out.remove(&key);
                }
            }
        }
    }
    // A totem summoned recently (P6-11hu): the check, or a totem main skill.
    let key = ConditionKey::new(SUMMONED_TOTEM_RECENTLY);
    if out.get(&key).copied() == Some(false) {
        match summoned_totem(input) {
            Some(false) => {}
            Some(true) => {
                out.insert(key, true);
            }
            None => {
                out.remove(&key);
            }
        }
    }
    // The main skill used by you (P6-11hv): no config check sets it.
    if let Some(v) = skill_used_by_you(input) {
        out.insert(ConditionKey::new(SKILL_USED_BY_YOU), v);
    }
    // The enemy cursed (P6-11cu): the check, or a curse applied.
    let key = ConditionKey::new(ENEMY_CURSED);
    if out.get(&key).copied() == Some(false)
        && super::gems::curses_enemy(input, &super::gem_levels::GemLevels::none())
    {
        out.insert(key, true);
    }
    // The enemy marked (P6-11de): only where the skill list settles it.
    if let Some(v) = super::gems::marks_enemy(input, &super::gem_levels::GemLevels::none()) {
        out.insert(ConditionKey::new(ENEMY_MARKED), v);
    }
    // Affected by a herald (P6-11ct): PoB's herald count above 0.
    let (heralds, _) = super::gems::herald_and_aura_counts(input);
    out.insert(ConditionKey::new(AFFECTED_BY_HERALD), heralds > 0);
    // Using a life flask (P6-11cq): the active flasks; no config check
    // sets it.
    if let Some(v) = super::flasks::using_life_flask(input) {
        out.insert(ConditionKey::new(USING_LIFE_FLASK), v);
    }
    // Using any flask (P6-11dz): its check, or an active flask.
    let key = ConditionKey::new(USING_FLASK);
    if out.get(&key).copied() == Some(false) && super::flasks::using_flask(input) {
        out.insert(key, true);
    }
    // A rare or unique enemy nearby (P6-10b): either condition; a count of
    // another type leaves it unsettled unless the enemy is rare or unique.
    let rare = seen.get(ENEMY_RARE_OR_UNIQUE).copied().flatten();
    let nearby =
        match set.and_then(|s| last(s, NEARBY_RARE_OR_UNIQUE_INPUT, ConfigInputKind::Input)) {
            None => Some(false),
            Some(ConfigValue::Number(n)) => Some(n.value().0 >= 1.0),
            Some(_) => None,
        };
    let either = match (rare, nearby) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), Some(false)) => Some(false),
        _ => None,
    };
    if let Some(v) = either {
        out.insert(ConditionKey::new(ENEMY_RARE_OR_UNIQUE_NEARBY), v);
    }
    // The auras affecting the character (P6-11o).
    let levels = super::gem_levels::GemLevels::none();
    let auras = super::gems::affected_by(input, &levels);
    for (name, condition) in AFFECTED_BY {
        out.insert(ConditionKey::new(condition), auras.contains(name));
    }
    // Killed recently: a kill count above 0 sets it too (PoB's
    // `multiplierKilledRecently`, `ConfigOptions.lua`; P6-11p).
    let key = ConditionKey::new(KILLED_RECENTLY);
    let count = match set.and_then(|s| last(s, KILLED_RECENTLY_COUNT_INPUT, ConfigInputKind::Input))
    {
        None => Some(false),
        Some(ConfigValue::Number(n)) => Some(n.value().0 > 0.0),
        Some(_) => None,
    };
    match (out.get(&key).copied(), count) {
        (Some(true), _) | (_, Some(true)) => {
            out.insert(key, true);
        }
        (Some(false), Some(false)) => {}
        _ => {
            out.remove(&key);
        }
    }
    // Used a warcry recently (P6-11ev): a warcry count above 0 sets it too.
    let key = ConditionKey::new(USED_WARCRY_RECENTLY);
    let count = match set.and_then(|s| last(s, WARCRY_RECENTLY_COUNT_INPUT, ConfigInputKind::Input))
    {
        None => Some(false),
        Some(ConfigValue::Number(n)) => Some(n.value().0 > 0.0),
        Some(_) => None,
    };
    match (out.get(&key).copied(), count) {
        (Some(true), _) | (_, Some(true)) => {
            out.insert(key, true);
        }
        (Some(false), Some(false)) => {}
        _ => {
            out.remove(&key);
        }
    }
    // Warcried in the past 8 seconds (P6-11hz): its check, or used a warcry
    // recently (with its count), which imply it.
    let recent = out.get(&ConditionKey::new(USED_WARCRY_RECENTLY)).copied();
    let key8 = ConditionKey::new(USED_WARCRY_IN_PAST_8_SECONDS);
    match (out.get(&key8).copied(), recent) {
        (Some(true), _) | (_, Some(true)) => {
            out.insert(key8, true);
        }
        (Some(false), Some(false)) => {}
        _ => {
            out.remove(&key8);
        }
    }
    // Hit recently: a count above 0 sets it too; "not hit recently" is its
    // negation, unsettled when it is (P6-10b part 13).
    let key = ConditionKey::new(BEEN_HIT_RECENTLY);
    let count = match set.and_then(|s| last(s, HIT_RECENTLY_COUNT_INPUT, ConfigInputKind::Input)) {
        None => Some(false),
        Some(ConfigValue::Number(n)) => Some(n.value().0 > 0.0),
        Some(_) => None,
    };
    let hit = match (out.get(&key).copied(), count) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), Some(false)) => Some(false),
        _ => None,
    };
    match hit {
        Some(v) => {
            out.insert(key, v);
            out.insert(ConditionKey::new(NOT_HIT_RECENTLY), !v);
        }
        None => {
            out.remove(&key);
        }
    }
    // Not blocked recently (P6-11eg): the negation of the three blocked
    // checks, settled when they are.
    if let Some(v) = out.get(&ConditionKey::new(BLOCKED_RECENTLY)).copied() {
        out.insert(ConditionKey::new(NOT_BLOCKED_RECENTLY), !v);
    }
    // Not crit and not killed recently (P6-11fb): the same.
    for (key, not) in [
        (CRIT_RECENTLY, NOT_CRIT_RECENTLY),
        (KILLED_RECENTLY, NOT_KILLED_RECENTLY),
        // P6-11hy: Emperor's Vigilance's bypass.
        (EV_BYPASS, NOT_EV_BYPASS),
    ] {
        if let Some(v) = out.get(&ConditionKey::new(key)).copied() {
            out.insert(ConditionKey::new(not), !v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_checks_should_be_sorted_and_apart_from_the_named_only_inputs() {
        assert!(CHECKS.windows(2).all(|w| w[0] < w[1]));
        for (input, _) in CHECKS {
            assert!(!super::super::config::named_only(input), "{input}");
        }
    }

    #[test]
    fn only_a_checks_boolean_should_be_handled() {
        assert!(handles(
            "conditionAtCloseRange",
            &ConfigValue::Boolean(false)
        ));
        assert!(!handles(
            "conditionAtCloseRange",
            &ConfigValue::String("x".into())
        ));
        assert!(handles("conditionFullLife", &ConfigValue::Boolean(true)));
        assert!(!handles("conditionStationary", &ConfigValue::Boolean(true)));
    }
}
