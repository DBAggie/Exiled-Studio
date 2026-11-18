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

**Versions:** an older application reading a newer bundle ignores the file (the loader reads only the files it knows).

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
- **Values** are read as whole numbers. A value that is not one goes in `values` as text, not dropped (ADR-015). PoB's `tonumber` gives nil for these, and its exporter would fail on such a minion. Build 26530219 has two such types: a commented-out line and mercenary dialogue scripts.
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
