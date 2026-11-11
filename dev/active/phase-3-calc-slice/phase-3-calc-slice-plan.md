Last Updated: 2026-11-11 (P3-4: engine core)

# Phase 3: calc slice (plan)

## Summary
This plan covers Phase 3 of `docs/plan/ACTION-PLAN.md` ("Calc slice"), which follows Phase 2 (gate met 2026-11-11, `docs/phase-2/phase-2-report.md`). The ACTION-PLAN gives the scope:
- self-cast spell and non-triggered attack archetypes;
- core defences;
- a structured breakdown graph;
- completeness states;
- the divergence register in use.

**Gate (ACTION-PLAN):** corpus builds in scope match the oracle or have registered, evidenced divergences.

ADR-013 narrows the archetypes to **one self-cast spell and one non-triggered attack, no minions, with core defences**; builds outside them still import and show an explicit partial or unsupported state. This plan reads "in scope" through ADR-013: a build is in scope when its main skill is one of the two chosen skills and it passes the archetype rules (decision 3). Where `docs/plan/IMPLEMENTATION-PLAN.md` Phase 3 ("Rust vertical parity core") does not contradict the ACTION-PLAN, its work packages add detail; its application-task packages (WP1, WP2, WP6) belong to the ACTION-PLAN's Phase 4 (decision 12).

## Current state (2026-11-11, PR #42's branch `feat/p2-1-domain-model` at `020e218`, which holds the whole Phase 2 stack)
- `loom-calc` holds only its crate docs. Nothing computes.
- `loom-domain` has the `Build` aggregate: character (class, ascendancy, level, main socket group), passive specs (tree version, nodes, mastery effects, jewel sockets), skills (socket groups, gems with level, quality, enabled, main active skill), items with every line resolved to stat IDs, a typed `kind`, or unresolved with a reason (ADR-015 Amendment 1), item sets and slots, and config inputs as typed key-value pairs. `Completeness` is `Complete` or `Partial` (a calculation adds `Indeterminate`, ADR-014).
- `loom-data` bundle format v1 holds only what item import reads: stats, mods, base items, item classes, client texts, item rules, lookups, stat descriptions, and the `poe1` section (grafts, quality types). **No passive, character, skill, gem, weapon or armour property, or monster table is in the bundle.** The extractor already extracts `PassiveSkills`, `PassiveSkillMasteryEffects`, the passive skill graphs (`*.psg`), `Characters`, `Ascendancy`, `ActiveSkills`, `ActiveSkillType`, `GrantedEffects`, `GrantedEffectsPerLevel`, `GrantedEffectStatSets`, `SkillGems`, `GemTags` (Spike A); per-level stat sets, weapon, armour and shield types, and default monster stats are not extracted yet.
- The oracle (Spike C) runs PoB v2.67.2 on the 30 public fixtures in CI (required `oracle` jobs) and on the 21 corpus builds on the workstation. Each output holds `main_output` (706 fields for a community build) and ten breakdowns (`Life`, `Mana`, `EnergyShield`, `Armour`, `Evasion`, the four resistances, `TotalDPS`) as PoB's display lines (`"2074 ^8(base)"`, `"x 2.84 ^8(increased/reduced)"`, `"= 5890"`).
- `oracle-compare` compares two oracle documents leaf by leaf, exactly, classified with the divergence register (`docs/parity/`, empty, format 1). An absent field on one side is a difference.
- Real game data stays local (Phase 2 Q1 default); CI has synthetic PoE1 and PoE2 bundles only.

## Target state
| Crate or tool | What Phase 3 adds |
|---|---|
| `loom-data` | Bundle format v2: the calc tables (characters, passives and their graph, mastery effects, skills, gems, per-level stats, skill types, weapon and armour properties, monster defaults) as canonical types, game-tagged where game-specific, loaded and validated like v1. |
| `loom-calc` | A pure `calculate(&CalcInput) -> CalcResult` over an immutable snapshot (ADR-003): a modifier store with conditions, the stat-to-effect mapping (ADR-015; data where possible, code where necessary, PoE1 rules in a `poe1` module), character and defence phases, the skill model, spell and attack hit damage, the enemy model, and the buffs the target builds need. Every output carries a completeness state and a provenance graph. A cooperative cancellation hook; no threads, clocks, or I/O (ADR-016). |
| `loom-serialization` | The calc result document v1 (DTO, deterministic JSON, generated schema), so outputs and provenance are API data (ADR-003, ADR-014). |
| Tools | `loom-parity-cli`: `bundle` writes v2; `data-parity` (bundle values against PoB v2.67.2's data); `calc-survey` (archetype classification and mechanic inventory); `calc-run` (successor outputs); `calc-parity` (successor against oracle, completeness-aware, register-classified, with breakdown comparison). |
| Fixtures | Mechanic fixtures: small PoB XML builds of our own, one per supported mechanic, in the oracle fixture manifest (so CI's `oracle` job runs them) with the successor's expected comparison. |
| CI | Engine unit and property tests on synthetic bundles; determinism and cross-platform byte equality of calc results; the PoE1-identifier scan extended to `loom-calc`; calc parity on the public fixtures if Q1 is answered yes. |
| Report | `docs/phase-3/phase-3-report.md` with the gate verdict and scrubbed corpus summaries. |

## Design constraints and decisions
Decisions marked (Q) wait for the maintainer; their defaults are in the context file, "Open questions". The plan proceeds on those defaults.

1. **Compute only what the slice needs; report everything else.** The engine computes the **slice fields** (decision 4) for the main skill and the character. Any content that can affect a slice field and that the engine does not model (an unmapped stat, an unresolved or excluded item line, a typed `kind` without a handler, an unsupported skill, support, config input, or mechanic) makes that field `Partial` with a named reason from a closed list (`UnsupportedReason`). Nothing is ignored silently (ADR-014, ADR-015). A field is `Complete` only when every contribution was modelled.
2. **Stat IDs in, our own names out.** Inputs are stat IDs with values (items, passives, gems, bases) and typed resolutions. The stat-to-effect mapping turns each into effects on our own quantities (`Life`, `MaximumResistance(Fire)`, ...) with an operation (`Base`, `Increased`, `More`, `Flag`, `Override`) and conditions. PoB's output names (`main_output` keys) appear only in `tools/parity-cli`'s mapping, as dat-schema names appear only in the extractor (Phase 2 decision 8).
3. **Archetype rules decide "in scope", independently of the engine.** `calc-survey` classifies each build from game data, not from what the engine supports, so the engine cannot shrink its own gate by marking things unsupported. A build is in scope when:
   - its main skill (main socket group, main active skill) is one of the two chosen skills (Q2);
   - that skill is cast or used by the character: not triggered, not a totem, trap, mine, brand, minion, or mirage, by the skill's `ActiveSkillType` flags and its supports;
   - its tree is not old-tree (PoB converts those on load; Spike C limitation 9), and its tree version equals the bundle's passive tree version (the bundle holds one tree; another version gives `Indeterminate` with `CALC_TREE_VERSION`);
   - it uses none of the **excluded mechanics**: a closed list committed in P3-3, before any engine PR, each entry derived from game data (for example a timeless or cluster jewel in an allocated socket, by base item and jewel kind) and approved by the maintainer. Plan-review fact: 15 of the 21 community fixtures carry the `timeless-jewel` tag, Boneshatter's among them, so the list decides the denominator and is fixed before the engine can influence it.

   Everything else is out of scope: it is calculated as far as the engine goes and reported, but it is not in the gate's denominator. **Minimum denominator:** at least 3 in-scope builds per chosen skill across the corpus and public fixtures. If fewer exist, P3-3 adds public community builds of that skill under ADR-007 Amendment 1 (credited, removal on request) until 3 do. After P3-3, a build leaves the denominator only through the fallback, with the maintainer's approval and a recorded reason.
4. **The slice fields are a committed, versioned list.** `docs/phase-3/slice-fields.json` names each field, its PoB `main_output` key, its comparison (`exact`, or `abs`/`rel` with a written reason, ADR-003 "per-field tolerance metadata", ADR-006 "no broad close-enough rule"), and whether the oracle has a breakdown for it. Default list (Q3):
   - **Character and core defences:** `Str`, `Dex`, `Int`, `Life`, `Mana`, `EnergyShield`, `Armour`, `Evasion`, `FireResist`, `ColdResist`, `LightningResist`, `ChaosResist`, `BlockChance`, `SpellBlockChance`, `SpellSuppressionChance`, `LifeUnreserved`, `ManaUnreserved`.
   - **Main skill, hit:** `AverageHit`, `Speed`, `HitChance` (attacks), `CritChance`, `CritMultiplier`, `TotalDPS`.

   Damage over time, ailments, minions, and `CombinedDPS` beyond hit DPS are out of the slice; their absence is reported, not compared.
5. **The game is the truth; PoB v2.67.2 is the oracle; data versions are told apart first.** Our bundle comes from the current Steam build; the oracle reads PoB v2.67.2's data. Before any calc comparison, `data-parity` compares the values the slice reads (character base stats, passive node stats, gem per-level values and skill types, base item properties, monster defaults) between the bundle and PoB's `src/Data` and tree at v2.67.2. Each difference gets a category. The oracle pin does not move in Phase 3 (Q5).
   - **Data overlay, so `data-version` entries cannot hide formula bugs.** `calc-run --data-overlay <data-parity output>` runs the engine through a tool-side `CalcData` that substitutes PoB's values for exactly the rows `data-parity` found different. A calc difference is accepted as `data-version` (with patch-note evidence, ADR-001, `docs/parity/README.md`) only when the overlay run matches the oracle on that field; otherwise it is a calc difference like any other. The core keeps the game's data; the overlay exists only in the parity tool.
6. **Bundle v2 adds the calc tables; real data stays local unless Q1 says otherwise.**
   - v2 = v1 plus the calc tables in canonical types with our own names (Phase 2 decision 8). Game-specific tables sit in game sections; a PoE2 bundle loads with its own sections or without them (ADR-001).
   - The loader reads v2 only for calculation and still reads v1 for import; `bundle-check` validates both (new referential checks: node stats to stats, gems to granted effects, per-level rows to effects, classes to characters).
   - CI keeps synthetic bundles, grown with calc rows. Whether a pruned real PoE1 bundle is committed so CI can run calc parity is Q1 (ADR-000 allows shipping derived data; Phase 2 kept it local).
7. **The engine is a pure function over a snapshot (ADR-003, ADR-016).**
   - `CalcInput` holds the resolved `Build` (borrowed, immutable), a `CalcData` view (trait over the bundle, as `ItemData` is for import), and calculation options (which skill, which set). The build's `DataVersion` must match the data; a mismatch is a typed error.
   - `calculate` is deterministic: no hash-map iteration order reaches output, no clocks, no randomness. `f64` inside; outputs are canonicalized at the boundary with the rounding points written per field (PoB rounds some values in the middle; the engine rounds where the game does, and each rounding point has a test).
   - Only IEEE-exact operations (`+ - * /`, `sqrt`, `floor`, `round`, comparisons) are used on values that reach output; any other function goes through one audited module (`loom_calc::math`) with a cross-platform test, so Windows, Linux, and wasm32 give identical bits.
   - A cancellation hook (`&dyn Fn() -> bool`, polled between phases) lets the Phase 4 job runner stop a stale calculation; Phase 3 tests it but runs nothing in the background.
8. **The provenance graph is the breakdown.** Each output is a node; each contribution is an edge from a source (item line, passive node, gem, base, character, config input, enemy) with its operation and value. The value of every output equals the graph's evaluation (property test). Breakdowns for the UI and for comparison are views of the graph, never separate strings (ADR-003: explanations are API data).
9. **Comparison is layered (ADR-006).**
   - **Exact or tolerance per field** on `Complete` fields, through the divergence register.
   - **Semantic breakdown comparison** for the ten fields with an oracle breakdown: PoB's display lines are parsed into terms (base, increased, more, and the result) and compared with the same terms of our graph. PoB rounds each line for display (`"x 2.84"`), so each term is compared at the decimals its line shows (our value rounded the way PoB's display rounds it). The grouping of our graph into PoB's terms (which contributions form "base", which "increased/reduced", the order of `more` lines) is written per field in `slice-fields.json`, so the comparison checks structure without copying PoB's string format.
   - **Completeness:** a `Partial` field is not compared; the report lists it with its reasons. A field PoB has and we do not produce is listed as not computed, never as a pass.
   - `calc-parity` reuses `oracle-compare`'s register classification and exit codes (0, 19, 20; a failure class when a side failed).
10. **Mechanic fixtures grow with the engine.** Each engine PR adds small PoB XML builds of our own (a level 1 character with one item, one passive, one gem, ...) that isolate what it adds. They go into `tools/oracle/fixtures/fixtures.json` (tier `mechanic`), so CI's `oracle` job runs them; their successor comparison runs wherever the real data is (decision 6). A mechanic is "supported" only with a mechanic fixture that matches.
11. **Typed resolutions each get a handler or a reason (ADR-015 Amendment 1).** `granted-skill`, `item-property`, and `catalyst-quality` are handled explicitly; one without a handler marks the fields it can reach `Partial` with `UnhandledKind`. Catalyst quality scales the matching mods' values, as the game does. Adapted lines (`case-folded`, `split-section`) compute like any resolved line; their `adapted` flag is carried into provenance so the UI can mark them.
12. **Application task, jobs, and history wait for Phase 4.** The ACTION-PLAN puts the application slice (async recalculation, item edit, save) in Phase 4, so IMPLEMENTATION-PLAN Phase 3 WP1 (single-writer task, command/query/event contracts), WP2 (command history, snapshots, autosave hooks), and WP6 (background jobs, stale-result rejection) move there. Phase 3 delivers what they consume: the pure engine, the snapshot input, the cancellation hook, and the result document (Q4).
13. **No PoE1 rules in shared calc code (ADR-001).** The modifier store, provenance graph, completeness, result document, and math are game-neutral. The stat-to-effect mapping, archetype flags, PoE1 formulas (for example the resistance cap, leech, spell suppression), and PoE1 config inputs live in `loom_calc::poe1`. Gate 3c's identifier scan (Phase 2) already covers `loom-calc` (`CORE_CRATES` in `crates/loom-parser/tests/no_poe1_identifiers.rs`); Phase 3 adds the calc's PoE1-only names (for example `Bandit`-style choices, PoE1 config keys, PoE1 formula names) to its `POE1_ONLY` list as they appear. A PoE2 build gets a typed `Unsupported` result, never a PoE1 calculation (PoE2 calculations start after Phase 4, ADR-001).
14. **Errors and states (ADR-014).** Failures that stop a calculation have `CALC_*` codes (no main skill, data version mismatch, missing data row, cancelled). Per-field states: `Complete`, `Partial` (reasons), `Unsupported` (outside the engine, for example damage over time in this phase), `Indeterminate` (the inputs contradict each other or the data cannot settle a value, for example a gem the bundle does not have). The build-level state is the worst of its slice fields.

## Steps

### 1. Bundle v2: calc tables (L, split into three PRs by table family)
Each sub-PR adds whole sections, end to end: canonical types and loader in `loom-data`, validator checks with a `BUNDLE_*` code and a mutation case each, synthetic PoE1 and PoE2 rows (PoE2 without PoE1 sections), the extractor tables it lacks, and the `bundle` converter for them. v1 still loads for import; the v2 manifest comes with 1a.
- **1a, skills:** v2 manifest; active skills and their types, granted effects, per-level and per-quality stats (stat sets; the extractor adds the per-level stat-set table), skill gems and gem tags. The survey (step 3) needs these first.
- **1b, character and passives:** characters (base stats per class, per-level growth), ascendancies, passives (node stats as stat IDs and values, keystones, notables, masteries, ascendancy membership, jewel sockets), the passive graph with its tree version, mastery effects.
- **1c, item properties and monsters:** weapon, armour, and shield properties of base items, default monster stats (the extractor adds these tables).
- **Accept (each):** synthetic v2 bundles load and validate in CI; the mutation set gives each new code; `bundle` is byte-identical over two runs per game; `bundle-check` exits 0 on the real PoE1 and PoE2 v2 bundles; scrubbed summaries committed; wasm32 build; `cargo deny`.
- **Depends on:** nothing; 1b and 1c follow 1a (manifest).

### 2. Data parity against the oracle's data (M; after 1c)
`loom-parity-cli data-parity <bundle-dir> <pob-checkout>` compares, for every row the slice reads, the bundle's values with PoB v2.67.2's `src/Data` and the tree PoB loads for each tree version the fixtures use (`local/pob-v2.67.2` and the oracle checkout; read only). PoB's data is Lua; the tool reads it with the oracle runtime through a small adapter script, not with a Lua parser of our own.
- Every difference gets a category: `patch-delta` (with the patch, evidence per `docs/parity/README.md`), `pob-derivation` (PoB computes the value instead of reading it; the rule cited), `representation` (same game value, different form), or `uncategorized`.
- PoB's data is read by a separate script under `tools/oracle/data/`, outside the oracle adapter and its protocol (protocol 3 is unchanged), run on the verified oracle runtime.
- `pob-derivation` and `representation` need a cited PoB source line or game-data row per difference, like `patch-delta` needs a patch note; the report lists the count per category.
- The tool writes the overlay file `calc-run --data-overlay` reads (decision 5).
- **Accept:** a scrubbed report committed (`docs/phase-3/data-parity-poe1-<build>.json`); `uncategorized` 0, and every other category with its evidence; tests on synthetic rows for each category and for the overlay.
- **Depends on:** 1.
- **Done in P3-2** (context file, "Decisions in P3-2"): the command takes the export `tools/oracle/data/pob_data.lua` writes instead of a checkout, and a fifth category `not-carried` marks game columns the bundle does not carry (kept out of the overlay).

### 3. Scope survey and slice definition (S-M)
`loom-parity-cli calc-survey <builds...> --bundle <dir> --out <dir> [--anonymous]` applies decision 3 to every public fixture and corpus build, and writes per build: the main skill and its types, the archetype verdict with the rule that decided it, and the mechanic inventory the slice fields would need (stat IDs by source, typed kinds, unresolved and excluded lines, supports, config inputs, buffs). It also counts, across builds, which stats and mechanics are reached most.
- Commits the excluded-mechanics list (decision 3), each entry with the game-data rule that detects it and a test.
- Picks the two skills (Q2): the self-cast spell and the non-triggered attack with the most in-scope builds across corpus and public fixtures, ties broken by the smaller inventory. Adds credited public builds of a chosen skill if it has fewer than 3 in-scope builds (decision 3). The plan is updated with the choice and the in-scope list, and the slice field list (decision 4) is committed with its comparison rules and breakdown term groupings.
- **Accept:** the classifier's rules and each excluded-mechanic rule have a test on synthetic builds; the survey summary is committed (scrubbed for the corpus); the chosen skills, the in-scope builds (corpus by SHA-256; at least 3 per skill), the excluded-mechanics list, and `docs/phase-3/slice-fields.json` are in the PR; the maintainer confirms the choice and the exclusions (Q2, Q7) before any engine PR (P3-6).
- **Depends on:** 1a, 1b (skills, passives and tree version; item jewel kinds come from v1 base items).
- **Done in P3-3** (PR #66; `docs/phase-3/calc-survey.md`; context file, "Decisions in P3-3"): `calc-survey` takes a committed scope file (`docs/phase-3/calc-scope-poe1.json`: tree version `3_29` with evidence, the proposed excluded mechanics, the chosen skills, the minimum) and the data-parity rules (for `not-carried` passives). Result: 1 of 50 builds in scope (Winter Orb), no attack, so the minimum of 3 per skill is not met; the skill choice (Q2), the exclusions (Q7), and how to reach 3 builds (Q8) wait for the maintainer, and the top-up with public builds is a follow-up PR (P3-3b) once the skills are chosen. `slice-fields.json` is committed (revision 1, proposed): `AverageDamage` replaces `AverageHit`, `HitSpeed` added.
- **Done in P3-3b** (PR #67; `docs/phase-3/calc-survey.md`, "P3-3b"): the maintainer chose (2026-11-11) **spell Ice Nova (`IceNova`) and attack Smite (`Smite`)**, confirmed the three exclusions, and accepted derived setups of a guide as builds (Q8 (b), minimum 3). Seven credited derived fixtures (Maxroll guides by aer0 and Shayd): Ice Nova 3 in scope, Smite 4; the scope file is `confirmed`.

### 4. Engine core (M-L)
In `loom-calc`, game-neutral:
- `CalcInput`, `CalcData` (trait), calculation options; the `DataVersion` check;
- the modifier store: effects keyed by quantity, operation, and conditions (flags such as `Spell`, `Attack`, `Hit`, damage types; build conditions; config conditions), with sum, product, and flag queries, each recording its provenance;
- the provenance graph and its evaluation; field states and `UnsupportedReason`; `CALC_*` errors; the cancellation hook; `loom_calc::math`;
- `CalcResult`: slice fields with value, state, reasons, and graph.
- **Accept:** unit tests for each query and condition; property tests: every output equals its graph's evaluation, results are byte-identical across two runs, effect order in the store does not change results; a stat with no mapping gives `Partial` with `UnmappedStat`, never a silent zero; the wasm32 build; no new dependency outside the pure set; the PoE1-identifier scan (which already covers `loom-calc`) gets the calc's PoE1-only names.
- **Depends on:** nothing (synthetic data); it uses step 1's types once they exist.
- **Done in P3-4** (branch `feat/p3-4-engine-core`; context file, "Decisions in P3-4"): the game-neutral core (`input`, `quantity`, `condition`, `store`, `graph`, `state`, `ingest`, `engine`, `math`, `result`) and `poe1` with the 24-field slice, an empty mapping, and no phases, so every PoE1 field is `not-computed` until P3-6; 57 tests including three fixed-seed property tests; `Calc::set_derived` carries dependencies' reasons.

### 5. Parity harness (M)
- `calc-run <builds...> --bundle <dir> --out <dir> [--anonymous] [--data-overlay <file>]`: import, resolve, calculate, and write each build's result as the calc result document v1 (step 13) plus a scrubbed summary (states and reasons by count, and calculation time per build as a baseline for ADR-013's performance budgets in Phase 4; not gated).
- The fixture manifest gets the `mechanic` tier: a `Tier::Mechanic` variant in `tools/parity-cli/src/oracle_fixtures.rs` (the manifest denies unknown fields), its count tests, and the CI `oracle` job running it.
- `calc-parity <oracle-run-dir> <calc-run-dir> --slice docs/phase-3/slice-fields.json [--register <file>] --out <dir>`: maps our fields to PoB's keys, compares `Complete` fields with their comparison rule, parses the ten oracle breakdowns into terms and compares them with our graph's terms, lists `Partial`, `Unsupported`, and not-computed fields, and classifies differences with the register. Exit codes as `oracle-compare` (0, 19, 20, failure classes); documented in AGENTS.md.
- **Accept:** tests with scripted oracle and successor documents for each outcome (match, tolerance, registered, unregistered, partial not compared, breakdown term mismatch at the displayed decimals, a side failed, a `data-version` entry refused because the overlay run differs); the PoB breakdown-line parser tested on every line shape the 30 public fixtures produce for the ten breakdowns; a manifest test for the `mechanic` tier.
- **Depends on:** 3 (slice fields), 4, 13 (result document).

### 6. Character, attributes, life, mana, energy shield (M)
PoE1 base stats by class and level; attributes and their implicit effects (strength to life, intelligence to mana and energy shield, dexterity to evasion and accuracy); life, mana, and energy shield from bases, passives, items (with local armour-piece rules), and gems; reservation of the main aura set when it is a plain percentage of mana or life (anything else is `Partial`).
- **Accept:** mechanic fixtures (a bare level 1 and level 90 character of each class; one per stat source) match the oracle on the fields of this step; their breakdowns match term by term; stat mappings added with a test each.
- **Depends on:** 1b, 4, 5.

### 7. Core defences (M)
Armour, evasion (with the armour and evasion items' local properties and quality), resistances (with maximum resistances, the PoE1 act penalty as the oracle's config sets it), block and spell block, spell suppression.
- **Accept:** as step 6, for these fields.
- **Depends on:** 1c, 6.

### 8. Stat-to-effect mapping for the in-scope builds (L, several PRs)
Map every stat that step 3's inventory finds in the in-scope builds' passives, items, jewels, and gems for the slice fields, most frequent first. Each mapping is a data row (or code where the effect needs logic) in `loom_calc::poe1`, with a test and, where it is new behaviour, a mechanic fixture. Keystones and conditional stats ("while ...", "if ...") map to conditions; a condition the engine cannot settle from config makes the field `Partial` with `UnmodelledCondition`.
- **Accept (each PR):** the inventory's unmapped count for the in-scope builds falls; no in-scope build gains an unregistered difference; the coverage summary is committed.
- **Depends on:** 3, 4; runs alongside 6, 7, 9, 10.

### 9. Skills and spell hits (L, split into 9a and 9b)
- **9a, skill model (M):** active gem, supports in its group (links), gem level and quality, the per-level and per-quality stats, skill types, the skill's own effects, damage effectiveness, cast or attack time, base crit chance; support gems the engine does not model make the skill's fields `Partial`. Accept: unit tests over synthetic skills; mechanic fixtures for gem level and quality on a field the step can already compute (for example a gem's mana cost or a support's reservation multiplier, if in the slice) or, failing that, engine tests against values read from the oracle output.
- **9b, spell hits (M):** base damage per level, added damage scaled by effectiveness, conversion and gain as the chosen spell needs, increased and more modifiers by damage type and flags, crit chance and multiplier, cast speed, average hit, and hit DPS against the enemy (step 11). Accept: mechanic fixtures (the chosen spell alone; with one support each; with added damage; with crit) match; the chosen spell's in-scope builds match on the hit fields, or each difference is registered or `Partial` with its reason.
- **Depends on:** 3 (Q2 confirmed), 4, 6, 11; 9b on 9a.

### 10. Attacks (L)
The chosen attack, Smite, has two PoB skill parts (Melee Hit, Area Hit): the harness (P3-5) and this step follow PoB's part selection (`skillPart`, default 1) and say which part each field compares.
Weapon damage from the base and local mods, attack speed, accuracy and hit chance, attack crit, one-hand, two-hand, and dual-wield rules the chosen attack's builds use (anything else `Partial`), the attack skill's per-level stats and supports, average hit and hit DPS.
- **Accept:** as step 9b, for the chosen attack.
- **Depends on:** 3 (Q2 confirmed), 4, 6, 9a, 11.

### 11. Enemy model (M)
The enemy the oracle's config describes (level, resistances, armour, evasion, boss flags, the default when the build sets nothing), penetration, exposure, and damage-taken modifiers the in-scope builds use.
- **Accept:** mechanic fixtures for each config value used by the in-scope builds; hit DPS fixtures match.
- **Depends on:** 1c, 4, 6.

### 12. Buffs and configuration for the in-scope builds (L, several PRs)
What step 3's inventory finds in the in-scope builds, in order of reach: charges, auras and heralds with their reservation, curses, flasks and their config, conditions set by config (`Config` inputs). Each config input is either modelled, listed as not affecting the slice (with a reason and a test), or makes the fields it can reach `Partial`.
- **Accept (each PR):** mechanic fixtures for what it adds; the in-scope builds' `Partial` counts fall; no unregistered difference.
- **Depends on:** 6 to 11 as each item needs.

### 13. Calc result document v1 (M; done right after step 4, before step 5)
`loom_serialization::dto::calc::v1`: the result with fields, states, reasons, and provenance; deterministic JSON; JSON Schema and TypeScript generated as for the build document; CI freshness and cross-platform byte comparison (as `dto cross-platform`). Every parity report from step 5 on uses this versioned document; a later shape change is a version bump with a migration (Phase 2 decision 14).
- **Accept:** golden bytes for synthetic results; a second run equal; Windows equal to Linux in CI; schema freshness; the schema validates the produced JSON.
- **Depends on:** 4.

### 14. Corpus run and register (M)
The maintainer runs `oracle-run`, `calc-run`, and `calc-parity` on the public fixtures and the 21-build corpus; every difference on a `Complete` field is fixed or registered with evidence and a test; scrubbed summaries committed. If Q1 is yes, a CI job runs `calc-run` and `calc-parity` on the public and mechanic fixtures against the `oracle` job's outputs and is added to `ci-ok` after the 10-green-run rule Spike C used.
- **Depends on:** 9, 10, 11, 12, 13.

### 15. Report and gate verdict (S-M)
`docs/phase-3/phase-3-report.md`: what was built, inputs, the verdict per gate item with its commands, coverage of out-of-scope builds (states and reasons by count), known limitations. Update the handoff, CLAUDE.md, the ACTION-PLAN; move the task folder to `dev/completed/`.
- **Depends on:** 14.

## Gate (objective, checkable)
All of the following hold on the recorded inputs (bundle Steam build, oracle pin, fixture and corpus hashes, slice-field list revision):
1. **In-scope builds match.** For every in-scope build (decision 3) among the public fixtures and the corpus, every slice field is `Complete`, and `calc-parity` exits 0 or 19: each field equals the oracle under its comparison, or the difference has an accepted register entry with evidence and a test. A `data-version` entry counts only when the `--data-overlay` run matches the oracle on that field (decision 5). There are at least 3 in-scope builds per chosen skill (decision 3); with fewer, the gate is not met.
2. **Mechanic fixtures match.** Every mechanic fixture (tier `mechanic`) exits 0 or 19 in `calc-parity` on every slice field it targets.
3. **Breakdowns match.** For in-scope builds and mechanic fixtures, the ten oracle breakdowns' terms equal our graph's terms at each line's displayed decimals, grouped as `slice-fields.json` writes (or are registered).
4. **No silent results anywhere.** For every build that imports (in scope or not), each slice field has a state; no field is `Complete` while an input that can reach it is unmodelled (mutation test in CI: inject an unmapped stat, an unresolved line, a typed `kind` with no handler, an unknown config input, an unsupported support gem; each makes the reachable fields `Partial`; a tree version other than the bundle's gives `Indeterminate`); and no `Complete` field of an out-of-scope build has an unregistered difference.
5. **Divergence register in use.** `divergence-check` passes; every accepted entry names its test, which passes in CI; no entry covers nothing on the latest run (`unused_entries` empty, or the entry retired).
6. **Determinism and hygiene.** Calc result documents are byte-identical across two runs and across Windows and Linux (CI); every core crate still builds for wasm32; the PoE1-identifier scan lists the calc's PoE1-only names; schema freshness and `cargo deny` are green; `data-parity` has `uncategorized` 0 and evidence for every other difference.

The report states that the slice fields do not stand for whole-build DPS: `TotalDPS` here is hit DPS of the main skill, and for some builds most damage is elsewhere (`boneshatter-juggernaut`: `TotalDPS` about 10% of `CombinedDPS` in the oracle).

Items 1 and 3 on the corpus, and on public fixtures while Q1 is "no", are maintainer-run with committed summaries (the Phase 2 model); items 2 (if Q1 is yes), 4, 5, and 6 run in CI.

## Fallback
- **An in-scope build cannot reach `Complete` in this phase** (a mechanic too large for the slice): with the maintainer's approval and a recorded reason, either the excluded-mechanics list grows (recorded in the plan and report, the build leaves the denominator, the gate still needs 3 builds per skill, topped up with credited public builds if needed), or the chosen skill changes to the next one from the survey.
- **A stat or line cannot be expressed as a stat-ID effect** (ADR-015): the hybrid fallback, per area, by an ADR-015 amendment the maintainer approves; until then the field stays `Partial`.
- **PoB and the game disagree and the evidence is unclear:** the field stays unregistered and the build fails item 1; the open case goes to the maintainer with what was found. No tolerance is widened to make it pass.
- **Cross-platform bits differ:** the operation moves into `loom_calc::math` with a deterministic implementation (for example `libm`, MIT, pure Rust), checked with `cargo deny`.
- **The real data cannot be in CI (Q1 no):** the gate's real-data items stay maintainer-run with summaries; CI keeps the synthetic and mutation tests.

## Out of scope (named so the gap is visible)
- Damage over time, ailments, minions, totems, traps, mines, brands, triggers, mirages, and `CombinedDPS` beyond hit DPS (`Unsupported` in this phase).
- Cluster jewels, timeless jewels, and anything the survey finds only outside the in-scope builds (`Partial` with a reason).
- Old-tree builds (PoB converts them; out of scope by decision 3).
- The application task, command history, jobs, autosave, and UI (Phase 4, decision 12).
- PoE2 calculations (after Phase 4, ADR-001); PoE2 builds get `Unsupported`.
- Moving the oracle pin (Q5), signed bundles and distribution (Phase 5).

## Risks and mitigations
| Risk | Mitigation |
|---|---|
| Endgame builds carry far more mechanics than a slice | The gate counts only in-scope builds of two skills; step 3 picks skills by reach and inventory size; `Partial` with reasons is an honest result, and the fallback narrows only with the maintainer |
| The engine shrinks its gate by marking content unsupported | Archetype classification is independent of the engine (decision 3); gate 1 requires every slice field `Complete` on in-scope builds; gate 4 forbids `Complete` with unmodelled inputs |
| Data differences look like calc bugs, or `data-version` entries hide them | Step 2 compares data first; a `data-version` entry is accepted only when the overlay run with PoB's values matches (decision 5) |
| Timeless and cluster jewels sit in most community builds | The excluded-mechanics list is fixed in P3-3 from game data, before the engine exists; minimum 3 in-scope builds per skill, topped up with credited public builds |
| Mapping thousands of stats by hand | Only stats the in-scope builds reach, most frequent first (step 8); data rows where the effect is regular; each with a test |
| PoB's rounding and order of operations differ from a clean formula | Rounding points written per field with tests (decision 7); semantic breakdown comparison shows where terms diverge |
| `f64` differs across platforms or in wasm32 | IEEE-exact operations only on output paths; one audited math module; cross-platform byte comparison in CI |
| Real data is not in CI, so parity regressions show only in maintainer runs | Q1 offers a pruned derived bundle (ADR-000 allows derived data); until then, mechanic fixtures run in every engine PR's maintainer evidence |
| PoB's display breakdown strings are not a contract | The parser is tested on every line shape the public fixtures produce; a shape it cannot read is an `ORACLE_NORMALIZATION`-class failure, never a pass |
| Unresolved unique and custom item lines block in-scope builds | They make fields `Partial` with the line named; step 8 includes resolver fixes in `loom-parser` where the survey finds them, keeping Spike B's fields unchanged or recording the change |
| Scope creep into Phase 4 | Decision 12: only the pure engine, the snapshot, the hook, and the result document |

## Planned PR breakdown
Each PR is one session, small and single-purpose, and fills in the evidence sections of the PR template. "Maintainer-run" evidence is pasted as scrubbed summaries (corpus) or full reports (public fixtures).

| PR | Content | Evidence |
|---|---|---|
| P3-0 | This plan (docs only) | None for tests (docs only); plan-reviewer summary in the context file |
| P3-1a | Step 1a, bundle v2 manifest and skill tables | Loader and mutation tests on synthetic v2 bundles; two-run determinism and `bundle-check` on real bundles (maintainer-run, summaries); wasm32; `cargo deny` |
| P3-1b | Step 1b, character and passive tables | Same |
| P3-1c | Step 1c, item property and monster tables | Same |
| P3-2 | Step 2, `data-parity` and the overlay file | Category and overlay tests; scrubbed report with `uncategorized` 0 and evidence per category (maintainer-run) |
| P3-3 | Step 3, `calc-survey`, exclusions, skill choice, slice fields | Classifier and exclusion tests; survey summary; `slice-fields.json` |
| P3-3b | Step 3, after the maintainer's Q2/Q7/Q8: the chosen skills in the scope file, credited public builds until each has 3 in scope | Survey summary with `pass` true |
| P3-4 | Step 4, engine core | Unit and property tests; wasm32; identifier list |
| P3-13 | Step 13, calc result document v1 | Golden bytes; cross-platform job; schema freshness |
| P3-5 | Step 5, `calc-run`, `calc-parity`, `mechanic` tier | Scripted-document tests per outcome; breakdown parser tests on the public fixtures |
| P3-6 | Step 6, character, attributes, life, mana, ES | Mechanic fixtures with `calc-parity` reports |
| P3-7 | Step 7, core defences | Same |
| P3-8a… | Step 8, stat mappings (as many PRs as the inventory needs) | Mapping tests; coverage summary; no new unregistered difference |
| P3-9a | Step 9a, skill model | Unit tests; mechanic fixtures where a slice field shows the effect |
| P3-9b | Step 9b, spell hits | Mechanic fixtures; the spell's in-scope builds (maintainer-run) |
| P3-10 | Step 10, attacks | Same, for the attack |
| P3-11 | Step 11, enemy model | Mechanic fixtures |
| P3-12a… | Step 12, buffs and config (several PRs) | Mechanic fixtures; `Partial` counts |
| P3-14 | Step 14, corpus run, register entries, CI job if Q1 | `calc-parity` on public fixtures and corpus; register check |
| P3-15 | Step 15, report and gate | Gate commands and their outputs |

Order (table order): P3-1a, P3-1b, P3-1c, P3-2, P3-3, P3-4, P3-13, P3-5, then 6, 7, 11, 9a, 9b, 10 with 8 and 12 interleaved as the inventory needs, then 14, 15. P3-4 does not depend on step 1 and may come earlier if the maintainer prefers. Engine PRs (P3-6 on) wait for the maintainer's confirmation of the skills and exclusions (Q2, Q7).

**Parity and divergence.** From P3-6 on, every engine PR carries a `calc-parity` report on its mechanic fixtures and, once step 9 or 10 lands, on the in-scope builds. Every difference is fixed, registered with evidence and a test, or the field is `Partial` with its reason.
