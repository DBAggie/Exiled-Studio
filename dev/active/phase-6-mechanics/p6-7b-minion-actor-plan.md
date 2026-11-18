# P6-7b plan: the minion actor (life and defences)

Phase 6, step P6-7b (plan decision 7: minions are a second actor in the shared core). Reviewed by `plan-reviewer` (2026-11-17); every finding is applied below, under "Review". P6-7c (minion hits and DoT, spectres) follows.

## What PoB v2.67.2 does (the main skill's minion only)

1. **The minion** (`CalcActiveSkill.lua` 658 to 786).
   - **Which minion:** a skill with a minion list, PoB's hand-written `minionList` of short names (`RaisedZombie`). Only PoB's template `Export/Minions/Minions.txt` maps a name to a monster variety.
   - **Spectres** pick from the build's `<Spectre>` library (`isValueInArray(env.build.spectreList, skillMinion) or 1`, 662 to 688), which loom does not import: P6-7c.
   - **Level:**
     - PoB's flags `minionLevelIsEnemyLevel`, `minionLevelIsPlayerLevel` (`m_min(characterLevel, value)`), the support's level, or `skillData.minionLevel`;
     - else the gem level's level requirement;
     - clamped to 1 to 100.
     - The gem stats (`display_minion_monster_level`, `SkillStatMap.lua` 308 to 311) are read stat-ID first (ADR-015).
2. **Base values** (`CalcPerform.lua` `initMinionModDB`, 1119 to 1210; maximums from `calcs.initModDB`, `CalcSetup.lua` 18 to 27):
   - **Life:** `m_floor(lifeTable[level] x minionData.life)`, times `mapLevelLifeMult[enemyLevel]` for a hostile minion.
     - The table is `monsterLifeTable` if hostile.
     - Otherwise `monsterLifeTable2` or `3` for `AltLife1` or `AltLife2` (`MinionType::life_scaling`).
     - Otherwise `monsterLifeTable` for a spectre.
     - Otherwise `monsterAllyLifeTable`.
   - **Energy shield:** `m_floor(monsterAllyLifeTable[level] x life x energyShield)`, where `energyShield = 0.4 x Type.EnergyShield / 100`. It uses the ally table even for AltLife minions.
   - **Armour:** `round(monsterArmourTable[level] x (armour or 1))`. The table is PoB's, from `MonsterDamageReductionImprovement` (`miscdata.lua` 29 to 31), as `poe1::enemy::monster_armour` already computes it.
   - **Evasion:** `round(monsterEvasionTable[level] x (evasion or 1))`.
   - **The `or 1` quirk:** the exporter omits a zero armour or evasion, so PoB gives that minion the whole table. It is mirrored as a cited `pob-derivation` rule, with an open question for game evidence.
   - **Resistances:** the type's merciless ones; the maximums are the character constants.
   - **The variety's `modList`:** its mods, special mods, and `.ot` stats, through the skill stat map only. Any other stat is a comment with no effect (`minions.lua` 226 to 241). The engine maps them with `SkillMapping` or a dedicated minion mapping, never `Poe1Mapping`.
   - **PoB's template `#mod` extras:** 10 in `Minions.txt` (`UniqueAnimatedWeapon`'s `Life MORE 698`, `Condition:FullLife` on two minions) and 27 for spectres. These are base values.
3. **The player's `MinionModifier` mods** (1907 to 1925): from `env.player.mainSkill.skillModList` with its `skillCfg`, which holds the supports, the active gem, and the parent `modDB`.
   - Lines that name one minion type have a `value.type` filter.
   - Keystones are granted to the minion (1919 to 1923).
   - `StrengthAddedToMinions` (1207).
   - Necromantic Aegis (`aegisModList` 1167; block at `CalcDefence.lua` 681).
4. **Buffs and auras on the minion:**
   - **Buffs** (2282 to 2290): only with `applyMinions` or `applyAllies` (`skillData.buffMinions`, `buffAllies`), or `BuffAppliesToAllies`.
   - **Auras** (2426 to 2439): skipped under `SelfAurasCannotAffectAllies` and kin.
   - **`MinionModifier` lines inside buffs:** 3289 to 3290.
   - **The merge** (3293 to 3296): one copy per buff name.
   - **Scaling** reads the minion-side `BuffEffectOnSelf` and `AuraEffectOnSelf` (arriving through `MinionModifier`), plus the skill's `AuraEffect` and `BuffEffect`.
5. **Defences** (`calcs.defence(env, env.minion)`, 3732), as the player's. Minion life comes after resistances (`CalcDefence.lua` 647 to 657). `ResistanceAddedToMinions` (648 to 652) reads the player's final resistances.
6. **Output:** under `output.Minion`. The adapter writes all of `mainOutput`, so `/main_output/Minion/Life` exists. Breakdowns come only from the player's.

## Steps (each its own stacked PR with evidence)

### P6-7b1: data (done: `docs/phase-6/minions.md`, "P6-7b1")

- **The `.ot` stats.** The extractor reads every `metadata/monsters/**.ot` (`itemtype::is_object_template`).
  - **Bundle:** the stats resolved per object type path, deduplicated, in `poe1/minions.json` (not per variety).
  - **Resolution** follows PoB's reader quirks (`minions.lua` 41 to 73): it stops at `Metadata/Monsters/Monster` and `nothing`, reads superclass stats first, skips any line with `//`, reads only the `Stats` block, and parses values with `tonumber`.
  - **Blocked:** the files are in game bundles the local depot download of build 26530219 lacks (`Bundles2/Tiny_F1.bundle.bin` first), so the extraction needs a new depot download (maintainer).
- **PoB's minion-name table:** each `Minions.txt` name with its variety ID, `#mod` extras (cited), `#limit` (P6-7c), and `#skill` extras, generated from the export.
  - Also: each skill's `minions` list (export format 10) in `pob_skills.rs`.
  - Both are PoB-derived tables, like the skill parts (P6-6b).
- **Mod-list data-parity** (export format 13): a text parse of the generated `Data/Minions.lua` and `Spectres.lua` comments (`-- id [stat = value]`), compared with the variety's mods, special mods, and `.ot` stats. A stated rule covers data-version differences.

### P6-7b2: the actor in the core, and the parity tool (no value change)

- **Engine:** one `Calc` and one `Graph`, with a per-actor state (`ModStore`, global `Reasons`, settled states).
  - The minion's store is built by an ordinary second `Ingest` in a minion step, after the player's resistances.
  - Only the defence phases that are reused (`attributes`, `life_mana`, `resistances`, `block`, `primary_defences`) take an actor handle.
  - Minion nodes may read player field nodes (`ResistanceAddedToMinions`).
  - Each actor keeps its own global reasons and states. Minion fields carry the player's global reasons too (unmapped "Minions have ..." lines land there).
- **Fields:** a shared variant (decision 2), `Field::Minion(MinionField)`, with names:
  - `minion.life`, `minion.energy-shield`, `minion.armour`, `minion.evasion`;
  - `minion.resistance.<type>`, `minion.block-chance`.
  - Their readers are updated: UI labels and order, dashboards, golden calc documents, and tests that count fields.
- **Sources and reasons in calc result v1** (decision 3, no new closed variant):
  - base values: `Source::Rule("minion:<variety>")`;
  - unmapped monster stats: `UnmappedStat` from that rule;
  - an unknown minion: `MissingData { table: "minions", key }`;
  - item sets, Necromantic Aegis, hostile minions, and others' buffs: `UnmodelledSkill` or `UnmodelledItem`.
- **Parity tool:**
  - slice revision 4, with a nested PoB path (`pob_path: ["Minion", "Life"]`, the pointer built per segment) and distinct IDs (`Minion.Life`);
  - a `MINION_NAMES` table;
  - a group rule: every minion field is compared when `/main_output/Minion` exists or ours has a minion field, replacing `applies: present`, which would skip a missed 0;
  - minion fields compare by value only (no breakdown spec).
- **Gate:**
  - every public, corpus, and mechanic fixture's calc-run document is byte-identical before and after;
  - the instruction-count bench is unchanged (decision 12, ADR-013);
  - tests for the slice revision.

### P6-7b3a: base values and fields (after P6-7b1's data)

- **The minion step:**
  - the type, from PoB's minion list and name table;
  - the level, stat-ID first;
  - the base values above;
  - the variety's `modList` through the minion mapping;
  - `#mod` extras;
  - a mapping-parity check: loom's effects for each stat against PoB's `mod(...)` lines.
- **Reasons:**
  - every aura, buff, and `MinionModifier` PoB would apply to the minion is a reason on minion fields until P6-7b3b and P6-7b4;
  - spectre main skills stay `UnmodelledSkill` (P6-7c).
- **Cited `pob-derivation` rules with open questions:** the ES factor 0.4, armour and evasion `or 1`, and `#mod` extras.

### P6-7b3b: `MinionModifier` and the `NoEffect` audit

- **`MinionModifier`:** from the main skill's list with its config, the type filter, granted keystones, and `StrengthAddedToMinions`. Necromantic Aegis is a reason.
- **The audit:** every `NO_EFFECT`, `NO_EFFECT_PREFIXES`, and `BuffStat::NoSlice` row about minions or golems becomes a minion-store row or a reason on minion fields:
  - `minion_maximum_life_+%`, `minion_life_regeneration_rate_per_minute_%`;
  - `golem_maximum_life_+%`, `golem_immunity_to_elemental_damage`;
  - `revive_golems_if_killed_by_enemies_ms`;
  - Convocation's `minion-buff`, `GOLEM_CONDITION`.
- **Gate:** a test that no `NoEffect` row maps to a quantity a minion field reads.

### P6-7b4: buffs and auras on minions

- PoB's rules above, rescaled from the unscaled buff lists (the player's scaled buffs cannot be reused).
- Minion conditions (`FullLife` from `#mod`, `AffectedBy*`), and PoB's minion config inputs (`ifMinionCond`), classified.

## Evidence

- **Minion sweep:** one build per summoning gem's minion (PoB's lists), compared by value on the minion fields.
- **Gate (non-vacuous):** at least N `Complete` minion fields across the sweep, with zero differences from PoB (N set from b3a's first run).
- **Mechanic fixtures:**
  - Raise Zombie with "Minions have #% increased maximum Life";
  - an AltLife minion;
  - a minion with energy shield;
  - a type with zero armour or evasion (`or 1`);
  - `UniqueAnimatedWeapon` (`#mod` life);
  - a type-filtered `MinionModifier`;
  - `golem_maximum_life_+%`;
  - Minion Life Support;
  - `ResistanceAddedToMinions` (or that stat registered a reason);
  - a skeleton with an aura (b4).
- **Real-data limit:** real-data parity waits for the maintainer's `.ot` extraction; until then the evidence uses a local bundle.
- **Performance:** start time with the `.ot` stats, and the `calculate` bench with a minion build.

## Review (plan-reviewer, 2026-11-17), applied

- **C1, minion `NoEffect` rows:** P6-7b3b's audit and its gate.
- **C2, the parity tool:** slice revision 4, nested paths, the group rule; in P6-7b2.
- **C3, base-value formulas:** corrected above (floor; the ally table for ES; PoB's armour table; `or 1`; resistance maximums; life after resistances; cross-actor reads).
- **C4, `#mod` extras for minions:** P6-7b1's name table; base values.
- **C5, per-actor reasons and states:** P6-7b2.
- **Other changes:**
  - decision 2's shared field names;
  - decision 3's sources stated;
  - spectres to P6-7c (they need the build's spectre library, a build v1 addition);
  - three `pob-derivation` rules with questions;
  - corrected line numbers;
  - `MinionModifier` from the skill's list;
  - the skill stat map for monster mods;
  - the `.ot` reader's quirks, stored per object type;
  - the aura and buff rules;
  - reasons between b3 and b4;
  - b2's gate on every fixture and the bench;
  - a non-vacuous b3 gate;
  - more fixtures;
  - the minion level, stat-ID first;
  - the new fields' readers;
  - the `.ot` data limit.
- **The simpler design:** one `Calc` with per-actor state.
- **The split:** b3 into b3a and b3b.
