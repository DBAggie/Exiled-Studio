//! Config inputs (Phase 3 plan, step 12; P3-12a, P3-12b): which of PoB's
//! config inputs and placeholders can change a computed field.
//!
//! The engine models two inputs (`resistancePenalty` and the enemy
//! model's numeric inputs, [`super::collect`]); every other input is
//! `unmodelled-config` on every computed field, except the ones this
//! module lists: inputs whose only effect is a named condition,
//! multiplier, or distance that modifiers read ([`NAMED_ONLY`]), and the
//! enemy's damage, which only PoB's damage-taken estimates read
//! ([`ENEMY_DAMAGE`]).
//!
//! **Placeholders** (P3-12b). PoB applies a numeric input's placeholder
//! when the input is unset, or 0 unless the option is `countAllowZero`
//! (`Classes/ConfigTab.lua`,
//! `BuildModList`, line 1147), through the input's own `apply`, and its
//! calc modules read a few placeholders directly (`env.configPlaceholder`:
//! the enemy's damage, the distances). So a placeholder is classified as
//! its input is: [`NAMED_ONLY`] and [`ENEMY_DAMAGE`] hold for both. The
//! enemy's level, resistances, armour, and evasion placeholders are
//! [`super::enemy::RESET_PLACEHOLDERS`]: PoB's `enemyIsBoss` sets them on
//! every run, so a build's saved ones are not read. Any other placeholder
//! is `unmodelled-config` under its input's name.
//!
//! Why such an input changes no computed field: in PoB v2.67.2 the input
//! adds `Condition:<name>` or `Multiplier:<name>` mods
//! (`Modules/ConfigOptions.lua`), and only modifiers read them. Every
//! modifier comes from a stat, and every stat the engine ingests is one
//! of three kinds:
//! - mapped: no mapping reads a named condition (a test in
//!   [`super::mapping`] checks the global and hit rows; local and weapon
//!   rows are built with no condition);
//! - `NoEffect`: the stat changes no slice field whatever the config
//!   (P3-8a's criterion, `docs/phase-3/stat-mapping.md`);
//! - a reason (`unmapped-stat`, `unmodelled-gem`, and the others), which
//!   already makes every computed field `Partial`.
//!
//! And where an effect reads a named condition, the store leaves it out
//! with `unmodelled-condition` ([`crate::condition`]): the engine never
//! sets a named condition from the config, so a future mapping that reads
//! one is a reason on its own, never a silent value.
//!
//! An input stays here only if PoB's calc modules (`CalcSetup.lua`,
//! `CalcPerform.lua`, `CalcOffence.lua`, `CalcDefence.lua`) read its names
//! only through modifiers or flags a modifier sets, or add from them only
//! effects outside the slice; each row's comment gives what was checked.
//! A distance (`meleeDistance`, `projectileDistance`) is the same case:
//! PoB reads it as the skill's `skillDist`, and only the `DistanceRamp`
//! and `MeleeProximity` tags of modifiers read that
//! (`Classes/ModStore.lua` 599, 619).
//! The stats behind those modifiers are [`DEPENDS_ON`]: they must stay
//! reasons (`unmapped-stat`), and mapping one (even as `NoEffect`) must
//! revisit this table (a test checks). Inputs with their
//! own effects (a minion's aura, and every input not listed) stay
//! reasons; charges ([`super::charges`], P3-12e), the enemy's damage
//! taken and evasion (shock, the boss kind, delirium: P3-12f; Intimidate,
//! Unnerve, Blind: P3-12g; [`super::damage_taken`]), the bandit
//! ([`super::collect`], P3-12f), the pantheon ([`super::pantheon`],
//! P3-12g), and the enemy's exposure and Elemental Army's exposure type
//! ([`super::exposure`], P3-8k) have their effects. Custom
//! modifier blocks are not inputs: their lines act through their
//! resolutions (`collect::config`, P3-12c), and so do the lines of PoB's
//! legacy `customMods` input where PoB moves them into a block
//! (`collect::legacy_custom_mods`, P3-12d).

use loom_domain::{ConfigInputKind, ConfigSet, ConfigValue};

/// Config inputs whose only effect is named conditions or multipliers
/// that modifiers read, with those names (PoB v2.67.2,
/// `Modules/ConfigOptions.lua`), sorted by input (a test checks the
/// order). Matched by name whatever the value's type, unlike the enemy
/// inputs: PoB's `apply` keys only on the value (a list input sets a
/// condition per value, a count a multiplier, a check a condition), so
/// every value only sets names. The names are documentation; no code
/// reads them.
///
/// - `OverloadedIntensity`: `Multiplier:OverloadedIntensity` (at most 3,
///   line 514); read by the skill stat map of the gems that grant it.
/// - `bloodSandStance` moved to [`super::gems`] in the Phase 3 close-out
///   (Flesh and Stone's Blood Stance row reads it).
/// - `conditionBeenHitRecently`, `conditionChilledEnemyRecently`,
///   `conditionFullLife`: the condition of that name (lines 1375, 1339,
///   174; the blocked-recently checks moved to [`super::conditions`] in
///   P6-11g, as "+# Armour if you've Blocked Recently" reads them); the
///   calc modules read them
///   only through modifiers
///   or flags a modifier sets (`CalcDefence.lua` 707, 723: maximum block
///   if not blocked recently, from a stat); `FullLife` is set by Chaos
///   Inoculation (`CalcPerform.lua` 86), not read.
/// - `conditionCritRecently` moved to [`super::conditions`] in the Phase 3
///   close-out ("if you've dealt a Critical Strike Recently" reads it; its
///   `SkillCritRecently` stays a name only modifiers read).
/// - `conditionEnemyChilled`, `conditionEnemyChilledEffect`: moved to
///   [`super::damage_taken`] in P3-8m, which turns the chill into damage
///   taken when a modifier's flag asks for it.
/// - `conditionOnConsecratedGround` and `conditionStationary` moved to
///   [`super::buffs`] in P6-10b: consecrated ground's life regeneration
///   is a field now.
/// - Moved in P3-8n: `conditionShockedEnemyRecently` to
///   [`super::conditions`] (its condition settled for the rows that read
///   it); `conditionEnemyOnConsecratedGround` and `conditionEnemyIgnited`
///   to [`super::damage_taken`]; `conditionConvergence` to
///   [`super::buffs`]; `intensifyIntensity`, `multiplierRage`, and
///   `multiplierNonInstantSpellCastRecently` to [`super::stacks`].
/// - `multiplierNearbyEnemies`: `Multiplier:NearbyEnemies` and
///   `Condition:OnlyOneNearbyEnemy` (line 1234); the calc modules never
///   read them.
///
/// Added in P3-12b, the inputs whose placeholders PoB writes into a build
/// (each has a `defaultPlaceholderState`):
///
/// - `ScorchStacks`, `ShockStacks`: the enemy's `Multiplier:ScorchStacks`,
///   `Multiplier:ShockStacks` (lines 1892, 1951). `CalcPerform.lua` (3550,
///   3566) reads them in the shock and scorch effects, only with the
///   `ShockCanStack` or `ScorchCanStack` flag, which only a modifier sets
///   (`ModParser.lua` 3045, 3047).
/// - `aspectOfLunarisLunarProtection`, `aspectOfLunarisLunarResistance`,
///   `aspectOfSolarisSunscaldStacks`, `configSpectralTigerCount`,
///   `multiplierHallowingFlameStacksRemovedByAlly`, `sigilOfPowerStages`:
///   `Multiplier:LunarProtection`, `LunarResistance`, `SunscaldStack`,
///   `SpectralTigerConfig` and `SpectralTigerCount`,
///   `HallowingFlameStacksRemovedByAlly`, `SigilOfPowerStage` (lines 262,
///   265, 269, 644, 1994, 631); the calc modules never read them; the
///   skills' stat maps and modifiers do (`HallowingFlameStacksRemovedByAlly`:
///   extra lightning damage per flame removed, `ModParser.lua` 2898, its
///   stat in [`DEPENDS_ON`]).
/// - `meleeDistance`, `projectileDistance`: no `apply` (lines 1789,
///   1790); `CalcActiveSkill.lua` (457) makes the input or placeholder the
///   skill's `skillDist`, which only modifier tags read (module docs).
///   `CalcOffence.lua` (1083, 1086) adds such modifiers for the
///   `PointBlank` and `FarShot` flags, which only modifiers set.
/// - `multiplierEnemyHallowingFlame`: the enemy's
///   `Multiplier:HallowingFlame` (line 1991); the calc modules never read
///   it (`CalcPerform.lua` 3440 reads the condition, not the multiplier,
///   and only with `CanInflictHallowingFlame`, a modifier's flag).
/// - `multiplierWitheredStackCountSelf`: `Multiplier:WitheredStack` (line
///   1231); `CalcPerform.lua` (852) reads it for chaos damage taken by
///   you (not a slice field), only with `CanBeWithered`, a skill's or
///   map's flag.
///
/// Added in P3-12e:
///
/// - `buffFortification`, `overrideFortification`: `Condition:Fortified`
///   and `FortificationStacks` OVERRIDE in combat (lines 1039, 1042).
///   `CalcSetup.lua` (94) turns the condition into the `Fortified` flag;
///   `CalcPerform.lua` (659 to 680) then sets the stacks (the override or
///   the maximum), `DamageTakenWhenHit` MORE per stack (since P6-10b part
///   12 the enemy's hit, [`super::enemy_hit`], reads the check and the
///   override itself), the `HaveMaximumFortification` flag, and one more
///   `BuffOnSelf`; 3889 exports the stacks to a party (not a slice
///   field). Every other reader is a modifier (per fortification, while
///   fortified, at maximum fortification, per buff on you).
///
/// Added in P3-12f, inputs whose effect needs a condition only modifiers
/// (or another config input, itself a reason) set:
///
/// - `multiplierRage` was here until P3-8n, which gives rage its effect
///   with "Gain # Rage on Melee Hit" ([`super::stacks`]). The other stats
///   that gain or regenerate rage stay in [`DEPENDS_ON`]: with one of
///   them, PoB's rage would differ from the engine's, so they must stay
///   reasons (`ModParser.lua` 2201 to 2585 and 5517 to 5539; the skill
///   data of Chain Hook, Vengeful Cry, a curse's stun rage, and the Spirit
///   of Akoya tattoo).
/// - The three exposure checks were here until P3-8k, which gives them
///   their effect ([`super::exposure`]), with the exposure stats then in
///   [`DEPENDS_ON`].
///
/// Added in P3-12g, the public builds' most frequent remaining inputs
/// (each line of `ConfigOptions.lua`, then every reader of its names in
/// `CalcSetup.lua`, `CalcPerform.lua`, `CalcOffence.lua`, `CalcDefence.lua`,
/// `CalcTriggers.lua`, `CalcActiveSkill.lua`, `CalcTools.lua`, `Calcs.lua`,
/// `CalcMirages.lua`, `CalcBreakdown.lua`):
///
/// - `conditionCastSpellRecently`, `conditionUsedMinionSkillRecently`:
///   `Condition:CastSpellRecently`, `Condition:UsedMinionSkillRecently`,
///   each with `Condition:UsedSkillRecently` (lines 1489, 1526); no calc
///   module reads them.
/// - `conditionKilledRecently`: `Condition:KilledRecently` (line 1270);
///   only `CalcTriggers.lua` (1136, 1156) reads it, for trigger skills
///   (other gems, reasons).
/// - `conditionLeeching`: `Condition:Leeching` (line 1068); no calc module
///   reads it.
/// - `conditionEnemyBleeding`, `conditionEnemyPoisoned`: the enemy's
///   `Condition:Bleeding`, `Condition:Poisoned` (lines 1786, 1798); no
///   calc module reads them on the enemy (`CalcSetup.lua` 73 sets
///   `Poisoned` from poison stacks). `conditionEnemyLowLife` left this
///   list in P6-9e: Punishment's hex reads it (`conditions::CHECKS`).
/// - `conditionEnemyMaimed`: the enemy's `Condition:Maimed` (line 1825);
///   `CalcSetup.lua` (63) gives a maimed actor 30% reduced movement speed
///   (not a slice field).
///
/// Added in P6-4a (59 inputs), every remaining input of the public builds
/// and the corpus whose `apply` (`ConfigOptions.lua`) sets only
/// `Condition:` and `Multiplier:` names, checked as in P3-12g against
/// every calc module (each name searched for in `CalcSetup`, `CalcPerform`,
/// `CalcOffence`, `CalcDefence`, `CalcActiveSkill`, `CalcTriggers`,
/// `CalcMirages`, `Calcs`, `CalcTools`, `ModStore`, `ModDB`, `ModList`;
/// `docs/phase-6/config-inputs.md`). No name is one the engine settles (a
/// test), so a modifier that reads one stays a reason:
/// - no calc module reads the names: the corpse, warcry, totem, mine,
///   channelling, blocking, hit, kill, and impale conditions and counts;
///   the enemy's `Burning`, `Hindered`, `Taunted`, `ChampionIntimidate`,
///   and presence time; Focused, Gale Force, Soul Eater (its speed needs
///   `Condition:CanHaveSoulEater`, set only by modifiers), Resonance,
///   Otherworldly Pressure, Infusion, Magmatic Strikes, Plague Bearer,
///   Pride's maximum effect (Pride is not a modelled gem), Perforate's
///   spikes, Raise Spiders, Meat Shield, Arcane Cloak, Cyclone, and the
///   nearby rare count;
/// - calc modules set them from the build too (`condList` in
///   `CalcPerform.lua` 242 to 276: `AttackedRecently`, `HitRecently`,
///   `HitSpellRecently`, `Channelling`, `SummonedTotemRecently`,
///   `TotemsHitRecently`, `DetonatedMinesRecently`; the enemy's `Cursed`
///   3303; `UsingFlask` from active flasks 1735; the summoned minion
///   count 1459, read at 1469 to set `OnlyMinion`, a condition) and read
///   them only through modifiers;
/// - read only outside the slice: `LowLife` (a flask's instant recovery,
///   1605; the input itself in `CalcDefence.lua` 2216 and 2238, the life
///   pools of the defence estimate), `Lifetap`'s `FlaskLifeRecovery`, `Elusive` (avoidance and
///   movement speed 808 to 810; Nightblade's crit, `CalcOffence.lua` 521,
///   with a support that is not modelled), the enemy's `Debilitated` and
///   `MalignantMadness` (its damage and speed, `CalcSetup.lua` 68 to 71),
///   `KilledRecently` (trigger skills, `CalcTriggers.lua` 1136), the
///   player's `Ignited` (its `Burning`, `CalcSetup.lua` 72), and the
///   enemy's `Withered` stacks (`ChaosDamageTaken` only with
///   `WitherEffectStack`, 812 to 815, which only unmapped stats and gems
///   that are not modelled give).
///
/// `conditionChampionIntimidate` and `conditionSacrificeMinion` are
/// `defaultState = true` options: PoB saves such an input only when it
/// differs from the default (`ConfigTab.lua` 895, 919), so an absent one
/// is on. Nothing settles their names; settling one must read absent as
/// true ([`DEFAULT_ON`]).
pub const NAMED_ONLY: &[(&str, &[&str])] = &[
    (
        "DetonatedMinesRecently",
        &["Condition:DetonatedMinesRecently"],
    ),
    ("OverloadedIntensity", &["Multiplier:OverloadedIntensity"]),
    ("ScorchStacks", &["enemy Multiplier:ScorchStacks"]),
    ("ShockStacks", &["enemy Multiplier:ShockStacks"]),
    (
        "arcaneCloakUsedRecentlyCheck",
        &["Condition:ArcaneCloakUsedRecently"],
    ),
    (
        "aspectOfLunarisLunarProtection",
        &["Multiplier:LunarProtection"],
    ),
    (
        "aspectOfLunarisLunarResistance",
        &["Multiplier:LunarResistance"],
    ),
    (
        "aspectOfSolarisSunscaldStacks",
        &["Multiplier:SunscaldStack"],
    ),
    ("buffElusive", &["Condition:Elusive", "Elusive"]),
    (
        "buffFortification",
        &["Condition:Fortified", "Fortified", "FortificationStacks"],
    ),
    (
        "buffNgamahuFlamesAdvance",
        &["Condition:NgamahuFlamesAdvance"],
    ),
    (
        "carrionGolemNearbyMinion",
        &["Multiplier:NearbyNonGolemMinion"],
    ),
    ("changedStance", &["Condition:ChangedStanceRecently"]),
    (
        "conditionAttackedRecently",
        &["Condition:AttackedRecently", "Condition:UsedSkillRecently"],
    ),
    (
        "conditionCastSpellRecently",
        &["Condition:CastSpellRecently", "Condition:UsedSkillRecently"],
    ),
    (
        "conditionCausedBleedingRecently",
        &["Condition:CausedBleedingRecently"],
    ),
    (
        "conditionChampionIntimidate",
        &["enemy Condition:ChampionIntimidate"],
    ),
    (
        "conditionChilledEnemyRecently",
        &["Condition:ChilledEnemyRecently"],
    ),
    (
        "conditionConsumedCorpseInPast2Sec",
        &["Condition:ConsumedCorpseInPast2Sec"],
    ),
    (
        "conditionConsumedCorpseRecently",
        &["Condition:ConsumedCorpseRecently"],
    ),
    ("conditionEnemyBleeding", &["enemy Condition:Bleeding"]),
    ("conditionEnemyBurning", &["enemy Condition:Burning"]),
    ("conditionEnemyCursed", &["enemy Condition:Cursed"]),
    (
        "conditionEnemyDebilitated",
        &["enemy Condition:Debilitated"],
    ),
    ("conditionEnemyHindered", &["enemy Condition:Hindered"]),
    ("conditionEnemyMaimed", &["enemy Condition:Maimed"]),
    (
        "conditionEnemyMalignantMadness",
        &["enemy Condition:MalignantMadness"],
    ),
    ("conditionEnemyPoisoned", &["enemy Condition:Poisoned"]),
    ("conditionEnemyTaunted", &["enemy Condition:Taunted"]),
    ("conditionFocused", &["Condition:Focused"]),
    ("conditionFullLife", &["Condition:FullLife"]),
    ("conditionHaveEnergyShield", &["Condition:HaveEnergyShield"]),
    ("conditionHitRecently", &["Condition:HitRecently"]),
    (
        "conditionHitSpellRecently",
        &["Condition:HitSpellRecently", "Condition:HitRecently"],
    ),
    ("conditionIgnited", &["Condition:Ignited"]),
    ("conditionImpaledRecently", &["Condition:ImpaledRecently"]),
    (
        "conditionKilledUniqueEnemy",
        &["Condition:KilledUniqueEnemy"],
    ),
    ("conditionLeeching", &["Condition:Leeching"]),
    ("conditionLowLife", &["Condition:LowLife"]),
    (
        "conditionSacrificeMinion",
        &["Condition:SacrificeMinionOnAttack"],
    ),
    (
        "conditionSpawnedCorpseRecently",
        &["Condition:SpawnedCorpseRecently"],
    ),
    (
        "conditionSummonedTotemRecently",
        &["Condition:SummonedTotemRecently"],
    ),
    (
        "conditionTotemsHitSpellRecently",
        &[
            "Condition:TotemsHitSpellRecently",
            "Condition:TotemsHitRecently",
        ],
    ),
    (
        "conditionUsedMinionSkillRecently",
        &[
            "Condition:UsedMinionSkillRecently",
            "Condition:UsedSkillRecently",
        ],
    ),
    (
        "conditionUsedWarcryInPast8Seconds",
        &["Condition:UsedWarcryInPast8Seconds"],
    ),
    (
        "conditionUsedWarcryRecently",
        &[
            "Condition:UsedWarcryRecently",
            "Condition:UsedWarcryInPast8Seconds",
            "Condition:UsedSkillRecently",
        ],
    ),
    ("conditionUsingFlask", &["Condition:UsingFlask"]),
    ("configResonanceCount", &["Multiplier:ResonanceCount"]),
    (
        "configSpectralTigerCount",
        &[
            "Multiplier:SpectralTigerConfig",
            "Multiplier:SpectralTigerCount",
        ],
    ),
    (
        "enemyMultiplierEnemyPresenceSeconds",
        &["enemy Multiplier:EnemyPresenceSeconds"],
    ),
    (
        "heraldOfTheHivePressure",
        &["Multiplier:OtherworldlyPressure"],
    ),
    (
        "meatShieldEnemyNearYou",
        &["Condition:MeatShieldEnemyNearYou"],
    ),
    ("meleeDistance", &["skillDist"]),
    (
        "multiplierBeenHitRecently",
        // Its condition is settled with the check's (P6-10b part 13,
        // `poe1::conditions`).
        &["Multiplier:BeenHitRecently"],
    ),
    (
        "multiplierChannelling",
        &["Multiplier:ChannellingTime", "Condition:Channelling"],
    ),
    (
        "multiplierCorpseConsumedRecently",
        &[
            "Multiplier:CorpseConsumedRecently",
            "Condition:ConsumedCorpseRecently",
        ],
    ),
    (
        "multiplierEnemyHallowingFlame",
        &["enemy Multiplier:HallowingFlame"],
    ),
    ("multiplierGaleForce", &["Multiplier:GaleForce"]),
    (
        "multiplierHallowingFlameStacksRemovedByAlly",
        &["Multiplier:HallowingFlameStacksRemovedByAlly"],
    ),
    (
        "multiplierKilledRecently",
        // Its condition is settled with the check's (P6-11p,
        // `poe1::conditions`).
        &["Multiplier:EnemyKilledRecently"],
    ),
    (
        "multiplierManaSpentRecently",
        &["Multiplier:ManaSpentRecently"],
    ),
    (
        "multiplierMineDetonatedRecently",
        &["Multiplier:MineDetonatedRecently"],
    ),
    ("multiplierNearbyCorpse", &["Multiplier:NearbyCorpse"]),
    (
        "multiplierNearbyRareOrUniqueEnemies",
        // Its share of `Multiplier:NearbyEnemies` is `poe1::stacks`'s
        // (P6-11v).
        &[
            "Multiplier:NearbyRareOrUniqueEnemies",
            "Condition:AtMostOneNearbyRareOrUniqueEnemy",
            "enemy Condition:NearbyRareOrUniqueEnemy",
        ],
    ),
    ("multiplierSoulEater", &["Multiplier:SoulEaterStack"]),
    ("multiplierSummonedMinion", &["Multiplier:SummonedMinion"]),
    (
        "multiplierWarcryUsedRecently",
        &[
            "Multiplier:WarcryUsedRecently",
            "Condition:UsedWarcryRecently",
            "Condition:UsedWarcryInPast8Seconds",
            "Condition:UsedSkillRecently",
        ],
    ),
    (
        "multiplierWitheredStackCount",
        &["enemy Multiplier:WitheredStack"],
    ),
    (
        "multiplierWitheredStackCountSelf",
        &["Multiplier:WitheredStack"],
    ),
    ("overrideFortification", &["FortificationStacks"]),
    (
        "perforateSpikeOverlap",
        &["Multiplier:PerforateSpikeOverlap"],
    ),
    (
        "plagueBearerState",
        &[
            "Condition:PlagueBearerIncubating",
            "Condition:PlagueBearerInfecting",
        ],
    ),
    ("prideEffect", &["Condition:PrideMaxEffect"]),
    ("projectileDistance", &["skillDist"]),
    (
        "raiseSpidersSpiderCount",
        &["Multiplier:RaisedSpiderConfig", "Multiplier:RaisedSpider"],
    ),
    ("sigilOfPowerStages", &["Multiplier:SigilOfPowerStage"]),
];

/// The enemy's damage (PoB v2.67.2, `Modules/ConfigOptions.lua` 2352 to
/// 2376), sorted (a test checks): inputs with no `apply`, which only
/// `calcs.buildDefenceEstimations` reads (`CalcDefence.lua` 1737 to 1757,
/// 2965, 3552: damage taken, effective life, maximum hit; no slice field)
/// as an input or placeholder. `enemyDamageRollRange` is also read by
/// `presetBossSkills`'s `apply` (2286), which only sets the damage
/// placeholders. No effect for an input or a placeholder.
///
/// Added in P6-4a, the other inputs only that estimate reads:
/// `DisableEHPGainOnBlock` (`CalcDefence.lua` 2760, 2793) and
/// `enemyDamageType` (1643), neither with an `apply`.
pub const ENEMY_DAMAGE: &[&str] = &[
    "DisableEHPGainOnBlock",
    "enemyChaosDamage",
    "enemyColdDamage",
    "enemyColdPen",
    "enemyCritChance",
    "enemyCritDamage",
    "enemyDamageRollRange",
    "enemyDamageType",
    "enemyFireDamage",
    "enemyFirePen",
    "enemyLightningDamage",
    "enemyLightningPen",
    "enemyPhysicalDamage",
    "enemyPhysicalOverwhelm",
    "enemySpeed",
];

/// The game stats through which PoB turns a [`NAMED_ONLY`] input into a
/// slice field, sorted (a test checks the order, and that each is
/// `Unmapped` through every mapping): the argument holds only while each
/// is a reason. PoB's own text for each (`ModParser.lua`) or skill data:
///
/// - `additional_spell_block_%_if_havent_blocked_recently`,
///   `chance_to_block_attack_damage_if_not_blocked_recently_%`,
///   `max_chance_to_block_attacks_if_not_blocked_recently`,
///   `spell_block_is_maximum_if_not_blocked_recently`: block conditioned
///   on not `BlockedRecently` (4146, 4147; `CalcDefence.lua` 707, 723).
/// - `your_consecrated_ground_grants_accuracy_rating_+%`:
///   `ConsecratedGroundAlsoAccuracy` (4176), read with
///   `conditionOnConsecratedGround`. (The enemy's consecrated ground and
///   Convergence left in P3-8n: [`super::damage_taken`], [`super::buffs`].)
/// - `ignites_apply_fire_resistance_+`: Combustion's fire resistance on an
///   ignited enemy (`CalcPerform.lua` 2709 to 2716); `conditionEnemyIgnited`
///   is [`super::damage_taken`]'s since P3-8n, and Combustion's support is
///   not modelled.
/// - Chill's damage taken flags and Bonechill's stats: moved in P3-8m to
///   [`super::keystones`], [`super::damage_taken::CHILL_STATS`] (mapped),
///   and [`super::damage_taken::CHILL_DEPENDS_ON`], with the chill inputs.
///
/// Added in P3-12b:
///
/// - `can_apply_additional_scorch`, `raider_shocks_you_apply_can_stack_up_to_50_times`:
///   `ScorchCanStack`, `ShockCanStack` (`ModParser.lua` 3045, 3047).
/// - The distance modifiers (`DistanceRamp`, `MeleeProximity`):
///   `arrow_critical_strike_chance_+%_max_as_distance_travelled_increases`,
///   `arrow_damage_+%_max_as_distance_travelled_increases`,
///   `deadeye_projectile_damage_+%_final_max_as_distance_travelled_increases`,
///   `impale_effect_+%_max_as_distance_travelled_increases`,
///   `projectile_damage_+%_max_as_distance_travelled_increases`,
///   `projectile_damage_+%_max_before_distance_increase`,
///   `slayer_damage_+%_final_from_distance` (`ModParser.lua` 2511, 2662,
///   3013, 3917, 4638 to 4640); `keystone_point_blank`, `player_far_shot`,
///   `gain_player_far_shot_while_do_not_have_iron_reflexes`,
///   `local_display_socketed_gems_supported_by_level_x_point_blank` (the
///   `PointBlank` and `FarShot` flags, 2317, 2661, 5489; the last through
///   the Point Blank support, `sup_dex.lua`); and the skill
///   stats `lancing_steel_damage_+%_at_close_range`,
///   `shattering_steel_hit_damage_+%_final_scaled_by_projectile_distance_per_ammo_consumed`,
///   `shield_crush_damage_+%_final_from_distance`,
///   `support_slashing_damage_+%_final_from_distance` (the skills' stat
///   maps, `Data/Skills/act_dex.lua`, `act_str.lua`, `sup_dex.lua`).
///   Freezing Pulse and Eye of Winter add theirs in their own skill code
///   (`act_int.lua`), outside any stat: those skills are not modelled.
/// - `physical_damage_%_to_add_as_lightning_damage_per_hallowing_flame_consumed_by_ally_up_to_80%`:
///   reads `Multiplier:HallowingFlameStacksRemovedByAlly` (`ModParser.lua`
///   2898).
///
/// Added in P3-12f (PoB's lines in [`NAMED_ONLY`]'s rows), each found by
/// its description in the bundle's stat descriptions (the game's text)
/// and matched to PoB's text or skill data:
///
/// - rage: the stats that gain or regenerate rage (`gain_rage_on_hit`,
///   `gain_x_rage_on_*`, `regenerate_1_rage_per_x_*_regeneration`,
///   `warcry_grant_X_rage_per_5_power`, and the others listed), and the
///   skill stats of Chain Hook, Vengeful Cry, a curse's stun rage, Rage
///   support, and the Spirit of Akoya tattoo;
/// - exposure: moved in P3-8k to [`super::exposure`] (its rows and its
///   `DEPENDS_ON`), which gives the exposure checks their effect.
///
/// Each is a stat of the real bundle (a maintainer-run test,
/// `tests/enemy.rs`), so a misspelt ID cannot pass as `Unmapped`.
pub const DEPENDS_ON: &[&str] = &[
    "additional_spell_block_%_if_havent_blocked_recently",
    "ancestral_buff_base_rage_regeneration_per_minute",
    "arrow_critical_strike_chance_+%_max_as_distance_travelled_increases",
    "arrow_damage_+%_max_as_distance_travelled_increases",
    "berserker_gain_rage_on_attack_hit_cooldown_ms",
    "berserker_warcry_grant_X_rage_per_5_power_while_less_than_25_rage",
    "can_apply_additional_scorch",
    "chain_hook_attachment_rage_to_gain_per_hit",
    "chain_strike_gain_x_rage_if_attack_hits",
    "chance_to_block_attack_damage_if_not_blocked_recently_%",
    "deadeye_projectile_damage_+%_final_max_as_distance_travelled_increases",
    "enemy_grant_X_rage_on_stun",
    "enemy_rage_regeneration_on_stun",
    "gain_1_rage_on_use_%_chance",
    "gain_max_rage_on_losing_temporal_chains_debuff",
    "gain_player_far_shot_while_do_not_have_iron_reflexes",
    "gain_rage_on_hit",
    "gain_rage_on_hit_%_chance",
    "gain_rage_on_hitting_rare_unique_enemy_%",
    "gain_rage_on_hitting_rare_unique_enemy_with_tinctured_weapon_%",
    "gain_rage_on_kill",
    "gain_rage_when_you_use_a_warcry",
    "gain_x_rage_on_attack_crit",
    "gain_x_rage_on_attack_hit",
    "gain_x_rage_on_bow_hit",
    "gain_x_rage_on_hit_with_axes",
    "gain_x_rage_on_hit_with_axes_swords",
    "gain_x_rage_on_hit_with_axes_swords_1s_cooldown",
    "gain_x_rage_on_hit_with_tinctured_weapons",
    "gain_x_rage_on_melee_crit",
    "gain_x_rage_per_200_mana_spent",
    "gain_x_rage_when_hit",
    "gain_x_rage_when_you_use_a_life_flask",
    "glove_implicit_gain_rage_on_attack_hit_cooldown_ms",
    "ignites_apply_fire_resistance_+",
    "impale_effect_+%_max_as_distance_travelled_increases",
    "kaoms_primacy_gain_rage_on_attack_crit_cooldown_ms",
    "keystone_point_blank",
    "lancing_steel_damage_+%_at_close_range",
    "local_display_socketed_gems_supported_by_level_x_point_blank",
    "local_display_tattoo_trigger_level_x_akoya",
    "local_gain_X_rage_on_attack_hit_with_melee_abyss_jewel_socketed",
    "max_chance_to_block_attacks_if_not_blocked_recently",
    "physical_damage_%_to_add_as_lightning_damage_per_hallowing_flame_consumed_by_ally_up_to_80%",
    "player_far_shot",
    "projectile_damage_+%_max_as_distance_travelled_increases",
    "projectile_damage_+%_max_before_distance_increase",
    "rage_warcry_gain_X_rage_per_minute_per_5_monster_power_max_25_power",
    "rage_warcry_maximum_rage_+",
    "raider_shocks_you_apply_can_stack_up_to_50_times",
    "regenerate_1_rage_per_x_life_regeneration",
    "regenerate_1_rage_per_x_mana_regeneration",
    "retaliation_skills_gain_X_rage_per_enemy_hit",
    "shattering_steel_hit_damage_+%_final_scaled_by_projectile_distance_per_ammo_consumed",
    "shield_crush_damage_+%_final_from_distance",
    "slayer_damage_+%_final_from_distance",
    "spell_block_is_maximum_if_not_blocked_recently",
    "support_rage_gain_rage_on_melee_hit_cooldown_ms",
    "support_slashing_damage_+%_final_from_distance",
    "warcry_grant_X_rage_per_5_power",
    "you_and_nearby_party_members_gain_x_rage_when_you_warcry",
    "your_consecrated_ground_grants_accuracy_rating_+%",
];

/// The value of the last input named `name` of `kind` in `set`: PoB keeps
/// a set's inputs in a table, so the last one read counts
/// (`Classes/ConfigTab.lua`, `Load`, 850 to 876).
#[must_use]
pub fn last_input<'a>(
    set: &'a ConfigSet,
    name: &str,
    kind: ConfigInputKind,
) -> Option<&'a ConfigValue> {
    set.inputs
        .typed()
        .filter(|i| i.kind == kind && i.name == name)
        .last()
        .map(|i| &i.value)
}

/// Whether `input` only sets named conditions or multipliers that
/// modifiers read ([`NAMED_ONLY`]).
#[must_use]
pub fn named_only(input: &str) -> bool {
    NAMED_ONLY
        .binary_search_by(|(name, _)| (*name).cmp(input))
        .is_ok()
}

/// The [`NAMED_ONLY`] inputs whose option is `defaultState = true`
/// (`ConfigOptions.lua` 1475, 1555): absent is on in PoB, sorted.
pub const DEFAULT_ON: &[&str] = &["conditionChampionIntimidate", "conditionSacrificeMinion"];

/// Inputs PoB v2.67.2 neither defines (`Modules/ConfigOptions.lua`) nor
/// reads (`configInput`, `input`, in any module), sorted (P6-4a): saved by
/// other versions of PoB, they change nothing in v2.67.2. `buffFortify`
/// is only written by `ConfigTab:ImportCalcSettings` (1165) from an old
/// calcs tab, and no option reads it (fortification is
/// `buffFortification`). No effect for an input or a placeholder.
pub const INERT: &[&str] = &[
    "brandAttachedToEnemy",
    "buffFortify",
    "enemyAwakeningLevel",
    "igniteMode",
];

/// Whether PoB v2.67.2 ignores `input` ([`INERT`]).
#[must_use]
pub fn inert(input: &str) -> bool {
    INERT.binary_search(&input).is_ok()
}

/// Whether `input` is the enemy's damage ([`ENEMY_DAMAGE`]).
#[must_use]
pub fn enemy_damage(input: &str) -> bool {
    ENEMY_DAMAGE.binary_search(&input).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_should_be_sorted_and_unique() {
        assert!(NAMED_ONLY.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(ENEMY_DAMAGE.windows(2).all(|w| w[0] < w[1]));
        assert!(INERT.windows(2).all(|w| w[0] < w[1]));
        for input in INERT {
            assert!(inert(input) && !named_only(input) && !enemy_damage(input));
        }
        assert!(NAMED_ONLY.iter().all(|(_, names)| !names.is_empty()));
    }

    #[test]
    fn named_only_should_find_listed_inputs_alone() {
        for (input, _) in NAMED_ONLY {
            assert!(named_only(input), "{input}");
        }
        // Inputs with their own effects stay reasons.
        for input in [
            "useFrenzyCharges",
            "usePowerCharges",
            "useEnduranceCharges",
            "conditionEnemyShocked",
            "conditionShockEffect",
            "summonLightningGolemEnableWrath",
            "bloodSandStance",
            "customMods",
            "bandit",
            "pantheonMajorGod",
            "pantheonMinorGod",
            "enemyIsBoss",
            "resistancePenalty",
        ] {
            assert!(!named_only(input), "{input}");
        }
    }

    #[test]
    fn the_stats_the_rule_depends_on_should_stay_reasons() {
        use crate::ingest::{StatMap, StatMapping};
        use crate::poe1::ArmourSlot;
        use crate::poe1::mapping::{PieceMapping, Poe1Mapping, SkillMapping, WeaponMapping};
        use loom_domain::StatId;

        assert!(DEPENDS_ON.windows(2).all(|w| w[0] < w[1]));
        let piece = PieceMapping {
            slot: ArmourSlot::Helmet,
        };
        let mappings: [&dyn StatMapping; 4] = [&Poe1Mapping, &SkillMapping, &piece, &WeaponMapping];
        for stat in DEPENDS_ON {
            for m in mappings {
                assert_eq!(
                    m.map(&StatId::new(*stat)),
                    StatMap::Unmapped,
                    "{stat}: mapping it must revisit NAMED_ONLY"
                );
            }
        }
    }

    /// No name a [`NAMED_ONLY`] input sets is a condition the engine
    /// settles (P6-4a review): a settled condition the config sets would be
    /// left at the engine's value, so such an input would need its effect.
    #[test]
    fn no_named_only_name_should_be_settled() {
        use crate::poe1::{conditions, gems, keystones, mapping, states};
        let mut settled: Vec<&str> = conditions::CHECKS.iter().map(|(_, c)| *c).collect();
        settled.extend([
            states::USING_STAFF,
            states::USING_SHIELD,
            states::NO_LIFE_MODIFIERS_ON_BODY_ARMOUR,
            keystones::CRIT_IN_PAST_8_SEC,
            gems::STAT_THRESHOLD,
            gems::LIFE_THRESHOLD,
        ]);
        for (_, rows) in mapping::STATE_ROWS {
            settled.extend(rows.iter().map(|r| r.3.trim_start_matches('!')));
        }
        for (input, names) in NAMED_ONLY {
            for name in *names {
                // `Condition:X` is `X`; the enemy's is `Enemy:X`.
                let key = if let Some(rest) = name.strip_prefix("enemy Condition:") {
                    format!("Enemy:{rest}")
                } else {
                    name.trim_start_matches("Condition:").to_owned()
                };
                assert!(
                    !settled.contains(&key.as_str()),
                    "{input}: {name} is settled by the engine"
                );
            }
        }
        for input in DEFAULT_ON {
            assert!(named_only(input), "{input}");
        }
        assert!(DEFAULT_ON.windows(2).all(|w| w[0] < w[1]));
    }

    /// Fortification (P3-12e): every reader of its names is a modifier,
    /// and no mapped stat reads or changes it (the `NoEffect` rows that
    /// name it are listed and reach no slice field).
    #[test]
    fn no_mapped_stat_should_read_fortification() {
        use crate::poe1::mapping::{HIT_ROWS, LOCAL_ROWS, ROWS, WEAPON_ROWS};

        let ids = HIT_ROWS
            .iter()
            .map(|r| r.0)
            .chain(ROWS.iter().map(|r| r.0))
            .chain(LOCAL_ROWS.iter().map(|r| r.0))
            .chain(WEAPON_ROWS.iter().map(|r| r.0));
        // P6-10b part 12: the maximum, which the enemy's hit reads while
        // Fortified (`poe1::enemy_hit`, reading the check itself).
        for id in ids {
            assert!(
                !id.contains("fortif") || id == "base_max_fortification",
                "{id}: revisit NAMED_ONLY"
            );
        }
        // `NoEffect` rows naming fortification, each checked: a chance to
        // fortify, its duration; none reaches a slice field. Regeneration
        // per stack is a `mapping::REGEN_REASONS` entry (P6-10b): the life
        // regeneration field's reason, never a value.
        let checked = ["chance_to_fortify_on_melee_hit_+%", "fortify_duration_+%"];
        for (id, _) in crate::poe1::mapping::NO_EFFECT {
            assert!(
                !id.contains("fortif") || checked.contains(id),
                "{id}: a NoEffect row naming fortification; check it and list it"
            );
        }
    }

    /// Rage and exposure (P3-12f): no row of the shared tables names them,
    /// and the `NoEffect` rows that do are listed, each reaching no slice
    /// field. Exposure's own rows are `poe1::exposure::STATS` (P3-8k),
    /// read only by `exposure()`.
    #[test]
    fn no_mapped_stat_should_name_rage_or_exposure() {
        use crate::poe1::mapping::{HIT_ROWS, LOCAL_ROWS, NO_EFFECT, ROWS, WEAPON_ROWS};

        let names = |id: &str| {
            id.split(|c: char| !c.is_ascii_alphanumeric())
                .any(|w| w.eq_ignore_ascii_case("rage") || w.eq_ignore_ascii_case("exposure"))
        };
        let ids = HIT_ROWS
            .iter()
            .map(|r| r.0)
            .chain(ROWS.iter().map(|r| r.0))
            .chain(LOCAL_ROWS.iter().map(|r| r.0))
            .chain(WEAPON_ROWS.iter().map(|r| r.0));
        for id in ids {
            assert!(!names(id), "{id}: revisit NAMED_ONLY");
        }
        // Rage's loss delay (PoB `InherentRageLossDelay`): rage's loss,
        // never its gain or effect.
        let checked = ["rage_loss_delay_ms"];
        for (id, _) in NO_EFFECT {
            assert!(
                !names(id) || checked.contains(id),
                "{id}: a NoEffect row naming rage or exposure; check it and list it"
            );
        }
        assert!(!names("average_damage"));
    }

    #[test]
    fn the_table_should_not_overlap_the_modelled_inputs() {
        for (input, _) in NAMED_ONLY {
            assert_ne!(*input, super::super::collect::RESISTANCE_PENALTY_INPUT);
            assert!(
                !super::super::enemy::ENEMY_INPUTS.contains(input),
                "{input}"
            );
        }
    }

    #[test]
    fn the_enemy_damage_should_be_sorted_and_apart_from_the_other_tables() {
        use super::super::enemy::{ENEMY_INPUTS, RESET_PLACEHOLDERS};

        assert!(ENEMY_DAMAGE.windows(2).all(|w| w[0] < w[1]));
        for input in ENEMY_DAMAGE {
            assert!(enemy_damage(input), "{input}");
            assert!(!named_only(input), "{input}");
            assert!(!ENEMY_INPUTS.contains(input), "{input}");
            assert!(!RESET_PLACEHOLDERS.contains(input), "{input}");
        }
        for input in RESET_PLACEHOLDERS {
            assert!(!named_only(input), "{input}");
            assert!(!enemy_damage(input), "{input}");
        }
        assert!(!enemy_damage("enemyIsBoss"));
        assert!(!enemy_damage("enemyArmour"));
    }
}
