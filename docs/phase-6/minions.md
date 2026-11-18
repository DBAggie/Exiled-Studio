# P6-7a: minion data

Plan decision 7 (minions are a second actor) and decision 8 (data first): this step adds the monster data a minion or a spectre is computed from, with its `data-parity` table. No calculation reads it yet; P6-7b and P6-7c do.

## The data

- **Bundle:** `poe1/minions.json` (`loom_data::bundle::poe1::Minions`; converter `tools/parity-cli/src/minions.rs`). It holds every monster variety, since a spectre can be any monster: 10,691 rows from build 26530219. It also holds the monster types they use.
- **Extraction:** the tables are already extracted. `MonsterVarieties` and `MonsterTypes` come with existing roots, and `MonsterResistances` is referenced by `MonsterTypes`.
- **What is kept**, as PoB's minion exporter reads it (`src/Export/Scripts/minions.lua`, `#emit`):
  - **A variety's:**
    - name and tags;
    - life and damage multipliers;
    - attack duration in milliseconds (the schema's `AttackSpeed`);
    - maximum attack distance;
    - main- and off-hand item classes;
    - granted effects (its skills), mods, and special mods;
    - object type (`BaseMonsterTypeIndex`).
  - **Its type's:**
    - accuracy, armour, evasion, and energy shield;
    - damage spread;
    - the merciless resistances (`MonsterResistances`);
    - the alternate life scalings;
    - whether its base damage ignores attack speed;
    - the game's player-minion mark.
- **Not carried until P6-7b1:** the object type's `.ot` stats, which PoB adds to a minion's mod list (below, "P6-7b1").
- **Validator:** unique variety IDs, and each variety's type in the file. A synthetic row and a mutation test cover these checks.

## `data-parity`, table `minions` (export format 12)

The export adds `data.minions`, PoB's minions and spectres, keyed by the monster variety's ID. Spectres are keyed by it. A minion's PoB key is a short name, which PoB's own template (`Export/Minions/Minions.txt`, `#monster <id> <name>`) maps to its variety.

Each is compared in PoB's form:
- the name;
- life, damage, and accuracy over 100;
- energy shield `0.4 x value / 100`, and armour and evasion over 100, each absent at 0;
- the resistances;
- damage spread over 100;
- attack time over 1000;
- attack range;
- the life scaling;
- base damage ignoring attack speed;
- the weapon types (PoB's `itemClassMap`);
- the damage fixup from the mods;
- the tags;
- the skill list, as a prefix: PoB adds skills its template names.

PoB's mod list is not compared, because it includes the `.ot` stats.

**Result:** 332 minions and spectres, 5,959 values. The only difference is `GuardianRelicAll`, a minion PoB writes by hand in its template (no game variety), which is a `pob-derivation` rule. The PoB skill table, regenerated from format 12, is unchanged. Of the 968 differences, 353 are uncategorized, all `cluster_jewels`, as before (P6-3).

**Evidence bundle:** `local/p6/packs/b-minions` is the flask evidence bundle with only `poe1/minions.json` added, and `bundle-check` reports no findings. The bundle is local and unsigned. **Maintainer:** the next data run ships the file.

**Size and start:** the file is 11 MB as JSON, zipped when shipped. It is parsed at load. `loom-bench wall`, 7 runs on the flask evidence bundle without and with the file, gives:
- load: 631 ms against 668 ms;
- start: 743 ms against 783 ms (+40 ms).

**Versions:** an older application reading a newer bundle ignores the file (the loader reads only the files it knows). P6-7b1 changed the file's shape: "Bundle versions" below.

**The P6-7a review (code-architecture-reviewer), applied in its own PR (P6-7a2):**
- The converter fails on a variety without an ID or a type.
- The validator checks:
  - duplicate type IDs;
  - empty IDs, names, and types;
  - negative values (the game has varieties at 0);
  - each variety's skills against the granted effects.
- The rod's weapon key is PoB's verbatim.
- `MinionType::life_scaling` holds PoB's precedence (AltLife2 over AltLife1).
- `pob_data.lua` stops on a duplicate minion or a missing template.

Still open:
- the varieties PoB does not list are checked only structurally;
- PoB's mod list, and the `.ot` stats it adds, are neither carried nor compared. P6-7b must not compute a minion's life or resistances without them.

## The P6-6e review, applied here

`docs/phase-6/totems.md`, "P6-6e":
- the trap and mine rate ignores throwing speed, whose stats are unmapped;
- the module docs are corrected.

The review's other suggestion, integration tests for the trap and mine rate and reasons, is left to P6-7b's test pass. The sweep (33 builds against the oracle) is the evidence meanwhile.

## P6-7b2: the actor in the core, and the parity tool

The plan's step b2 (`dev/active/phase-6-mechanics/p6-7b-minion-actor-plan.md`) changes no value.

**Engine** (`loom_calc::engine`):
- `Actor` (`Player`, `Minion`), and `ActorState` (a modifier store, the reasons collecting it gave, its settled states).
- `Calc` keeps one graph and one field map, shared, with the player's state and an optional minion's. `store`, `context`, `global`, `settle_state`, `chain`, `set`, and `set_derived` work on the current actor.
- `with_actor` runs a step as another actor, and `add_minion` adds the minion's collected state.
- A minion's fields carry the player's global reasons too: an unmapped "Minions have ..." line lands there.
- **`Field::Minion(MinionField)`:** `minion.life`, `minion.energy-shield`, `minion.armour`, `minion.evasion`, `minion.resistance.<type>`, `minion.block-chance` (plan decision 2, a shared name). Nothing emits it yet (P6-7b3a).

**Parity tool:**
- Slice revision 4: a field may give `pob_path`, PoB's nested keys under `main_output`, with the pointer built per segment (`/main_output/Minion/Life`).
- `MINION_NAMES` lists the eight minion fields: life, energy shield, armour, evasion, and the four resistances. Block is left for b3a.
- `applies: minion`: each is compared when the oracle's `main_output` has a `Minion` table or our result has the field, so a minion's 0 is compared, not skipped. They are compared by value only; the oracle writes no minion breakdown.

**Evidence:**
- **Documents:** every public, mechanic, and corpus calc-run document is byte-identical to P6-6e's.
- **Public `calc-parity`:** 153 builds, 99 expected differences, 0 unexpected. Not-computed rows rose by 16: the minion fields of the public builds with a minion, which the engine does not compute yet.
- **Corpus:** no unexpected difference (not computed +8).
- **Tests:** the slice file's revision and fields, and the nested lookup and its pointer.
- **Not run locally:** the instruction-count bench runs in CI.

**The P6-7b2 review (code-architecture-reviewer), applied in P6-7b2b:**
- `Calc::with_actor` returns `None` for the minion when there is none, so a minion step never falls back to the character's store.
- The computed skill's sources are per actor (`ActorState::skill_sources`).
- `global()` is documented as the current actor's own reasons. The result's build-wide reasons are the character's; a minion's reach the result through its fields.
- Engine tests cover the refusal, the actor restored, states, skill sources, and reasons apart, and a minion field's reasons from both actors. Further tests cover the minion fields' names and order.

For b3a:
- A defence phase reused for the minion must write `Field::Minion(..)`, never the character's field.
- `MINION_NAMES` gains block when b3a emits it.
- A minion field the oracle lacks while ours has it needs a test.

Not changed: the commit trailer's model name, which follows the session's attribution instruction.

## P6-7b1: the `.ot` stats and PoB's minion templates

The data step of the minion actor (`dev/active/phase-6-mechanics/p6-7b-minion-actor-plan.md`). It needed the maintainer's depot download of build 26530219's `Bundles2` (2026-11-17). No calculation reads the new data yet: P6-7b3a does.

### Extraction (manifest format 6)

- **What is read:** every `.ot` file under `Metadata/Monsters/` (`tools/extractor/src/monstertemplate.rs`), into `monster_templates/**/*.json`: 7,176 files in build 26530219, all read, 6,212 stat lines.
- **Why a separate reader:** these files hold script blocks (`on_death = { RemoveEffects(); }`), a nested `client` section, and several `extends` lines. The item type grammar rejects 1,910 of them.
- **How it reads:** each file line by line, as PoB's reader does (`src/Export/Scripts/minions.lua` `getOTStats`, 41 to 73). It keeps, in file order:
  - every `extends "..."` target, as written (Lua's greedy capture, anywhere in the line);
  - each `key = value` line of the top-level `Stats` block. The block opens at a line starting with `Stats` and closes at one starting with `}`. A line with `//` is skipped, and all whitespace (Lua's `%s`) is removed before splitting at the first `=`.
- **What fails a file:** only text that is not UTF-16LE, or a bad path. An `extends` target outside the folder is kept as written; the bundle checks the chains it needs.
- **`loom-data`** reads extract manifest formats 3 to 6.

### Bundle: `object_stats` in `poe1/minions.json`

`Minions::object_stats` is an optional list of `ObjectStats`, sorted by object type. Each entry is an object type a variety names, resolved as PoB does (`tools/parity-cli/src/minions.rs`):
- **Chain order:** the type's file, then each superclass where its `extends` line stands. Repeats are kept: PoB adds each line, and a diamond reads its shared file twice.
- **Stops:** at `Metadata/Monsters/Monster` and `nothing`, compared exactly as PoB does. File lookup ignores case, as PoB's `getFile` lowercases. A variety whose object type is `Metadata/Monsters/Monster` reads no `.ot` stats (`minions.lua` 220).
- **Values** are read as whole numbers, since the bundle's pairs are integers. A value that is not one goes in `values` as text, not dropped (ADR-015). Build 26530219 has two such types, both script text: a commented-out line and mercenary dialogue. PoB's `tonumber` gives nil for script text, and its exporter would fail on such a minion. A decimal or hexadecimal number, which `tonumber` would read, would also go in `values`: it cannot be one of the integer pairs. The build has none.
- **`unread`** names the first file a chain reaches that the extract does not hold: a template outside `Metadata/Monsters/`, such as `Metadata/Utility/InanimateObject` or NPC and trap bases. 12 targets reach no file: blight towers, traps, NPCs. No minion or spectre PoB has is among them.
- **A loop fails the conversion**, since PoB's reader would not end. Build 26530219 has none.
- **A type with nothing to carry is left out.** `None` (the field absent) means the bundle was built before P6-7b1: the `.ot` stats are unknown, not empty.

**Counts:** 5,049 object types named, 3,265 with an entry, 22,762 stat lines.

**Validator:** a repeated object type gives `BUNDLE_DUPLICATE_ID`. A list out of order gives `BUNDLE_VALUE_RANGE`, since `Minions::object_stats` uses binary search. A synthetic entry and two mutation tests cover these checks.

### PoB's minion templates (export format 13)

- **The table:** `loom_calc::poe1::pob_minion`, generated as `pob_minions.rs` by `loom-parity-cli pob-minions` from the export's `minion_templates`. It holds the 331 minions PoB's `Export/Minions/Minions.txt` and `Spectres.txt` emit, by PoB's name. Each entry has:
  - the variety;
  - `#limit` and `#hostile`;
  - the extra skills (`#monster` arguments, then `#skill`);
  - each `#mod` as written, cited by its line.
- **`#mod` extras:** 10 for minions and 27 for spectres. They are PoB's, not the game's, so the step that applies one types it with evidence (P6-7b3a for base values).
- **Reading the template:** the export reads it as `Export/Main.lua` does, matching the first `#word` of a line anywhere in it. An unknown directive stops the run.
- **The skill table:** `pob_skills.rs` gains each skill's `minions` and `add_minions` lists, in PoB's order (the first is the default). A test checks that every name a skill lists is in the minion table. Spectres are variety IDs (P6-7c). The exception is `GuardianRelicAll`, which PoB writes by hand (`Minions.txt` 183).
- **Determinism:** both tables regenerate byte-identical.

**Bundle versions:** `minions.json` changed shape additively. A bundle without `object_stats` still loads, with the stats unknown. `Minions` refuses unknown fields, though, so an application built after P6-7a and before P6-7b1 would refuse the new file. No released application reads `minions.json` (0.9.0 predates P6-7a), so no bundle schema change is needed. Should one be wanted later, it goes with `Minions`' `deny_unknown_fields` (#164 review).

### `data-parity`, table `minion_mods` (export format 13)

- **PoB's side:** the export reads the comments PoB's exporter wrote beside each `modList` line of `Data/Minions.lua` and `Data/Spectres.lua` (`-- <mod> [<stat> = <value>]`, one per stat, `minions.lua` 226 to 241). `data.minions` keeps only the parsed mods, so the IDs are only in these comments.
- **Our side:** for each minion and spectre, by variety ID, the bundle's list in PoB's order:
  - the variety's mods, then its special mods, one line per stat with the lowest value;
  - then its object type's `.ot` stats, the stat as the ID.
- **Result:** 332 lists, 700 lines. Every `.ot` stat matches, in PoB's order. The 17 differences have two rules:
  - **`minion-mods-six-stat-slots`** (`pob-derivation`, 16): PoB's exporter reads a mod's first six stats only (`for i = 1, 6`, `minions.lua` 228). `CannotBeAugmented`'s seventh is `monster_no_talismans = 1`, which PoB leaves out.
  - **`minion-mods-pob-written`** (`pob-derivation`, 1): `GuardianRelicAll`, as in table `minions`.
- **Not compared:** the template directives themselves (`NOT_COMPARED`): they are PoB's own data, which `pob-minions` copies.

The run on the evidence bundle has 985 differences in all, 370 of them uncategorized. All 370 are `cluster_jewels`, waiting on P6-3's data, as in P6-7a. `minion_mods` adds 17, all categorized.

### Evidence

- **Evidence bundle:** `local/p6/packs/b-ot` is the minion evidence bundle (`b-minions`) with only `poe1/minions.json` replaced. The file comes from the new extract. Its types and varieties are unchanged; only `object_stats` is added. `bundle-check` reports no findings, and the bundle is local and unsigned. **Maintainer:** the next data run ships it.
- **Size and start:** the file grows from 11.2 MB to 13.3 MB as JSON. `loom-bench wall`, 7 runs on each, gives:
  - load: 671.5 ms against 674.8 ms;
  - start: 796.9 ms against 806.7 ms (+10 ms).
- **Calculation:** no value or state changed, with the flask evidence bundle as every Phase 6 step (`local/p6/p6-7b1` against `p6-11e`) and with `b-ot`.
  - Public `calc-parity`: 157 builds, 109 expected differences, 0 unexpected.
  - Corpus: 0 unexpected.
  - Both runs are byte-identical.

## The P6-7b1 review (P6-7b1b)

The code-architecture-reviewer found no blockers in #163. Applied:
- **The monster base:** `Metadata/Monsters/Monster.ot` is an object template only. `is_monster_template` excludes it rather than leaving the choice to the order of checks in `Extraction::add`. A test pins this. The extract's output does not change, because the object templates already took the file.
- **`MainSkill::can_use`:**
  - It has its own test: no row, an empty list, a listed class, `Unarmed`, and an unlisted class.
  - Its docs were wrong about supports and now follow PoB (`CalcActiveSkill.lua` 287 to 291): each applied support's `weaponTypes` joins the list, and the weapon must be in every one. That is not modelled, and neither is `countsAsAll1H` (200) or a minion skill ignoring the weapon types (296 to 299).
- **Validator:** an `object_stats` entry that no variety names gives `BUNDLE_DANGLING_REF`, and an entry with nothing to carry gives `BUNDLE_VALUE_RANGE`. Each has a mutation test.
- **`data-parity`:** a bundle without `object_stats` gives one difference (`minion_mods`, `*`, `object_stats`) instead of one per minion. A test covers it.
- **Generator:** `pob-minions` sorts by name itself, and `run_skills` and `run_minions` replace a boolean switch.
- **Export:** an `#emit` before any `#monster` stops the export with its line. The export's bytes are unchanged.
- **Docs:** the bundle versions note and the decimal-value wording above.

**Evidence:** both tables and the PoB export regenerate byte-identical, and `b-ot` has no findings. No calculation value or state changed (`local/p6/p6-7b1b` against `p6-7b1`).

## P6-7b3a: the minion's base values and fields

The engine computes the main skill's minion (`loom_calc::poe1::minion_actor`) as PoB v2.67.2 computes `env.minion`, with the minion fields of slice revision 4: life, energy shield, armour, evasion, the four resistances, and attack block. Block is computed but not compared, since the slice has no minion block field.

### Which minion

- **The list:** the skill's `minionList` and the minions its supports add (`CalcActiveSkill.lua` 658 to 700).
- **The choice:** the gem's `skillMinion` when it is in the list, else the first. A gem keeps its untyped attributes, so no build schema change was needed.
- **The variety:** PoB's name maps to it through `pob_minions.rs` (P6-7b1).
- **Not computed:** these minions' fields are `Unsupported`, each with an `unmodelled-skill` reason:
  - `minion-list`: a skill that makes minions without a list (Raise Spectre picks from the build's spectre library, P6-7c);
  - `minion-data`: a minion PoB writes by hand, or one the data lacks;
  - `minion-items`: a minion with items (Animate Weapon, Animate Guardian, Blink Arrow, Mirror Arrow);
  - `hostile-minion`;
  - `minion-level`: a level PoB takes from the enemy, the character, or a level requirement.

### Level

PoB uses `skillData.minionLevel`, else the gem level's level requirement, clamped to 1 to 100. The bundle does not carry level requirements, so the game's `base_display_minion_actor_level` stat is read instead (stat-ID first).

It equals PoB's level requirement at 1,822 of PoB's 1,861 minion gem levels. The 39 others are all Vaal Summon Skeletons', where the game's stat is lower: 88 at gem level 20, against PoB's 90. The sweep shows the effect on life, armour, and evasion. Those fields are `Partial` (the gem's minion stat), so no register entry is needed yet. One is needed once they are `Complete` (P6-7b3b).

Skills without the stat (Dominating Blow, Herald of Agony, Holy Strike, Summon Skitterbots, item-granted Raise Spiders) give `minion-level`.

### Base values and mod list

- **Base values** follow `initMinionModDB`:
  - life is `floor(table x life)` on the ally table, or the AltLife tables;
  - energy shield is `floor(ally table x life x 0.4 x ES / 100)`;
  - armour and evasion are `round(table x (multiplier or 1))`, PoB's `or 1`, a `pob-derivation`;
  - the merciless resistances;
  - the character constants' maximums.

  The multipliers are read as PoB's exporter wrote them (Lua's `tostring`, 14 significant digits).
- **The mod list:** the variety's mods (first-to-sixth and later stats, lowest value), its special mods, and its `.ot` stats, through `MinionMapping`. That mapping holds the 29 stats PoB's stat map turns into mods on the skills' minions.
  - Five reach a minion field: `base_maximum_energy_shield`, `physical_damage_reduction_rating_+%`, `set_maximum_life_is_one` (a life override), `monster_base_block_%`, and `block_while_dual_wielding_%` (a minion never dual wields).
  - The other 24 are P6-7c's (damage, speed, immunities).
  - Every other stat is a comment in PoB, with no effect.
- **Template extras:** `TEMPLATE_MODS` reads the seven `#mod` extras of `Minions.txt`: `Life MORE 698` applies, `Condition:FullLife` is a minion state, and the rest are P6-7c's. A test checks that every one is read.
- **The life field:** PoB's `override or max(round(base x inc x more), 1)`.

### Reasons until P6-7b3b and P6-7b4

Every minion field carries:
- **the character's reasons:** an unmapped "Minions have ..." line, and the main gem's own `unmodelled-gem`;
- **`minion-modifier`**, from three sources:
  - each stat of the main skill's gems (active and supports) that PoB's stat map sends to the minion (`PobSkill::minion_stats`, generated: 77 skills);
  - the four mapping rows that were `NoEffect` (`minion_maximum_life_+%`, `minion_life_regeneration_rate_per_minute_%`, `golem_maximum_life_+%`, `golem_immunity_to_elemental_damage`). They now map to the flag `Poe1Quantity::MinionPending`, which no character field reads. A flag's value is never read, so an unsettled range on a flag-only stat now sets it instead of giving `unsettled-value`. Only Forged Frostbearer's custom lines met that case, and its states are back to P6-7b1's.
- **`minion-buffs`:** each other enabled group whose active skill is an aura, or a buff PoB applies to minions (`buffMinions`, `buffAllies`).

### Evidence

- **The sweep:** one Witch level 90 build per summoning gem (gem level 20), plus Summon Skeletons with `skillMinion` set to the caster: 52 builds in all (`local/p6/minion-fx`, PoB's outputs `local/p6/mfx-oracle`).
  - 0 unexpected differences.
  - 32 minion fields are `Complete`, all equal to PoB.
  - Every `Partial` field equals PoB's value except seven, each explained by its reason: Herald of Purity, Raise Zombie, Raging Spirit of Enormity, and Stone Golem of Safeguarding through the gem's minion life stat, and Vaal Summon Skeletons through the level.
  - The plan's gate (N `Complete` minion fields with zero differences) is met with N = 32.
- **Fixtures:** four of the sweep are mechanic fixtures, those with fields to target: Flame Golem, Stone Golem, Stone Golem of Safeguarding, and Herald of Purity. The other builds have no `Complete` field yet (the main gem's reason reaches every field).
- **The corpus build with a minion** matches PoB on energy shield, evasion, and chaos resistance. Its life, armour, and elemental resistances differ through the character's minion modifiers, which are reasons.
- **Calculation:**
  - On the flask bundle (no minion data), against P6-7b1: no value changed. The three public and one corpus minion builds gain minion fields, `Unsupported` with `minion-data`.
  - On `b-ot`: public `calc-parity` has 161 builds, 109 expected differences, and 0 unexpected; the corpus has 0 unexpected.
  - Both runs are byte-identical.
- **Evidence bundle:** from P6-7b3a on, the evidence bundle is `local/p6/packs/b-ot`. The minion fixtures' targets need its `.ot` stats.

## The P6-7b3a review (P6-7b3a2)

The code-architecture-reviewer found no blockers in #165. Applied:
- **Item skills:** the list now holds all 12 skills PoB gives `minionHasItemSet` or `minionUses`. That adds Holy Strike, Animate Guardian's Weapon, Manifest Dancing Dervishes, and the four Enchantments of Reflection. Animate Weapon of Ranged Arms still comes through `minionUseMainHandWeapon`. The list is sorted and tested.
- **`minion-buffs`:** this reason now also covers the main group's other active gems, such as an aura linked beside the summon gem. Before the fix, such a minion's fields could have been `Complete` and wrong.
- **`extraSkillModList`:** PoB adds it to the minion (`initMinionModDB` 1149 to 1151). Its "Socketed Gems have ..." lines are mapped by no engine row, so each is already an `unmapped-stat` reason of the character, which every minion field carries. This is now documented in `deferred_reasons`.
- **The base arithmetic:** it is now a pure function, `base_numbers`, with a test of:
  - floor and round;
  - `or 1`;
  - the AltLife tables, where the second wins;
  - energy shield on the ally table, and only for a type with some.
- **`Ingest`:** a test covers the flag-only change. An unsettled range on a mapping that is empty, or that mixes a flag with another operation, is still a reason.
- **`CalcData::mod_stats`:** it reads a mod's first six stats, as PoB's exporter does.
- **Startup cost:** a build with no minion skill gem now returns before the minion work.
- **Mapping comments:** the misplaced comments in `ROWS` are restored, and the golem rows note that they are reasons for every minion type until P6-7b3b.
- **Left for later, with a reason:**
  - Vaal Summon Skeletons' level divergence gets its register entry with P6-7b3b, once its fields can be `Complete` (task added).
  - The life override has no unit test, because `minion_life` needs a whole calculation. No fixture minion has `set_maximum_life_is_one` with `Complete` fields yet; P6-7b3b's fixtures add one.
  - `MINION_STATS`' completeness is checked by the data-parity table `minion_mods`, not by a unit test: the stats live in the bundle.

**Evidence:**
- **The sweep:** only Holy Strike changed, its reason now `minion-items` instead of `minion-level`. There are still 0 unexpected differences.
- **Public and corpus on `b-ot`:** no value or state changed. There are 0 unexpected differences, and both runs are byte-identical.

## P6-7b3b: the character's minion modifiers on the minion

PoB adds every `MinionModifier` of the main skill's `skillModList` to the minion (`CalcPerform.lua` `addMinionModifiers`, 1213 to 1219, called at 1918), keeping only those whose `type` names the minion. "Golems have ..." lines carry the main skill's `SkillType.Golem` tag (`ModParser.lua` 1137).

### The rows

`poe1::minions::STATS` has 31 rows, sorted. Each carries PoB's mod type as its op. Each was checked against PoB's own reading:
- **item and passive lines:** PoB's `modLib.parseMod` on the game's text, rendered from the bundle's stat descriptions. The tool is `tools/oracle/data/pob_parse_lines.lua`, with the inner `MinionModifier` mod written out (local copy, `local/p6/parse2`).
- **gem stats:** PoB's stat map (`Data/SkillStatMap.lua` 1922 to 2061; `Data/Skills/sup_int.lua` 4675 and 4929).

The new kinds of `MinionValue` are the minion's life, energy shield, armour, evasion, each resistance, each maximum resistance, block and spell block, life regeneration, movement speed, and cooldown recovery. `MinionValue::on_minion` names the minion's quantity for each. Regeneration, movement speed, and cooldown recovery reach no minion field, just as the character's movement speed reaches no field. Damage and speeds are P6-7c's.

| Rows | PoB's reading |
|---|---|
| `minion_maximum_life_+%`, `minion_maximum_energy_shield_+%`, `minion_evasion_rating_+%` | `Life`, `EnergyShield`, `Evasion` INC |
| `active_skill_minion_life_+%_final`, `support_minion_maximum_life_+%_final`, `support_minion_damage_minion_life_+%_final`, `necromancer_defensive_notable_minion_maximum_life_+%_final`, `active_skill_minion_energy_shield_+%_final` | `Life` or `EnergyShield` MORE |
| `minion_physical_damage_reduction_rating` | `Armour` BASE |
| `minion_elemental_resistance_%`, `minion_fire_damage_resistance_%`, `minion_cold_damage_resistance_%`, `minion_chaos_resistance_%` | `<Type>Resist` BASE (all three for the elemental row) |
| `minion_maximum_all_elemental_resistances_%` | `ElementalResistMax` BASE |
| `minion_block_%`, `minion_additional_spell_block_%` | `BlockChance`, `SpellBlockChance` BASE |
| `minion_life_regeneration_rate_per_minute_%` | `LifeRegenPercent` BASE, per minute over 60 (no field) |
| `minion_movement_speed_+%`, `active_skill_minion_movement_velocity_+%_final`, `minion_cooldown_recovery_+%` | `MovementSpeed` INC and MORE, `CooldownRecovery` INC (no field) |
| `golem_maximum_life_+%`, `golem_maximum_energy_shield_+%`, `golem_physical_damage_reduction_rating` | the same, tagged `SkillType.Golem`: `Poe1Quantity::GolemMinion` |

**Moved out of `MinionPending`:**
- `Poe1Quantity::MinionPending` is gone. Its four stats are rows now, except `golem_immunity_to_elemental_damage`.
- `golem_immunity_to_elemental_damage` is back in `NO_EFFECT` (`minion-defence`): PoB's parser does not read its line ("Golems are Immune to Elemental Damage"), so it reaches none of PoB's outputs.

### The minion step

`poe1::minion_actor` now adds three things to the minion:
- **`minion_modifiers`:** the character's effects of those kinds. A golem kind applies only when the main skill has the `Golem` type. A conditional one is a `minion-modifier` reason.
- **`gem_modifiers`:** the main skill's own gems' stats, read through the gem mapping.
  - The character's store holds them only when the skill is also a hit or damage over time the engine computes. So they are read here, and their copies in the character's store are skipped.
  - A stat PoB's stat map sends to the minion (`PobSkill::minion_stats`) that the gem mapping leaves unmapped is an `unmapped-stat` reason on the minion. One it reads as no effect is a `minion-modifier` reason.
- **Unsettled values:** an unsettled range on a minion-only stat (`StatMapping::elsewhere`) is kept in `ModStore::elsewhere`. It becomes the minion's `unsettled-value` reason, not the character's.
  - Before this, such a range made every character field indeterminate (Forged Frostbearer's "Minions Regenerate (1-2)% of Life per second").
  - Minion damage and speeds stay the character's, since a redirect can give them to the character.

**PoB's more multiplier rounding:** `ModDB:MoreInternal` rounds each name's product of `1 + more / 100` to two decimals. The engine's `phases::chain` (life, mana, energy shield, armour, evasion) did not. It now does, adding the nodes only when the rounding changes the value. Raise Zombie with Minion Life shows it: 1.38 x 1.49 = 2.0562, which PoB rounds to 2.06. No character value of the public or corpus builds changed. `hit.rs` already rounded the hit's multipliers so.

**Not modelled, each a reason:** keystones given to the minion, `StrengthAddedToMinions`, and `ResistanceAddedToMinions` come from stats the engine does not map, so each is already the character's `unmapped-stat`. Reaper's "non-Reaper minions" life (a `GlobalEffect` buff) waits for P6-7b4.

### Evidence

- **The sweep (52 builds):**
  - 0 unexpected differences.
  - 40 minion fields are `Complete`, all equal to PoB. That is 32 before, minus those now carrying a gem's unmapped minion damage stat, plus Herald of Purity's and Stone Golem of Safeguarding's.
  - Every `Partial` field equals PoB except Vaal Summon Skeletons' life, armour, and evasion (the level). Its fields stay `Partial` through the gem's `unmodelled-gem`, so its register entry waits (task kept).
- **Fixtures:**
  - New: `minion-summon-stone-golem-golem-lines-witch.xml`, with golem life and armour lines and two more-life lines. All 25 fields are `Complete` and equal to PoB, the rounding included.
  - Stone Golem of Safeguarding's targets gain its minion fields.
  - A Raise Zombie build with the minion lines and Minion Life equals PoB on every minion field. It stays local (`local/p6/fx16`): the Raise Zombie gem is the character's `unmodelled-gem` until P6-9.
- **`set_maximum_life_is_one`:** every skill minion with it (skitterbots, Herald of Agony's spider, ancestral totems' minions) takes its level from the gem's level requirement or the character. The bundle carries neither, so those minions are `minion-level`, and the fixture waits on a data run with level requirements.
- **Public `calc-parity` on `b-ot`:** 162 builds, 109 expected differences, 0 unexpected. `Complete` fields went from 2,822 to 2,857 (2,840 without the new fixture), and the indeterminate count is unchanged.
- **Corpus:** 0 unexpected, field counts unchanged.
- Both runs are byte-identical.

## P6-7b4: the character's auras and buffs on the minion

PoB applies two kinds of the character's skills to the main skill's minion, merging them per name into `minionBuffs`:
- **a buff with `skillData.buffMinions` or `buffAllies`** (`CalcPerform.lua` 2282 to 2290);
- **every self aura** (2425 to 2438), unless the character's "auras cannot affect allies" flags say otherwise.

The minion's scale is `(1 + inc / 100) x more`, from the skill's `AuraEffect` and `BuffEffect`. It leaves out the "on self" terms the character's scale has, and adds the minion's own `BuffEffectOnSelf` and `AuraEffectOnSelf`.

**The engine:** `poe1::gems::minion_buffs`:
- It walks the enabled skills as `gems` does, and takes each skill whose buff reaches the minion (`reaches_minion`).
- For a modelled one (`Kind::Buff`), it adds the rows with `add_rows`. They are scaled by the character's `Totals::buff_scale` (now also built from a finished store, `Totals::from_effects`) and merged per name with each row's largest value, as `mergeBuff`. The "on self" terms PoB leaves out are terms the engine reads nowhere (they are unmapped, reasons), so the two scales are the same.
- An unmodelled one, or one with an unmodelled support, is a `minion-buffs` reason.
- Lightning Golem's Wrath, a minion skill's aura that reaches minions too (2775 to 2780), is a reason when the config enables it.
- The flags that stop auras reaching allies, and the minion's buff effect on itself, come from stats the engine does not map: the character's reasons, which every minion field carries.

`deferred_reasons` is gone: every source it named is now applied or a reason where it is read.

### Evidence

- **Fixture:** `minion-summon-stone-golem-discipline-witch.xml`: Stone Golem with Discipline in another group, and 15% increased effect of non-curse auras. The golem's energy shield is 249, as in PoB, and all 25 fields are `Complete` and equal to PoB.
- **The sweep:** unchanged (no aura in its builds).
- **Public `calc-parity` on `b-ot`:** 163 builds, 109 expected differences, 0 unexpected. No document but the new fixture's changed.
- **Corpus:** unchanged, 0 unexpected.
- Both runs are byte-identical.

## P6-7c, part 1: the minion's hit

PoB computes the minion's main skill with its own offence code: `calcs.offence(env, env.minion, env.minion.mainSkill)` (`CalcPerform.lua` 3737). The engine does the same. Under the minion actor, `Calc` writes and reads each field as the minion's copy (`Field::for_minion`, `Calc::with_actor`), so the character's attack path (`hit::attack_hit`) computes the minion's hit unchanged (`poe1::minion_hit`).

**What PoB does:**
- **The minion's skills** (`CalcActiveSkill.lua` 870 to 922): its `skillList` (the variety's skills, then the template's), each PoB's data has, at level 1, with the summoning skill's supports. The main one is the gem's `skillMinionSkill` (1-based, clamped), else the first; PoB's default attack when the list is empty.
- **Its weapon** (719 to 752):
  - `damage = monsterAllyDamageTable[level] x damage`, times `attackTime` unless the type's damage ignores attack speed;
  - `PhysicalMin = round(damage x (1 - damageSpread))` and `PhysicalMax = round(damage x (1 + damageSpread))`;
  - `AttackRate = 1 / attackTime`, `CritChance = 5`;
  - its type is the variety's main-hand class through the exporter's class map (`minions.lua` 76 to 93), else `None` (unarmed); the off hand holds nothing.
- **`damageFixup`** (488 to 490; the exporter, `minions.lua` 185 to 191): `Damage` MORE `-100 x fixup` and `Speed` MORE `100 x fixup` for attacks, from a `MonsterSpeedAndDamageFixup*` mod.
- **`initMinionModDB`** (`CalcPerform.lua` 1141 to 1146): `CannotBeEvaded`, and `CritMultiplier` BASE of the monster template's `base_critical_strike_multiplier - 100`.
- **The enemy is shared:** one `enemyDB`, so the character's curses, debuffs, exposure, and the configured enemy conditions reach the minion's hits.

**The engine:**
- `minion_actor::collect`:
  - the minion's main skill (`ActorState::skill`) and its weapon class (`ActorState::weapon`);
  - its monster weapon as the weapon quantities an item gives the character, from the variety, the type, and the new `CalcData::monster_ally_damage`. The table's single-precision number is widened as PoB's table holds it; the multipliers are the exporter's (`lua_number`). The rate is the new `WeaponValue::AttackRate`, PoB's `1 / attackTime` as is: the item path's millisecond rate differs from it by one ulp;
  - `damageFixup` and `CannotBeEvaded`;
  - the enemy's effects (every effect on an enemy quantity, and every effect of a curse or debuff skill) and the configured `Enemy:` conditions, copied from the character's.
- `hit::crit_multiplier` takes the monster template's base for the minion.
- `MinionValue::on_minion` gives damage and attack and cast speed with their flags, so the character's "Minions deal #% increased Damage" and kin, and the main skill's gems' minion stats, reach the minion's hit.
- The minion's own monster mods that reach its hit now have PoB's mods (`MINION_STATS`):
  - `active_skill_attack_speed_+%_final`;
  - added physical damage to attacks, and to the main hand's weapon;
  - the three physical conversions;
  - `dual_wield_inherent_attack_speed_+%_final` (the minion never dual wields).

  Until this step they had no effect, which was right while the minion had only defence fields.
- The minion's strength, dexterity, and intelligence are computed from its own store (none but the character's minion modifiers give it any), as its hit reads them.

**Computed:** a minion skill of `minion_hit::VETTED` (PoB's default attack `Melee`, the Raging Spirit's `SumonRagingSpiritMelee`: plain attacks in PoB's data, a test), when the summoning skill has no support. Otherwise the minion's offence fields are `unmodelled-skill`:
- `minion-skill`: any other minion skill (the golems' and relics' spells, the Reaper's);
- `minion-skill-support`: PoB applies the summoning skill's supports to the minion's skills, which the engine does not yet;
- `minion-weapon`: a weapon class PoB has no flags for.

**Slice revision 5:** `Minion.AverageDamage`, `Minion.Speed`, `Minion.HitChance`, `Minion.CritChance`, `Minion.CritMultiplier`, `Minion.TotalDPS` (PoB's `output.Minion`). The result gains the minion's copies of the offence fields and its attributes (`MinionField`, additive).

### Evidence

- **The sweep (52 builds):** 10 builds' minion hits computed, 58 offence values equal to PoB, 2 registered. Vaal Summon Skeletons' damage follows its minion level, 68 against PoB's 70: DIV-0123 and DIV-0124, `game-truth`, as DIV-0111 to DIV-0113. Minion fields equal to PoB: 175 (109 before). 0 unexpected.
  - The Raging Spirit first showed `Complete` but differed. Its monster mods (conversion to fire, added physical damage, attack speed MORE 40) had no effect, and nothing marked them a reason. Fixed with their PoB mods above. The other `Later` rows reach no minion field: damage taken, immunities, stun, leech, regeneration, and charges.
- **Fixture:** `minion-raise-zombie-damage-speed-lines-witch.xml`: the character's minion damage and speed lines, with all 14 minion fields equal to PoB. Nine minion fixtures are retargeted to their hit fields.
- **Public `calc-parity` on `b-ot`:** 183 builds, 123 expected differences, 0 unexpected. `Complete` fields: 3,447. No value of another build moved.
- **Corpus:** 0 unexpected, no value moved.
- Both runs are byte-identical.

## P6-7c, part 2: the summoning skill's supports on the minion's skills

PoB creates the minion's skills with the summoning skill's support list (`CalcActiveSkill.lua` 903). For a minion's skill, `canGrantedEffectSupportActiveSkill` checks a support against the summoning skill's types and its minion types (`CalcTools.lua` 104 to 105), the same check the summoning skill's own supports passed. So each of them reaches the minion's skill, except:
- **triggers:** `isTrigger`, since the minion is not the player (113);
- **supports that name weapon types:** the minion skills have none (119 to 130).

A support's stats then join the minion skill's list. Two kinds are not read there:
- **its minion modifiers:** they apply to the minion's own minions, which don't exist;
- **its `notMinionStat` stats:** `Modules/Data.lua` 81 to 90 tags them so a minion's skill skips them.

**The engine:**
- `minion_actor::support_modifiers` reads each such support's stats into the minion's store through the gem mapping (`SupportOnMinion`). It drops the minion modifiers, which `gem_modifiers` already gives the minion, and the `notMinionStat` stats.
- The tables come from `Data/Skills/sup_*.lua`: `NO_MINION_SUPPORTS` (27 triggers and weapon-typed supports) and `NOT_MINION_STATS` (36 supports).
- An unmapped stat is the minion's `unmapped-stat` reason, as for the character's skill.
- The `minion-skill-support` gate is gone.

**Rows added on the way** (the character's own skills read them too):
- Brutality's own `statMap` (`sup_str.lua` 802): `support_brutality_physical_damage_+%_final`, `PhysicalDamage` MORE;
- `SkillStatMap.lua` 1232 to 1239: `deal_no_elemental_damage` and `base_deal_no_chaos_damage`, PoB's `DealNo<Type>` flags. The gem mapping now takes `DealNo` as a hit quantity;
- Minion Damage's own `statMap` (`sup_int.lua` 4813): `support_minion_damage_+%_final`, a minion modifier holding `Damage` MORE (the redirect to the character reads only increases).

### Evidence

- **Fixture:** `minion-raise-zombie-supports-witch.xml`, Raise Zombie with Minion Damage, Minion Speed, Melee Physical Damage, and Brutality. All 31 compared fields `Complete` and equal to PoB, the zombie's 468.04 average damage included.
- **The sweep:** unchanged (no support in its builds), 0 unexpected.
- **Public `calc-parity` on `b-ot`:** 184 builds, 123 expected differences, 0 unexpected; `Complete` fields 3,491.
  - 10 values of other builds moved, 8 toward PoB: Brutality's more physical damage in Boneshatter, Cyclone, and Lacerate builds.
  - 2 moved away: Kinetic Blast of Clustering's main group has Void Manipulation, whose "deal no elemental damage" now applies, as in PoB. The build's fields stay `Partial` with other reasons, and PoB's value is far above ours for them.
- **Corpus:** 0 unexpected, no value moved.
- Both runs are byte-identical.

**Next (P6-7c, part 3):** more minion skills (spells, slams: each PoB skill's data, as the character's), the minion's ailments and damage over time (the copies exist: `MinionField::IgniteDps` and kin), spectres.

## The review of P6-9e to P6-11h (code-architecture-reviewer, no blockers)

Applied:
- **Every character field has a minion copy** (`Field::for_minion` is total: `MinionField` gains mana, the unreserved pools, and the game's own fields). Under the minion actor no write is dropped and no dependency falls back to the character's field. Test: `engine::tests::the_minion_actor_should_write_and_read_its_own_fields` (a minion write leaves the character's field alone; a dependency the minion did not compute makes the derived field `DependsOnNotComputed`).
- **The minion's enemy states** are every `Enemy:` state the rules settle (`states::states`), not only the config's checks.
- **`SupportOnMinion` drops the enemy's quantities**, which come with the enemy's effects (no double count).
- **`ExtraMinionSkill`:** the summoning skill's gems' stats that PoB's stat map turns into an extra minion skill (Infernal Legion, the Radiant Sentinel) are appended to the minion's list, as PoB's `CalcActiveSkill.lua` 880 to 884. From the character's lines it comes through unmapped stats, the character's reasons.
- **The minion skill's own list** is the summoning skill's supports (`set_skill_sources`): PoB rounds their mores apart.
- **The minion's ailments, damage over time, and hit speed** are `not-computed` until the slice compares them.
- **`conditions::settle`** removes only an entry it set itself.
- **The minion's weapon id** is a sentinel no build item has (`ItemId(u32::MAX)`).

Noted: the minion's life and mana thresholds (`STAT_THRESHOLD`, `LIFE_THRESHOLD`) are never settled for it, so a gem row that needs them is the minion's reason.

**Evidence:** public 186 builds and corpus 21, 0 unexpected, no compared value moved. Both runs are byte-identical. The sweep is unchanged, 0 unexpected.
