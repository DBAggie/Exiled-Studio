# Data parity, bundle against PoB v2.67.2's data (P3-2): evidence

Phase 3 plan, step 2 (`dev/completed/phase-3-calc-slice/`, decision 5): before any calc comparison, the values a calculation reads are compared between our bundle (the current game) and the data PoB v2.67.2, the oracle, holds. Each difference gets a category with cited evidence, so a later calc difference can be told apart from a data-version difference. Maintainer workstation, Windows, 2026-11-11.

## How
1. `tools/oracle/data/pob_data.lua` (maintainer-run, outside the oracle adapter and its protocol) starts PoB with its own `HeadlessWrapper.lua` on the pinned oracle runtime and writes the data PoB holds: `data.skills`, `data.gems`, `data.itemBases` (weapons and armour), the monster tables and constants of `src/Data/Misc.lua`, and the tree file `TreeData/3_29/tree.lua`. Command: `tools/oracle/data/README.md`.
2. `loom-parity-cli data-parity <bundle-dir> <pob-export> --rules docs/phase-3/data-parity-rules-poe1.json --out <dir>` puts each bundle value into PoB's form with the conversion PoB's exporter made (each cited in the summary's `transforms`), and compares exactly. Numbers are compared at 14 significant digits: PoB's data files were written with Lua's `tostring`.
3. Each difference gets the first rule of the rules file that matches its table, key, and field. A rule must match exactly its `count` and every one of its key patterns, so a new difference on a covered row fails the run. The rules file names the Steam build and PoB version it is for; other inputs are refused.

Categories: `patch-delta` (the game changed after PoB's export; a patch-note thread cited, and `named_in_notes` says whether the notes name the change), `pob-derivation` (PoB computes, edits, or writes the value itself; its v2.67.2 source line cited), `representation` (the same game value in another form), and `not-carried` (the game has the value in a column the bundle does not carry; not a game-versus-PoB difference, and a calculation that reaches it must report the field `Partial`). The overlay (`overlay.json`, for `calc-run --data-overlay` in P3-5) holds PoB's value for every `patch-delta` and `pob-derivation` difference.

## Inputs
- Bundle: P3-1c's PoE1 bundle (`local/p3/p3-1c/evidence2/bundle-poe1-a`, Steam build 26530219, `bundle.json` `6099814e…`, data schema 2).
- PoB export: PoB 2.67.2 (`b32759ab`, the pinned oracle), tree `3_29`, runtime LuaJIT 2.1.1784580905; SHA-256 `08a8c34e…`.
- Tool: clean commit `92199cc`.

## Results
Two exports are byte-identical, and the oracle checkout stayed clean (ignored files included). Two `data-parity` runs give byte-identical `summary.json`, `report.json`, and `overlay.json`. Exit 0. The summary is `data-parity-poe1-26530219.json` (counts and rules, no game values); the report and overlay stay local (`local/p3/p3-2/evidence/`).

| Table | Compared | Differences | PoB-only rows | Bundle-only (listed, not gating) |
|---|---|---|---|---|
| skills | 496,904 (P3-12i; 495,396 before the colour) | 205 | 13 | 11,332 (effects, levels, 35 effects' additional stat sets) |
| passives | 27,168 | 214 | 0 | 2,310 |
| bases | 6,550 | 5 | 5 | 10 |
| gems | 5,220 | 8 | 0 | 422 |
| monster levels | 1,100 | 100 | 0 | 0 |
| mastery effects | 353 | 1 | 0 | 1 |
| character constants | 89 | 1 | 0 | 0 |
| monster constants | 57 | 0 | 0 | 0 |
| game constants | 42 | 0 | 0 | 0 |
| characters | 28 | 0 | 0 | 0 |
| **All** | **536,003** | **534** | **18** | |

| Category | Differences | Rules |
|---|---|---|
| `patch-delta` | 130 | 2 |
| `pob-derivation` | 184 | 7 |
| `representation` | 57 | 5 |
| `not-carried` | 163 | 3 |
| `uncategorized` | **0** | |

**Game versions.** PoB v2.67.2 is dated 2026-08-07; its data is from 3.29.2 or earlier. The bundle's build is 3.29.3 or later: the [3.29.3 patch notes](https://www.pathofexile.com/forum/view-thread/3996516) (2026-08-12) fix level-one Conflagration, and the bundle has the fixed row where PoB has the bugged one. Outside these rows, the two sides agree on every compared value.

## The differences
| Rule | Category | Count | What |
|---|---|---|---|
| `skills-conflagration-level-1` | patch-delta (named) | 122 | Level one fixed in 3.29.3. PoB's bugged level 1 also sets its stat order, so every later level's values sit under the wrong names. PoB's own later re-export (dev `3dbca44`, 2026-10-29) matches the bundle. |
| `gems-vaal-transfigured-tags` | patch-delta (**not named**) | 8 | Tags of eight Vaal transfigured gem effects. PoB copies `GemEffects.Tags` verbatim, so only the game can have changed them; the notes do not name it. Tags do not enter the slice. |
| `skills-pob-written` | pob-derivation | 13 | Skills PoB writes by hand (monsters, minions, spectres); no granted effect in the game has the ID. |
| `skills-pob-written-levels` | pob-derivation | 37 | Monster skills whose stats and levels PoB replaces by hand. |
| `skills-exporter-stat-order` | pob-derivation | 26 | A level adding a stat the first level lacks: PoB's exporter appends the stat ID but writes the values in row order, so values land under the wrong names (10 ancestor skills, Animate Guardian's Weapon). |
| `skills-pob-added-types` | pob-derivation | 6 | `#addSkillTypes` lines and one hand-written type list. |
| `skills-windburst-level-31` | pob-derivation | 1 | PoB pairs level rows by index; the game's stat set has no level 31. |
| `monster-armour-derived` | pob-derivation | 100 | PoB computes monster armour instead of reading `DefaultMonsterStats.Armour`; P3-11 decides. |
| `passive-harness-the-void` | pob-derivation | 1 | PoB's tree file puts a Trickster node under Saboteur. |
| `passives-cluster-mastery-icons` | representation | 38 | Cluster jewel mastery icons: `IsJustIcon` in the game, `isMastery` in the tree file. |
| `passives-tree-text-wording` | representation | 8 | Same stats and values, other wording or number format in the tree file. |
| `passives-eldritch-notable` | representation | 5 | Notable in the tree file, not in the game row. |
| `bases-pob-names` | representation | 5 | Bases PoB renames (Energy Blade, the three Two-Toned Boots). |
| `mastery-tree-text-wording` | representation | 1 | Same, for one mastery effect. |
| `passives-bloodline` | not-carried | 147 | Bloodline nodes link through `PassiveSkills.DescendancyKey`, which the bundle does not carry. |
| `passives-buff-text` | not-carried | 15 | Ascendancy nodes whose stats are in a granted buff or skill (`PassiveSkillBuffs`, `GrantedEffectsPerLevel`). |
| `character-base-speed` | not-carried | 1 | `base_speed` is in `Character.ot`'s Pathfinding block. |

Evidence, sources, and key lists are in `data-parity-rules-poe1.json`.

## P3-12h: table `gem_effects`
Not bundle data: the engine's other-gem tables (`loom_calc::poe1::gems`) against PoB's skill stat maps (the export's format 4), for every entry the bundle holds; any difference is uncategorized. 255 values, 0 differences (`docs/phase-3/other-gems.md`, "The check against PoB").

## P3-12i: the skills' `color`
The bundle carries each granted effect's colour (`attribute`, the game's `GrantedEffects.Attribute` as stored); PoB's export (format 5) writes each skill's `color`, which its exporter copies from the same column (`src/Export/Scripts/skills.lua` 157). Table `skills`, field `color`: 1,508 values compared (every skill both sides hold), 0 differences; the other counts are unchanged (534 differences, `uncategorized` 0). Two PoB skills without a colour (`HarvestNessaCrabScreechDebuff`, `HarvestRhexScreechDebuff`) are among the 13 PoB-only rows.

## B-2: tables `mod_affixes` and `base_requirements` (UX sprint, data schema 4)
The item tooltip's Alt view shows each mod's affix name and level, and the requirement line (`dev/active/ux-improvements/b2-item-data-plan.md`). The export (format 6) adds:
- `mods`: each mod of the keyed `data.itemMods` files (Explicit, ItemExclusive, Corrupted, Delve, Synthesis, Scourge, Eldritch, Flask, Tincture, Graft, Jewel, JewelAbyss, JewelCluster, JewelCharm, Foulborn), by the game's mod ID, which `Prefix:`/`Suffix:` spec lines name (`src/Classes/Item.lua` 1008 to 1010). Each has its `affix` and `level` as PoB's exporter wrote them (`src/Export/Scripts/mods.lua` 101, 114) and the files that hold it. `conflict` is set when two files disagree. `Item` (a merge of seven of them) and `WatchersEye` (derived) are not read. `data.masterMods` (crafted mods) has no IDs.
- `base_requirements`: every `data.itemBases` entry's `req` and whether it is a flask.

`mod_affixes` compares `affix` and `level` by mod ID. `base_requirements` matches by name as `bases` does, but over every base. Several game bases under one name compare as one when their values in PoB's form agree. When they disagree, or PoB names a variant (`Two-Toned Boots (Armour/Evasion)`, looked up by the game's name), the row differs, and PoB's values must equal one candidate's: field `values`, compared every time, so a changed value is never hidden behind the row's rule. The required level is put into PoB's form as its exporter derived it (`src/Export/Scripts/bases.lua` 276 to 293): the drop level when above 4 for a weapon or armour and above 2 for a flask, raised to `floor(level * 0.8)` of each implicit, and absent when 1. Attributes are absent when 0 (294 to 305). A bundle before data schema 4 is refused.

**Run on the schema 4 bundle** (Steam build 26530219, 2026-11-14; summary `docs/ui/b2-data/data-parity-poe1-26530219.json`): PoB's export has 23,230 mods, none in conflict, and 1,106 bases.
- `mod_affixes`: 46,460 values, **0 differences**; every PoB mod ID is in the bundle.
- `base_requirements`: 5,461 values, 23 differences, all `row`, all `representation`; the 23 `values` checks are equal:
  - `base-requirements-pob-names` (15): bases PoB names by variant (Two-Toned Boots, Two-Stone Ring, the Avian Twins and Monkey Paw Talismans, Energy Blade), as `bases-pob-names`. Each is matched by the game's name, and its values equal one of those game bases'.
  - `base-requirements-legacy-quivers` (8): a legacy and a current quiver under one name. PoB's spec writes the current one last, so `data.itemBases` holds it. Each current quiver's values equal PoB's (the `values` check).
  - 14 more names shared by several game bases compare as one, because their values agree: Ashbark, Blood Sap, Borealwood, Fulgurite, Ironwood, Oakbranch, Poisonberry, and Rosethorn Tincture; Avian Twins, Black Maw, Greatwolf, and Monkey Paw Talisman; Shadowed Ring; Two-Stone Ring.
- Totals: `uncategorized` 0, no failed rule; the 314 overlay rows are identical to the close-out's.
- **The first run found a converter bug.** The game keys `ComponentAttributeRequirements` by the base's ID string, not a row number, so the bundle carried no attribute requirements: 1,293 differences, all on Str, Dex, and Int. Fixed in `tools/parity-cli/src/bundle.rs`, with a test using the game's layout; the second run has none.
- Review (`code-architecture-reviewer`) applied: PoB's values are checked against the candidates for the 23 rows (they were left out before), the 15 variant names are compared through the game's names, and a mod PoB's files disagree on is not compared value by value.

## B-2: table `flavour_text` (the item card's flavour text, export format 8)
The card shows a unique's flavour text (`docs/ui/b2-data/card-data.md`, Q-B2-1). The export (format 8) adds `flavour_text`: each `data.flavourText` entry (`src/Data/FlavourText.lua`) with its ID, name, and lines as PoB's exporter cleaned them (`src/Export/Scripts/flavourText.lua`, `cleanAndSplit`). `flavour_text` keys a row by the unique's name and the ID without trailing underscores, cleans the bundle's text the way PoB's exporter does (`data_parity::flavour::pob_lines`), and compares the lines. A PoB row the bundle lacks is a difference; a bundle row PoB lacks is listed, not gating.

**Run on the B-2 card bundle** (Steam build 26530219, 2026-11-15, PoB export format 8, twice byte-identical): 1,356 PoB rows compared. **Every row both sides have is equal**: the bundle's text, cleaned as PoB cleans it, is PoB's.
- 56 differences, all `row`, rule `flavour-upgraded-version-shares-base-art` (`pob-derivation`): upgraded versions (fated uniques and the like) whose stash row has `BaseVersion` set and their base's art. PoB's export gives every stash row its art's text, so these names carry their base version's (Kaom's Way with Kaom's Sign's). The bundle takes their text only from the per-game list (Kaom's Way: `FatedUnique1`), so it has no such row.
- 32 bundle-only rows: base versions that share their art with an upgraded one (Blackheart, Bramblejack, ...). PoB's export keys its stash lookup by art, so one name per art survives there.
- Totals: 613 differences (not-carried 163, patch-delta 130, pob-derivation 240, representation 80), `uncategorized` 0, 0 failed rules, 370 overlay rows (the 56 new rows are never read: no calculation reads `flavour_text`).

**B-2 item evidence data run** (same build and export, 2026-11-15, twice byte-identical; `docs/ui/b2-data/item-evidence.md`, "Main workstation data run"): one more difference, `The Light of Meaning|UniqueJewel181` (`row`), rule `flavour-game-copy-over-shared-art` (`pob-derivation`, count 1). The Light of Meaning shares its art with That Which Was Taken, so PoB's export gives it that art's text; the maintainer's in-game copy shows `FlavourText` row `UniqueJewel183_`, which the bundle now pairs with it (`poe1::FLAVOUR_PAIRS`). Its `UniqueJewel183` row is bundle-only. Totals: 614 differences (pob-derivation 241), `uncategorized` 0, 0 failed rules, 371 overlay rows. Summary: `docs/ui/b2-data/data-parity-poe1-26530219-evidence.json`.

## T3: table `cluster_jewels` (cluster jewels, export format 9)
The passive tree's cluster jewels (`docs/tree/cluster-jewels.md`) come from the game's expansion tables, which PoB exports the same way (`src/Export/Scripts/cluster.lua`). The export (format 9) adds `cluster_jewels`: `data.clusterJewels` with each jewel by base name, the notables' sort order, the keystones, and the proxies' start indices. `cluster_jewels` (`data_parity::cluster`) compares, keyed as written:
- `jewel|<base name>`: the layout;
- `skill|<base name>|<passive ID>`: the tag and name (PoB adds " (Legacy)" to a retired tag's);
- `notable|<name>`: the order, PoB's 1-based stat row;
- `keystone|<name>`;
- `proxy|<node>`: the start indices.

A PoB row the bundle lacks is a difference. A bundle without the file (an extract made before T3) has PoB's rows only. Run on PoB's side here: 9 jewels, 300 notables, 8 keystones. The run against a bundle with the table is maintainer-run.

## P6-4b: table `flasks` (flask bases, export format 11)
The flask bases (`poe1/flasks.json`, `docs/phase-6/flasks.md`) against PoB's `flask` base data, by name:
- life and mana per use;
- the duration in seconds;
- the buff's lines: the bundle's stats rendered together with `tincture_stat_descriptions`, the file PoB's exporter renders them with, compared sorted.

On the flask evidence bundle: 48 flasks, 192 values, 0 differences.

## P6-7a: table `minions` (monster varieties, export format 12)
PoB's minions and spectres against the bundle's monster varieties (`poe1/minions.json`, `docs/phase-6/minions.md`), by variety ID, in PoB's form. On the minion evidence bundle: 332 minions, 5,959 values. The one difference is `GuardianRelicAll`, which PoB writes by hand: rule `minions-pob-written`.

## P6-7b1: table `minion_mods` (minion mod lists, export format 13)
Each minion's and spectre's mod list as PoB's exporter commented it in `Data/Minions.lua` and `Data/Spectres.lua` (`-- <mod> [<stat> = <value>]`, read from the files' text). It is compared against the bundle's list, by variety ID: the variety's mods and special mods, one line per stat with the lowest value, then its object type's `.ot` stats (`docs/phase-6/minions.md`, "P6-7b1").

On the `.ot` evidence bundle: 332 lists, 700 lines. Every `.ot` stat matches. The 17 differences have two rules:
- `minion-mods-six-stat-slots`: PoB reads a mod's first six stats only, so `CannotBeAugmented`'s seventh, `monster_no_talismans`, is missing on 16 minions.
- `minion-mods-pob-written`: `GuardianRelicAll`.

Export format 13 also writes `minion_templates`, PoB's template directives. They are not compared: PoB's own data, which `pob-minions` copies into the engine's table.

## Data schema 5: gem level requirements and Vaal souls (export format 14)

Data schema 5 carries, per gem level (`GrantedEffectsPerLevel`):
- the character level requirement (`PlayerLevelReq`);
- the Vaal soul cost (`VaalSouls`);
- the stored Vaal uses (`VaalStoredUses`);
- the soul gain prevention (`SoulGainPreventionDuration`, milliseconds).

PoB export format 14 writes `levelRequirement`, `vaalStoredUses`, `soulPreventionDuration`, and the `Soul` cost, all compared.

**Result on the evidence bundle** (`b-gl`, build 26530219):
- The soul costs, stored uses, and soul prevention durations equal PoB's.
- 16 level requirements differ. All are monster or minion skills with a single level, whose requirement PoB's exporter takes from the stat set's level row instead (`Export/Scripts/skills.lua` 267). Rule `skills-level-requirement-single-level`, `pob-derivation`; each PoB value was checked against `GrantedEffectStatSetsPerLevel.PlayerLevelReq`.
- `uncategorized` stays 353.

## The cluster tables on build 26530219

With `poe1/cluster_jewels.json` in the bundle (evidence bundle `b-cl`, `docs/tree/cluster-jewels.md`), the 353 cluster rows of PoB's are compared, and `uncategorized` is 0. The one difference left is PoB's notable order for the duplicated name Evil Eye: rule `cluster-notable-order-duplicate-name`.

## Not compared
Listed in the summary's `not_compared`: interpolation kinds (level requirements and Vaal soul costs since data schema 5, above), PvP and PoB's own skill model, support constraints (allowed, added, and excluded types; P3-9a compares them), additional stat sets (PoB exports the main set only), gem display data, tree geometry, and the monster columns the bundle does not carry. Bundle rows and levels PoB does not have are listed in `report.json`, not gating: no PoB build reads them.

## Open for the maintainer
- The 8 Vaal transfigured gem tag differences stay `patch-delta` without a patch-note line: **confirmed by the maintainer 2026-11-13** (PoB copies `GemEffects.Tags` verbatim, so only the game changed them; the tags do not enter the calc slice). Recorded in P3-12h.
- `not-carried` is a fifth category, beside the plan's four.
- 15 ascendancy nodes (`passives-buff-text`) need either the passive buff tables in the bundle (a format change) or a `Partial` state when allocated. P3-3 and P3-8 decide.

## Tests
19 tests in `tools/parity-cli/src/data_parity/tests.rs` run on the synthetic PoE1 bundle and a hand-written export equal to it. They cover: no difference on equal data; one difference per changed value in each table; rows and levels on one side; how levels are found; the crit-chance columns; absent against 0; rendering; base name matching; each rule check, the committed rules file included; categories and the overlay; exact counts and stale keys; first match; and refused inputs. Workspace: 874 tests; clippy, fmt.
