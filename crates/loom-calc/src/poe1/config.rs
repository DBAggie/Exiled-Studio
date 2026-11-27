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
/// - `conditionBeenHitRecently`, `conditionChilledEnemyRecently`: the
///   condition of that name (lines 1375, 1339; `conditionFullLife` moved
///   to [`super::conditions`] in P6-11ai; the blocked-recently checks moved to [`super::conditions`] in
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
///   field). Life regeneration per stack reads the same stacks since
///   P6-11bu ([`super::regen`]). Every other reader is a modifier (while
///   fortified, at maximum fortification, per buff on you).
///
/// Added in P3-12f, inputs whose effect needs a condition only modifiers
/// (or another config input, itself a reason) set:
///
/// - `multiplierRage` was here until P3-8n, which gives rage its effect
///   with "Gain # Rage on Melee Hit" ([`super::stacks`]), and "Gain # Rage
///   on Attack Hit" since P6-11bc (PoB reads both as `CanGainRage`). The other stats
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
/// - `conditionUsedMinionSkillRecently` (1526) left this list in P6-11cm,
///   `conditionAttackedRecently` and `conditionCastSpellRecently` (1485,
///   1489) in P6-11em: checks `poe1::conditions` settles, read by
///   modifiers. Their implied `UsedSkillRecently` is no row's.
/// - `conditionKilledRecently`: `Condition:KilledRecently` (line 1270);
///   only `CalcTriggers.lua` (1136, 1156) reads it, for trigger skills
///   (other gems, reasons).
/// - `conditionLeeching`: `Condition:Leeching` (line 1068); no calc module
///   reads it.
/// - `conditionEnemyBleeding`, `conditionEnemyPoisoned`: the enemy's
///   `Condition:Bleeding`, `Condition:Poisoned` (lines 1786, 1798); no
///   calc module reads them on the enemy (`CalcSetup.lua` 73 sets
///   `Poisoned` from poison stacks). Close-out M1 moved
///   `conditionEnemyPoisoned`, `conditionEnemyTaunted`,
///   `conditionHitSpellRecently`, and `conditionTotemsHitSpellRecently` to
///   `poe1::conditions` (rows read them; their implied `HitRecently` and
///   `TotemsHitRecently` are no row's). `conditionEnemyLowLife` left this
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
///   pools of the defence estimate, which `poe1::enemy_hit` reads from the
///   input since P6-11ad), `LowEnergyShield` (the input itself caps the
///   energy shield pool, `CalcDefence.lua` 1061, read the same way), `Lifetap`'s `FlaskLifeRecovery`, `Elusive` (avoidance and
///   movement speed 808 to 810; Nightblade's crit, `CalcOffence.lua` 521,
///   with a support that is not modelled), `KilledRecently` (trigger skills, `CalcTriggers.lua` 1136), and the
///   player's `Ignited` (its `Burning`, `CalcSetup.lua` 72). The enemy's
///   `Withered` stacks (`multiplierWitheredStackCount`) are
///   [`super::damage_taken`]'s since P6-11aq (no longer in this list).
///
/// Added in open items S8:
///
/// - `ActiveBrands`: `Multiplier:ConfigActiveBrands` (line 321), which
///   `CalcPerform.lua` (1327 to 1330) reads only for an active brand skill
///   to set `Multiplier:ActiveBrand`, capped by its brand limit; only
///   modifiers read that ("per brand", `ModParser.lua` 1342, 1343: the
///   stats in [`DEPENDS_ON`]). A brand's own attached count is
///   `BrandsAttachedToEnemy`, another input.
/// - `buffAlchemistsGenius`: `Condition:AlchemistsGenius`, only while
///   `Condition:CanHaveAlchemistGenius` (line 1044). `CalcSetup.lua` (106)
///   makes it a flag and `CalcPerform.lua` (1290 to 1294) gives flask
///   effect and flask charges gained; but only a modifier sets
///   `CanHaveAlchemistGenius` ("Gain Alchemist's Genius when you use a
///   Flask", `ModParser.lua` 5102 to 5107: the stat
///   `gain_alchemists_genius_on_flask_use_%`, in [`DEPENDS_ON`]). Without
///   it the input does nothing.
/// - `conditionEnemyOnBrittleGround`, `conditionEnemyOnSappedGround`,
///   `conditionEnemyOnScorchedGround`: the enemy's `Condition:<ailment>` and
///   `Condition:On<ailment>Ground` (lines 1869 to 1872, 1913 to 1916, 1940
///   to 1943). The ailment table (`CalcPerform.lua` 3534 to 3546) adds the
///   ailment's effect only from `<ailment>Val`, which the effect input sets
///   only with the check's `<ailment>Config` condition, or from
///   `<ailment>Base`, `Override`, `Minimum` modifiers (unmapped stats): so a
///   ground alone gives an effect of 0, and its conditions are read only by
///   modifiers. The checks themselves have effects
///   (`super::damage_taken`: scorch and sap; brittle stays a reason).
/// - `multiplierNearbyAlly` (the agent list, S8): `Multiplier:NearbyAlly` in
///   combat (line 1122). The calc modules read it only for Rallying Cry's
///   exerted attacks (`CalcOffence.lua` 2661), which `super::gems` gives
///   its own reason (`rallying-cry-allies`) when allies are configured;
///   Rallying Cry's buff row (`act_str.lua` 9238) is a `Dummy`. Only
///   modifiers read it otherwise (`ModParser.lua` 1584, 1701, 2871, 2872:
///   "while you have at least # nearby allies", "while there is at least
///   one nearby ally"), whose stats, found by "nearby Ally" in the
///   bundle's stat descriptions, are in [`DEPENDS_ON`].
/// - Elemental Equilibrium's inputs (the agent list, S8):
///   `enemyConditionHitByColdDamage`, `enemyConditionHitByFireDamage`, and
///   `enemyConditionHitByLightningDamage` set the enemy's
///   `Condition:HitBy<element>Damage` (lines 2022 to 2030), and
///   `EEIgnoreHitDamage` (2034) has no `apply`. Only Elemental
///   Equilibrium's lines read the conditions (`ModParser.lua` 2287 to
///   2300: its resistances and exposure), and the calc modules read them
///   and the check only under its `ElementalEquilibrium` flag
///   (`CalcOffence.lua` 3605, `CalcPerform.lua` 3565 to 3570), which only
///   those lines set. Their stats are unmapped and in
///   [`super::exposure::DEPENDS_ON`] (`keystone_elemental_equilibrium`,
///   `elemental_equilibrium_effect_+%`, the socketed gems' line).
/// - `TotalSpectreLife` (the agent list, S8): the spectres' life pool as an
///   `OVERRIDE` (line 797), not a name, but read only where a name would
///   be: the calc modules read the pool only with its redirect,
///   `takenFromSpectresBeforeYou` (`CalcDefence.lua` 2458 to 2470, the
///   pool's life read when the redirect is not 0; `CalcPerform.lua` 3600 to
///   3690, the pool built only with it, and shared with Companionship's
///   only when both redirects are set), which only "#% of Damage from Hits
///   is taken from your Raised Spectres' Life before you"
///   (`ModParser.lua` 4992) sets: its stat,
///   `damage_removed_from_spectres_before_life_or_es_%`, is unmapped and in
///   [`DEPENDS_ON`].
///
/// `conditionChampionIntimidate` and `conditionSacrificeMinion` are
/// `defaultState = true` options: PoB saves such an input only when it
/// differs from the default (`ConfigTab.lua` 895, 919), so an absent one
/// is on. Nothing settles their names; settling one must read absent as
/// true ([`DEFAULT_ON`]).
pub const NAMED_ONLY: &[(&str, &[&str])] = &[
    (
        "ActiveBrands",
        &["Multiplier:ConfigActiveBrands", "Multiplier:ActiveBrand"],
    ),
    (
        "DetonatedMinesRecently",
        &["Condition:DetonatedMinesRecently"],
    ),
    // No `apply`: `env.configInput`, read only under the
    // `ElementalEquilibrium` flag.
    ("EEIgnoreHitDamage", &["configInput.EEIgnoreHitDamage"]),
    ("OverloadedIntensity", &["Multiplier:OverloadedIntensity"]),
    ("ScorchStacks", &["enemy Multiplier:ScorchStacks"]),
    ("ShockStacks", &["enemy Multiplier:ShockStacks"]),
    // Not a condition: the spectre life pool's `OVERRIDE`, read only with the
    // spectres' redirect (module docs of this table).
    ("TotalSpectreLife", &["TotalSpectreLife OVERRIDE"]),
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
    (
        "buffAlchemistsGenius",
        &["Condition:AlchemistsGenius if Condition:CanHaveAlchemistGenius"],
    ),
    (
        "buffFortification",
        &["Condition:Fortified", "Fortified", "FortificationStacks"],
    ),
    (
        "carrionGolemNearbyMinion",
        &["Multiplier:NearbyNonGolemMinion"],
    ),
    ("changedStance", &["Condition:ChangedStanceRecently"]),
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
    ("conditionEnemyHindered", &["enemy Condition:Hindered"]),
    (
        "conditionEnemyOnBrittleGround",
        &["enemy Condition:Brittle", "enemy Condition:OnBrittleGround"],
    ),
    (
        "conditionEnemyOnSappedGround",
        &["enemy Condition:Sapped", "enemy Condition:OnSappedGround"],
    ),
    (
        "conditionEnemyOnScorchedGround",
        &[
            "enemy Condition:Scorched",
            "enemy Condition:OnScorchedGround",
        ],
    ),
    ("conditionHaveEnergyShield", &["Condition:HaveEnergyShield"]),
    ("conditionHitRecently", &["Condition:HitRecently"]),
    ("conditionImpaledRecently", &["Condition:ImpaledRecently"]),
    ("conditionLowEnergyShield", &["Condition:LowEnergyShield"]),
    (
        "conditionSacrificeMinion",
        &["Condition:SacrificeMinionOnAttack"],
    ),
    (
        "conditionSpawnedCorpseRecently",
        &["Condition:SpawnedCorpseRecently"],
    ),
    (
        "configSpectralTigerCount",
        &[
            "Multiplier:SpectralTigerConfig",
            "Multiplier:SpectralTigerCount",
        ],
    ),
    (
        "enemyConditionHitByColdDamage",
        &["enemy Condition:HitByColdDamage"],
    ),
    (
        "enemyConditionHitByFireDamage",
        &["enemy Condition:HitByFireDamage"],
    ),
    (
        "enemyConditionHitByLightningDamage",
        &["enemy Condition:HitByLightningDamage"],
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
        &["Multiplier:CorpseConsumedRecently"],
    ),
    (
        "multiplierEnemyHallowingFlame",
        &["enemy Multiplier:HallowingFlame"],
    ),
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
        "multiplierMineDetonatedRecently",
        &["Multiplier:MineDetonatedRecently"],
    ),
    ("multiplierNearbyAlly", &["Multiplier:NearbyAlly"]),
    (
        "multiplierNearbyRareOrUniqueEnemies",
        // Its share of `Multiplier:NearbyEnemies` is `poe1::stacks`'s
        // (P6-11v); `Condition:AtMostOneNearbyRareOrUniqueEnemy` is
        // `poe1::conditions`' (P6-11ik).
        &[
            "Multiplier:NearbyRareOrUniqueEnemies",
            "enemy Condition:NearbyRareOrUniqueEnemy",
        ],
    ),
    ("multiplierSummonedMinion", &["Multiplier:SummonedMinion"]),
    (
        "multiplierWarcryUsedRecently",
        // Its `UsedWarcryRecently` is settled with the check's (P6-11ev,
        // `poe1::conditions`), and with it `UsedWarcryInPast8Seconds`
        // (P6-11hz); the others are no row's.
        &[
            "Multiplier:WarcryUsedRecently",
            "Condition:UsedSkillRecently",
        ],
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
///   [`super::gems`]' since P6-11hr (no longer listed).
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
///   Since P6-11ht the three `MeleeProximity` stats
///   (`shield_crush_damage_+%_final_from_distance`,
///   `slayer_damage_+%_final_from_distance`,
///   `support_slashing_damage_+%_final_from_distance`) are mapped and
///   scaled by the distance in `collect`, so they are no longer listed.
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
/// Added in open items S8: the "per Brand" stats (`ActiveBrand`, read by
/// `ModParser.lua` 1342 and 1343), found by the text "per Brand" in the
/// bundle's stat descriptions: `cooldown_speed_+%_per_brand_up_to_40%`,
/// `critical_strike_chance_+%_per_brand`,
/// `sigil_recall_cooldown_speed_+%_per_brand_up_to_40%`; and
/// `gain_alchemists_genius_on_flask_use_%`, the only source of
/// `CanHaveAlchemistGenius`. In the agent list (S8), the stats whose text
/// reads nearby allies (`multiplierNearbyAlly`, found by "nearby Ally" in
/// the bundle's stat descriptions): `accuracy_rating_+%_with_ally_nearby`,
/// `attack_area_of_effect_+%_with_ally_nearby`,
/// `damage_+%_final_with_at_least_1_nearby_ally`, the three Guardian
/// lines (`guardian_damage_+%_final_to_you_and_nearby_allies_per_nearby_ally_up_to_15%`,
/// `guardian_with_5_nearby_allies_you_and_allies_have_onslaught`,
/// `guardian_with_nearby_ally_damage_+%_final_for_you_and_allies`), and
/// Rallying Cry's `rallying_cry_damage_+%_final_from_osm_per_nearby_ally`;
/// `damage_removed_from_spectres_before_life_or_es_%`, the only source of
/// the spectres' redirect (`TotalSpectreLife`).
///
/// Each is a stat of the real bundle (a maintainer-run test,
/// `tests/enemy.rs`), so a misspelt ID cannot pass as `Unmapped`.
pub const DEPENDS_ON: &[&str] = &[
    "accuracy_rating_+%_with_ally_nearby",
    "additional_spell_block_%_if_havent_blocked_recently",
    "ancestral_buff_base_rage_regeneration_per_minute",
    "arrow_critical_strike_chance_+%_max_as_distance_travelled_increases",
    "arrow_damage_+%_max_as_distance_travelled_increases",
    "attack_area_of_effect_+%_with_ally_nearby",
    "berserker_gain_rage_on_attack_hit_cooldown_ms",
    "berserker_warcry_grant_X_rage_per_5_power_while_less_than_25_rage",
    "can_apply_additional_scorch",
    "chain_hook_attachment_rage_to_gain_per_hit",
    "chain_strike_gain_x_rage_if_attack_hits",
    "chance_to_block_attack_damage_if_not_blocked_recently_%",
    "cooldown_speed_+%_per_brand_up_to_40%",
    "critical_strike_chance_+%_per_brand",
    "damage_+%_final_with_at_least_1_nearby_ally",
    "damage_removed_from_spectres_before_life_or_es_%",
    "enemy_grant_X_rage_on_stun",
    "enemy_rage_regeneration_on_stun",
    "energy_shield_leech_does_not_stop_on_full_energy_shield",
    "energy_shield_leech_from_attacks_does_not_stop_on_full_energy_shield",
    "gain_1_rage_on_use_%_chance",
    "gain_alchemists_genius_on_flask_use_%",
    "gain_player_far_shot_while_do_not_have_iron_reflexes",
    "gain_rage_on_hit",
    "gain_rage_on_hit_%_chance",
    "gain_rage_on_hitting_rare_unique_enemy_%",
    "gain_rage_on_hitting_rare_unique_enemy_with_tinctured_weapon_%",
    "gain_rage_on_kill",
    "gain_rage_when_you_use_a_warcry",
    "gain_x_rage_on_attack_crit",
    "gain_x_rage_on_bow_hit",
    "gain_x_rage_on_hit_with_axes_swords_1s_cooldown",
    "gain_x_rage_on_melee_crit",
    "glove_implicit_gain_rage_on_attack_hit_cooldown_ms",
    "guardian_damage_+%_final_to_you_and_nearby_allies_per_nearby_ally_up_to_15%",
    "guardian_with_5_nearby_allies_you_and_allies_have_onslaught",
    "guardian_with_nearby_ally_damage_+%_final_for_you_and_allies",
    "impale_effect_+%_max_as_distance_travelled_increases",
    "kaoms_primacy_gain_rage_on_attack_crit_cooldown_ms",
    "keystone_point_blank",
    "lancing_steel_damage_+%_at_close_range",
    "life_leech_does_not_stop_at_full_life",
    "local_display_socketed_gems_supported_by_level_x_point_blank",
    "local_display_tattoo_trigger_level_x_akoya",
    "local_gain_X_rage_on_attack_hit_with_melee_abyss_jewel_socketed",
    "max_chance_to_block_attacks_if_not_blocked_recently",
    "physical_damage_%_to_add_as_lightning_damage_per_hallowing_flame_consumed_by_ally_up_to_80%",
    "player_far_shot",
    "projectile_damage_+%_max_before_distance_increase",
    "rage_warcry_gain_X_rage_per_minute_per_5_monster_power_max_25_power",
    "rage_warcry_maximum_rage_+",
    "raider_shocks_you_apply_can_stack_up_to_50_times",
    "rallying_cry_damage_+%_final_from_osm_per_nearby_ally",
    "regenerate_1_rage_per_x_life_regeneration",
    "regenerate_1_rage_per_x_mana_regeneration",
    "retaliation_skills_gain_X_rage_per_enemy_hit",
    "shattering_steel_hit_damage_+%_final_scaled_by_projectile_distance_per_ammo_consumed",
    "sigil_recall_cooldown_speed_+%_per_brand_up_to_40%",
    "spell_block_is_maximum_if_not_blocked_recently",
    "support_rage_gain_rage_on_melee_hit_cooldown_ms",
    "virtual_life_leech_does_not_stop_at_full_life",
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

/// Config inputs whose only effect is a `SkillData` value of named skills
/// (a `SkillName` tag) that only those skills' own skill functions read,
/// with those skills' IDs, sorted by input (open items, S8). Every such
/// skill is unvetted for its hit (its function, [`super::hit::pob_vetted`])
/// and its DoT ([`super::dot::pob_dot_vetted`]), and its ailments, leech,
/// and full DPS carry the hit's reason: so the value reaches no `Complete`
/// field (a test checks). PoB v2.67.2:
/// - `stormRainActiveArrows`: `activeArrowMultiplier` for Storm Rain of
///   the Conduit (`ConfigOptions.lua` 663 to 665), read by its
///   `preDamageFunc` as the `dpsMultiplier` (`act_dex.lua` 11851);
/// - `stormRainBeamOverlap`: `beamOverlapMultiplier` for Storm Rain (659 to
///   661; `act_dex.lua` 11747);
/// - `toxicRainPodOverlap`: `podOverlapMultiplier` for Toxic Rain and its
///   variants (700 to 702; `act_dex.lua` 14933, 15025, 15114).
/// - `detonateDeadCorpseLife` (the agent list, S8): `corpseLife` for every
///   skill (line 155, an untagged `SkillData`), which `CalcOffence.lua`
///   reads only for a skill whose data sets `explodeCorpse` (1856 to 1860:
///   the corpse's life times its explosion multiplier, as the hit's fire
///   `BonusMin` and `BonusMax`) and for the monster explosion's
///   `monsterExplode` (1861 to 1867). Those skills are listed (each with
///   `explodeCorpse` in its base mods, so unvetted in every part, or the
///   monster explosion's function); no DoT reads the value.
pub const SKILL_DATA_ONLY: &[(&str, &[&str])] = &[
    (
        "detonateDeadCorpseLife",
        &[
            "Bodyswap",
            "BodyswapAltX",
            "Cremation",
            "CremationAltX",
            "DeathWalk",
            "DeathWish",
            "DetonateDead",
            "DetonateDeadAltX",
            "DetonateDeadAltY",
            "EnemyExplode",
            "VaalDetonateDead",
            "VolatileDead",
            "VolatileDeadAltX",
            "VolatileDeadAltY",
        ],
    ),
    ("stormRainActiveArrows", &["StormRainAltX"]),
    ("stormRainBeamOverlap", &["StormRain"]),
    (
        "toxicRainPodOverlap",
        &["ToxicRain", "ToxicRainAltX", "ToxicRainAltY"],
    ),
];

/// Whether `input` only sets skill data its unvetted skills read
/// ([`SKILL_DATA_ONLY`]).
#[must_use]
pub fn skill_data_only(input: &str) -> bool {
    SKILL_DATA_ONLY
        .binary_search_by(|(name, _)| (*name).cmp(input))
        .is_ok()
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

    /// [`SKILL_DATA_ONLY`] (open items, S8): sorted, apart from the other
    /// lists, and each skill is in PoB's data with a function the hit does
    /// not read, or (corpse life, the agent list) `explodeCorpse` in its
    /// base mods, so its hit is vetted in no part (and Toxic Rain's DoT is
    /// not vetted).
    #[test]
    fn skill_data_only_skills_should_stay_unvetted() {
        assert!(SKILL_DATA_ONLY.windows(2).all(|w| w[0].0 < w[1].0));
        for (input, skills) in SKILL_DATA_ONLY {
            assert!(skill_data_only(input) && !named_only(input) && !inert(input));
            for id in *skills {
                let pob = super::super::pob_skill::skill(id).unwrap_or_else(|| panic!("{id}"));
                let explodes = pob.base_mods.iter().any(|m| m.key == Some("explodeCorpse"));
                assert!(!pob.functions.is_empty() || explodes, "{id}");
                for part in 0..=pob.parts.len() {
                    assert!(
                        !super::super::hit::pob_vetted_with(id, part, true),
                        "{id} {part}"
                    );
                }
                // Toxic Rain's pods are its DoT, which PoB multiplies by
                // the `dpsMultiplier` too (`CalcOffence.lua` 5747); Storm
                // Rain deals no DoT.
                if id.starts_with("ToxicRain") {
                    assert!(!super::super::dot::pob_dot_vetted(Some(pob)), "{id}");
                }
            }
        }
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
        // Fortified (`poe1::enemy_hit`, reading the check itself); P6-11bu:
        // life regeneration per stack, which `poe1::regen` reads with the
        // same stacks (`enemy_hit::fortification_stacks`); P6-11hq: Fortify
        // Support's more damage, which neither reads nor changes
        // fortification (`sup_str.lua` 2631 to 2636).
        for id in ids {
            assert!(
                !id.contains("fortif")
                    || id == "base_max_fortification"
                    || id == "life_regeneration_per_minute_%_per_fortification"
                    || id == "support_fortify_melee_damage_+%_final"
                    || id == "support_fortify_ailment_damage_+%_final_from_melee_hits",
                "{id}: revisit NAMED_ONLY"
            );
        }
        // `NoEffect` rows naming fortification, each checked: a chance to
        // fortify, its duration; none reaches a slice field.
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
        // Close-out M2: life lost per rage, a degeneration no slice field reads.
        let checked = [
            "rage_loss_delay_ms",
            "life_loss_%_per_minute_per_rage_while_not_losing_rage",
            // Close-out M3: rage lost faster, rage's loss.
            "rage_decay_speed_+%",
        ];
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
