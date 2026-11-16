# Phase 3 report: calc slice

Phase 3 of `docs/plan/ACTION-PLAN.md` ("Calc slice"), planned in `dev/completed/phase-3-calc-slice/` (plan, context, tasks; plan PR #59). Report written 2026-11-13 in the Phase 3 close-out PR (#104; P3-14 and P3-15, with the last of P3-8 and P3-12, in one PR at the maintainer's request).

**Verdict: the gate is met.** Every gate item below holds on the recorded inputs (section 2). The 7 in-scope builds of the two chosen skills (Ice Nova: 3, Smite: 4) have every computed slice field `Complete`; their only differences from PoB are 20 registered entries, all one cause: the game floors a weapon's attack time to whole milliseconds and PoB does not (Q11, confirmed in game 2026-11-12). Items 4, 5, and 6 run in CI on every PR; items 1, 2, and 3 need the real game data, which stays local (Q1), so the maintainer runs them and the summaries are committed (`docs/phase-3/phase-3-report/`). The maintainer confirms the verdict by merging the PR.

The slice fields do not stand for whole-build DPS: `TotalDPS` here is the main skill's hit DPS. For some builds most of the damage is elsewhere (`boneshatter-juggernaut`: `TotalDPS` is about 10% of PoB's `CombinedDPS`).

## 1. What was built
Fifteen plan steps in PRs #59 to #103 (#62 is Phase 4's UI reference material, #79 a docs PR for Q11) and the close-out PR, merged into `dev` (since 2026-11-11 the integration branch; before that a stack into `main`). Each step's decisions and review are in the context file (`dev/completed/phase-3-calc-slice/phase-3-calc-slice-context.md`).

| Step | PRs | What | Evidence |
|---|---|---|---|
| 0 Plan | #59 | The plan, 13 questions, the gate | plan |
| 1 Bundle v2 | #60, #61, #63, #75 | Skill, character, passive, item property, and monster tables; monster types for the default enemy | `bundle-v2-*.md`, `monster-types.md` |
| 2 Data parity | #64 | `data-parity` against PoB's own data: every difference categorized (`uncategorized` 0) | `data-parity.md` |
| 3 Scope | #66, #67 | `calc-survey`, the scope file, the chosen skills (Ice Nova, Smite) and their in-scope builds | `calc-survey.md` |
| 4 Engine core | #68 | `loom-calc`: modifier store, provenance graph, field states with reasons, `CalcInput` | `docs/phase-3/` |
| 13 Result document | #69 | Calc result document v1, schema, TypeScript | `schemas/calc-result.v1.schema.json` |
| 5 Parity harness | #70 | `calc-run`, `calc-parity`, the `mechanic` fixture tier, oracle adapter protocol 4 | `calc-parity.md` |
| 6 Character | #71 | Attributes, life, mana, energy shield, unreserved pools | `character-fields.md` |
| 7 Defences | #72 | Armour, evasion, resistances, block, spell block | `defence-fields.md` |
| 9 Skills and spells | #74, #77 | Skill model; a self-cast spell's hit (Ice Nova) | `skill-model.md`, `spell-hits.md` |
| 10 Attacks | #78, #81 | A main-hand attack (Smite); the weapon's attack time floored in ms (Q11) | `attack-hits.md` |
| 11 Enemy | #76 | The default enemy, as PoB's Pinnacle boss | `enemy-model.md` |
| 8 Stat mapping | #73, #80, #83, #93 to #103, close-out | Every stat the in-scope builds reach: mapped, `NoEffect` with guards, or a reason; rolled values of PoB's ranged lines | `stat-mapping.md`, `rolled-values.md` |
| 12 Buffs and config | #82, #84 to #92, close-out | Config inputs, placeholders, custom modifier blocks, charges, the enemy's damage taken, the pantheon, the other gems, socket colours, custom notes and patterns (ADR-015 Amendment 2) | `config-inputs.md`, `other-gems.md`, `custom-lines.md` |
| 14, 15 Corpus run, report | close-out | This report; the scrubbed summaries | `phase-3-report/` |

The close-out PR added: the gate builds' other gems (golems and Lightning Golem's Wrath, Heralds of Ash and Purity, Frost Bomb, Frostbite and Elemental Weakness, Flame Wall, Flesh and Stone, Righteous Fire of Arcane Devotion, Intimidating and Rallying Cry, Arcane Surge Support, Guardian's Blessing, Automation, Convocation, Shockwave; `other-gems.md`, "Phase 3 close-out"); custom notes and patterns, checked against PoB's own parse of every fixture line (Q13 option (b); `custom-lines.md`; ADR-015 Amendment 2); items in several slots and abyssal sockets; the body armour's Life Modifiers settled by PoB's affix record (`stat-mapping.md`, "Phase 3 close-out").

## 2. Inputs
- **Game data:** PoE1 Steam build 26530219, extracted and converted to a bundle (format 2, data schema 3; P3-8n's, `local/p3/p3-8n/bundle-poe1-a`, `bundle.json` SHA-256 `3f90c6d5f7031c2b…`). Local only (Q1).
- **Oracle:** PoB v2.67.2 (`b32759ab`), pinned in `tools/oracle/manifest.json`, adapter protocol 4. Its data export for `data-parity`: P3-12i's (`local/p3/p3-12i/evidence/pob-data-1.json`).
- **Public fixtures:** `tools/oracle/fixtures/` (ours, the `mechanic` tier, and 21 credited community builds) and PoB's own `spec/TestBuilds/3.13`: 136 builds (`malformed.xml` is meant not to import).
- **Corpus:** the maintainer's 21 PoB builds (local only, ADR-007; summaries name builds by SHA-256). Its oracle run is P3-5's (`local/p3/p3-5/evidence-p4/oracle-corpus`).
- **Slice:** `docs/phase-3/slice-fields.json` revision 2; scope `docs/phase-3/calc-scope-poe1.json`.
- **Register:** `docs/parity/divergence-register.json`, 71 accepted entries, all `game-truth`.
- **Tool commit:** the full script ran at `0cd2018` (the close-out branch) from a clean tree; documentation edits made while it ran mark its later summaries `dirty`, so the `calc-run`s (public, overlay, gate, corpus) and the three `calc-parity` comparisons were made again at `168a1bb` (same code, docs only after `0cd2018`) on a clean tree: every calc document byte-identical to the first run, and the committed summaries are these (`dirty: false`).

## 3. Gate verdict
The gate is the plan's "Gate (objective, checkable)". Commands run from the repository root; the full script is `local/p3/close/evidence.sh` (local).

### 1. In-scope builds match: met
```
loom-parity-cli calc-run <the 7 builds> --bundle <bundle> --scope docs/phase-3/calc-scope-poe1.json --out <gate>
loom-parity-cli oracle-run tools/oracle/manifest.json local/oracle --out <gate-oracle> <the 7 builds>
loom-parity-cli calc-parity <gate-oracle> <gate> --slice docs/phase-3/slice-fields.json --register docs/parity/divergence-register.json --out <dir>
```
Result at `0cd2018` (`close-gate-calc-parity-summary.json`): `calc-run` 7 builds, fields **158 `Complete`**, 0 `Partial`, 0 `Indeterminate`, 10 `Unsupported` (`not-computed`, below). `calc-parity` exit **19** (`expected`): 3 builds without difference, 4 with only registered ones, 0 unexpected; fields 147 match and 11 registered differences; 20 registered differences in all; `unused_entries` empty.

- **The denominator** is the survey's in-scope builds of the chosen skills: 3 Ice Nova Elementalist and 4 Smite Inquisitor builds, all public community fixtures (P3-3b), so the minimum of 3 per skill holds. The corpus has none in scope: `calc-survey` on the corpus gives 0 of 21 (tree versions and excluded mechanics; `calc-survey.md`), so on the corpus, gate item 1 has nothing to check, and item 4 applies (below).
- **Not computed is not missing.** `HitSpeed` is compared only when the oracle has it ("only when the oracle has it", `slice-fields.json`); PoB leaves it absent for Ice Nova and for Smite. `HitChance` is an attack's: PoB leaves it absent for a spell. These 10 fields are `Unsupported` (`not-computed`) on both sides, never `Complete` with a value.
- **The differences** are DIV-0052 to DIV-0071 (`game-truth`, Q11): each Smite build's attack rate is the game's (attack time floored to whole ms after increases), so its rate, hit DPS, and (through Intimidating Cry's exerted-attack uptime, exerts divided by the rate) average damage differ from PoB's. Evidence: the maintainer's in-game reading (1.72, 2026-11-12) and `attack-hits.md`, "The switch"; each entry names its test (`crates/loom-calc/tests/attack.rs`).
- **No `data-version` entry** is used, so the overlay rule has nothing to excuse; the overlay run is still made and compared (item 2).

### 2. Mechanic fixtures match: met
```
loom-parity-cli calc-run <public fixtures> --bundle <bundle> --scope <scope> --out <public>
loom-parity-cli calc-run <public fixtures> --bundle <bundle> --scope <scope> --data-overlay <data-parity overlay.json> --out <overlay>
loom-parity-cli calc-parity <oracle> <public> --slice docs/phase-3/slice-fields.json --register docs/parity/divergence-register.json --overlay-run <overlay> --manifest tools/oracle/fixtures/fixtures.json --out <dir>
```
Result at `0cd2018` (`close-public-calc-parity-summary.json`): **2078 of 2078 mechanic targets met over 99 mechanic fixtures** (match, or a registered `game-truth` difference); `missing_mechanic` empty. The whole public run: 136 builds, 113 without difference, 22 with only registered ones, 0 unexpected, 1 expected failure (`malformed.xml`); 71 registered differences, 0 unexpected; PoB's views differ in none. Its status is `incomplete` (exit 5) because 27 out-of-scope builds have fields not compared (`Partial` or `Indeterminate`), which item 4 covers.

### 3. Breakdowns match: met
For the 7 builds: 62 lines match, 4 differ (registered), 4 absent (`close-gate-calc-parity-summary.json`). The 4 differences are the Q11 entries above (the `TotalDPS` breakdown's rate and DPS lines, and the average-damage line through Intimidating Cry). `absent` is a line PoB does not print for that skill (hit chance for a spell, hit rate without one), absent on both sides. Over all public fixtures and mechanic fixtures, every breakdown difference is registered (`close-public-calc-parity-summary.json`).

### 4. No silent results anywhere: met
- **Every build that imports has a state for every slice field** (`calc-run`'s documents; the result DTO requires it). Public fixtures: 135 builds that import, 3240 fields: 2270 `Complete`, 176 `Partial`, 482 `Unsupported`, 312 `Indeterminate`. Corpus: 21 builds, 504 fields: 141 `Unsupported`, 363 `Indeterminate`; every corpus field is `Indeterminate` or `Unsupported` with its reasons (the corpus builds are out of scope: other tree versions, timeless and cluster jewels).
- **No `Complete` field while an input reaching it is unmodelled.** Tests in CI: an unmapped stat (`an_unmapped_stat_should_make_fields_partial_never_a_silent_zero`, `an_unmapped_stat_should_make_the_fields_partial`), an unresolved or unread line (`unread_lines_should_be_reasons`, `items_beyond_their_lines_should_be_reasons`), gems, config inputs, and choices (`gems_config_and_choices_should_be_reasons`, `other_gems_and_unmapped_gem_stats_should_be_reasons`, `untyped_config_content_should_be_a_reason`), a tree version other than the bundle's (`the_tree_version_should_be_checked_when_passives_are_allocated`), an unknown class or passive (`Indeterminate`). Custom lines: a note has no effect and is never a reason by ADR-015 Amendment 2; a line that reads as a modifier's sentence is never a note (`MODIFIER_WORDS`, added after review), and `custom-lines` checks that no note is a line PoB applies (46 notes, 0 applied by PoB).
- **No `Complete` field of an out-of-scope build has an unregistered difference:** the public run has 0 unexpected differences (item 2's run).

### 5. Divergence register in use: met
`loom-parity-cli divergence-check docs/parity/divergence-register.json` runs in CI (job `rust`, step "Divergence register"); each of the 71 entries names its test, which runs in CI. `unused_entries` is empty on the latest public run (`close-public-calc-parity-summary.json`).

### 6. Determinism and hygiene: met
- **Byte-identical runs:** two `calc-run`s of the public fixtures, and two of the corpus, give identical documents (`diff -r`; at `0cd2018`, 2026-11-13). Across Windows and Linux: CI `dto cross-platform` compares the calc result documents of the synthetic builds.
- **wasm32:** CI job `wasm32 core crates` (`loom-domain`, `loom-data`, `loom-parser`, `loom-calc`, `loom-serialization`).
- **PoE1 identifiers:** `no_poe1_identifiers` scans the shared crates; the calc's PoE1-only names live under `loom_calc::poe1`.
- **Schema freshness and `cargo deny`:** CI, required in `ci-ok`.
- **`data-parity`:** 534 differences (not-carried 163, patch-delta 130, pob-derivation 184, representation 57), **`uncategorized` 0**, 0 failed rules; the `gem_effects` table: 453 values, 0 differences. Each category has its evidence in `docs/phase-3/data-parity.md`.

## 4. Coverage of out-of-scope builds
States by count over every slice field of every build that imports (one `malformed.xml` is meant not to):

| Run | Builds | Complete | Partial | Unsupported | Indeterminate |
|---|---|---|---|---|---|
| Public fixtures | 135 | 2270 | 176 | 482 | 312 |
| Corpus | 21 | 0 | 0 | 141 | 363 |
| The 7 in-scope builds | 7 | 158 | 0 | 10 | 0 |

Against P3-8n's run (`close-public-vs-p3-8n.json`, `close-corpus-poe1-26530219-vs-p3-8n.json`): on the public fixtures, every computed field of the 7 gate builds went from `Partial` to `Complete`, 23 builds changed values, and 189 reasons are gone (119 `unmodelled-gem`, 49 `unresolved-line`, 10 `unmodelled-item`, 6 `unmapped-stat`, 5 `unmodelled-config`) against 29 added. On the corpus, 15 builds changed values, all in `Indeterminate` fields; 60 reasons are gone and 12 added. The added ones are inputs the close-out now reaches, honest as reasons: stats of jewels in abyssal sockets and of items in a second slot, now counted; other gems of the builds whose skills are now modelled; and two typed reasons of the new models (`socket-colour-quality`, `rallying-cry-allies`).

`Unsupported` is mostly `not-computed`: hit fields of skills outside the slice (damage over time, minions, totems, traps, mines, triggers, channelling, dual wielding) and `HitSpeed`/`HitChance` where PoB has none. `Indeterminate` is a build the engine cannot place: a tree version other than the bundle's (`3_29`), unknown passives, or missing data. The reasons per build are in each document (`reasons`) and the run's `summary.json`.

## 5. Scrubbed summaries
In `docs/phase-3/phase-3-report/`: `close-gate-calc-run-summary.json` and `close-gate-calc-parity-summary.json` (the 7 builds), `close-public-calc-run-summary.json` and `close-public-calc-parity-summary.json` (every public and mechanic fixture, with each mechanic target met or not), `close-corpus-poe1-26530219-calc-run-summary.json` and `close-corpus-poe1-26530219-calc-parity-summary.json` (builds by SHA-256), and the comparisons with P3-8n's run (`close-public-vs-p3-8n.json`, `close-corpus-poe1-26530219-vs-p3-8n.json`: values, states, reasons). The full runs stay in `local/p3/close/evidence/` (ADR-007).

## 6. Known limitations
- **Slice only.** Damage over time, ailments, minions, totems, traps, mines, brands, triggers, mirages, channelling, dual wielding, and `CombinedDPS` beyond hit DPS are `Unsupported`; cluster and timeless jewels are reasons (plan, "Out of scope").
- **Winter Orb** is in scope by archetype but not a chosen skill and not in the gate's denominator. Its own mechanics (channelling stages, Bonechill, Ice Bite, Assassin's Mark, flasks) were left by the plan's pre-approved fallback; the gems it shares with the gate builds are modelled.
- **Smaller stat mappings** no gate build needs: socketed gem levels, "with Brand/Minion Skills", the other weapon conditions. Each such stat is a reason (`unmapped-stat`), not a silent zero.
- **Q12 is open:** where global and skill attack speed enter relative to the floored attack time. The engine follows PoB's order; the maintainer's reading (2026-11-13) puts global speed after the floor. The Q11 entries hold for PoB's order and must be re-examined when Q12 closes.
- **Heuristic custom notes** (ADR-015 Amendment 2): outside the fixtures, a numberless line PoB applies without a modifier's word would still become a note. `custom-lines` is the check, run by the maintainer on new fixture lines.
- **An item in two slots** counts in each, as PoB counts it; the game cannot equip one item twice, so such a build is taken as written.
- **Real data is not in CI** (Q1 stays on its default): parity regressions on real builds show only in the maintainer's runs. CI runs the synthetic builds, the mechanic tests, and the cross-platform comparison.
- **The document versions:** build v1 and calc result v1 grew additively until this gate (`dto/calc/v1/mod.rs`, "Adding to v1"). From here, any shape change is a new version with a migration.

## 7. Open for the maintainer
- Merge the close-out PR into `dev` (and `dev` into `main` when you choose); that accepts ADR-015 Amendment 2 and the gate verdict.
- Q12 (above), Q1 (a pruned real bundle in CI), and the Phase 2 questions taken on their defaults.
- Next: Phase 4 (application slice and MCP addon), which starts with its own plan PR.

## 8. Reproduce
The commands are in `AGENTS.md` ("Commands") and section 3. The local inputs' paths are in `docs/handoff/SUCCESSOR-HANDOFF.md` (Workstation and Local resources).
