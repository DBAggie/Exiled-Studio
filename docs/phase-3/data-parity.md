# Data parity, bundle against PoB v2.67.2's data (P3-2): evidence

Phase 3 plan, step 2 (`dev/active/phase-3-calc-slice/`, decision 5): before any calc comparison, the values a calculation reads are compared between our bundle (the current game) and the data PoB v2.67.2, the oracle, holds. Each difference gets a category with cited evidence, so a later calc difference can be told apart from a data-version difference. Maintainer workstation, Windows, 2026-11-11.

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
| skills | 495,396 | 205 | 13 | 11,332 (effects, levels, 35 effects' additional stat sets) |
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

## Not compared
Listed in the summary's `not_compared`: level requirements, Vaal soul costs, interpolation kinds, PvP and PoB's own skill model, support constraints (allowed, added, and excluded types; P3-9a compares them), additional stat sets (PoB exports the main set only), gem display data, tree geometry, and the monster columns the bundle does not carry. Bundle rows and levels PoB does not have are listed in `report.json`, not gating: no PoB build reads them.

## Open for the maintainer
- The 8 Vaal transfigured gem tag differences stay `patch-delta` without a patch-note line: **confirmed by the maintainer 2026-11-13** (PoB copies `GemEffects.Tags` verbatim, so only the game changed them; the tags do not enter the calc slice). Recorded in P3-12h.
- `not-carried` is a fifth category, beside the plan's four.
- 15 ascendancy nodes (`passives-buff-text`) need either the passive buff tables in the bundle (a format change) or a `Partial` state when allocated. P3-3 and P3-8 decide.

## Tests
19 tests in `tools/parity-cli/src/data_parity/tests.rs` run on the synthetic PoE1 bundle and a hand-written export equal to it. They cover: no difference on equal data; one difference per changed value in each table; rows and levels on one side; how levels are found; the crit-chance columns; absent against 0; rendering; base name matching; each rule check, the committed rules file included; categories and the overlay; exact counts and stale keys; first match; and refused inputs. Workspace: 874 tests; clippy, fmt.
