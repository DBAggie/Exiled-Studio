Last Updated: 2026-11-11 (P3-13: calc result document v1)

# Phase 3: calc slice (context)

## Key files
| Path | Why it matters |
|---|---|
| `docs/plan/ACTION-PLAN.md` (Phase 3) | Scope and gate |
| `docs/plan/IMPLEMENTATION-PLAN.md` (Phase 3) | Work packages; WP1, WP2, WP6 move to Phase 4 (decision 12) |
| `docs/phase-2/phase-2-report.md` | What Phase 2 built and its limitations |
| `dev/completed/phase-2-canonical-model/` | Phase 2 decisions (bundle, domain, DTO, `ItemData`) the calc builds on |
| `crates/loom-calc/src/lib.rs` | Empty crate; the engine goes here |
| `crates/loom-domain/src/` | `Build`, `Character`, `PassiveSpec`, `SocketGroup`, `Gem`, `Item`, `LineResolution`, `Config`, `Completeness` |
| `crates/loom-data/src/bundle/` | Bundle v1 (`mod.rs`, `poe1.rs`, `source.rs`, `validate.rs`); v2 extends it |
| `crates/loom-parser/src/import/` | `ItemData`, `resolve_build`: the pattern `CalcData` follows |
| `crates/loom-parser/tests/no_poe1_identifiers.rs` | Gate 3c scan; extend to `loom-calc` |
| `tools/parity-cli/src/oracle_compare.rs`, `divergence.rs` | Register classification and exit codes `calc-parity` reuses |
| `tools/parity-cli/src/bundle.rs` | The extract-to-bundle converter (v2 in step 1b) |
| `tools/extractor/` | Tables to add: per-level stat sets, weapon, armour, shield types, default monster stats |
| `tools/oracle/fixtures/fixtures.json` | Fixture manifest; mechanic fixtures join it (tier `mechanic`) |
| `tools/oracle/adapter/oracle.lua` | Oracle output: `main_output` and ten breakdowns |
| `docs/parity/README.md`, `divergence-register.json` | Register rules; empty until this phase |
| `local/pob-v2.67.2` | `git archive` of PoB's `src/Data` at the oracle tag (Spike B) |
| `tools/oracle/data/pob_data.lua`, `tools/parity-cli/src/data_parity*` | P3-2: PoB data export and `data-parity` |
| `docs/phase-3/data-parity-rules-poe1.json` | The categorization rules with evidence (per Steam build) |
| `local/p3/p3-2/` | P3-2's export, runs, and overlay (local) |
| `local/determinism/poe1/run1`, `local/determinism/poe2/run1` | The Spike A/B extracts (Steam builds 26530219, 26476662) |

## ADRs this plan touches
- **ADR-001:** the game is the truth, PoB v2.67.2 the oracle; PoE2 calculations after Phase 4 (decision 13).
- **ADR-003:** pure engine over immutable snapshots, `f64` with canonicalization, provenance graph as API data, per-field tolerance metadata (decisions 7, 8, 4).
- **ADR-004:** single-writer task and jobs; Phase 4 (decision 12).
- **ADR-006:** layered comparison, register, no broad close-enough rule (decision 9).
- **ADR-007:** corpus local, scrubbed summaries; community fixtures named only through the manifest; every discrepancy adds a minimized fixture (decision 10).
- **ADR-013:** first milestone: one self-cast spell, one non-triggered attack, no minions, core defences (decision 3).
- **ADR-014:** complete, partial, indeterminate, fatal; stable codes (decision 14).
- **ADR-015 (and Amendment 1):** stat-ID first; calculations handle each typed `kind` explicitly; hybrid fallback per area by amendment (decisions 2, 11).
- **ADR-016:** core is wasm32, no threads, clocks, I/O (decision 7).
- **ADR-000:** derived data may be shipped (Q1).
No ADR needs an amendment for this plan.

## Decisions (plan, 2026-11-11; numbers match the plan's "Design constraints and decisions")
1. Compute only the slice; anything unmodelled that can reach a field makes it `Partial` with a reason from a closed list.
2. Stat IDs in, our own quantity names out; PoB's `main_output` names only in the parity tool's mapping.
3. "In scope" is decided by archetype rules from game data (`calc-survey`), not by the engine, so the engine cannot shrink its gate.
4. Slice fields are a committed list with per-field comparison rules (`docs/phase-3/slice-fields.json`).
5. Data versions are told apart before calc comparison (`data-parity`); the oracle pin stays v2.67.2.
6. Bundle v2 adds the calc tables; real data stays local unless Q1 says otherwise.
7. Pure, deterministic engine; IEEE-exact operations on output paths; audited `math` module; cancellation hook.
8. The provenance graph is the breakdown; every output equals its graph's evaluation.
9. Layered comparison: per-field rules on `Complete` fields, semantic breakdown terms, completeness listed.
10. Mechanic fixtures (our own small PoB builds) grow with the engine, run by CI's oracle job.
11. Each typed `kind` has a handler or makes fields `Partial` with `UnhandledKind`; adapted lines carry their flag into provenance.
12. Application task, history, and jobs wait for Phase 4 (ACTION-PLAN wins over IMPLEMENTATION-PLAN).
13. PoE1 rules only in `loom_calc::poe1`; PoE2 builds get `Unsupported`.
14. `CALC_*` codes; per-field states `Complete`, `Partial`, `Unsupported`, `Indeterminate`.

## Open questions for the maintainer (the plan proceeds on the default)
- **Q1** Commit a pruned, derived real PoE1 bundle (the rows the public and mechanic fixtures reach) so CI runs calc parity against the `oracle` job's outputs? ADR-000 allows shipping derived data; PoB commits the same kind of data. Default: **no for now** (Phase 2 Q1 stands); calc parity is maintainer-run with committed reports. The plan recommends **yes** before P3-6, so engine regressions show in CI; it needs the maintainer.
- **Q2** Which spell and which attack? Default: chosen by `calc-survey` (P3-3): the most in-scope builds across corpus and public fixtures, ties broken by the smaller mechanic inventory. The maintainer confirms before P3-6. Candidates seen in the public fixtures: attacks Boneshatter (`boneshatter-juggernaut`), Flicker Strike, Earthshatter (`facebreaker-luminary`), Elemental Hit of the Spectrum, Lacerate; spells Shock Nova of Procession (`shock-nova-archmage-hierophant`, Archmage). Winter Orb and Cyclone are channelled, Penance Brand is a brand, the rest are minions, totems, traps, mines, or damage over time.
- **Q3** The slice field list (plan decision 4). Default: the list there.
- **Q4** Application task, history, and jobs in Phase 4, not Phase 3. Default: yes (ACTION-PLAN order).
- **Q5** Keep the oracle at PoB v2.67.2 and register data differences as `data-version`. Default: yes; moving the pin needs ADR-001's process.
- **Q6** Default comparison per field: `exact` for values PoB rounds (life, mana, resistances, attributes), `rel` with a written reason (summation order) only where the survey shows non-rounded floats differ in the last bits. Default: yes, each written in `slice-fields.json`.
- **Q7** The excluded-mechanics list (plan decision 3), written in P3-3 from game data. Default: what P3-3 proposes, at least timeless and cluster jewels in allocated sockets if the survey shows they reach slice fields; the maintainer confirms before P3-6. P3-3 proposes `timeless-jewel`, `cluster-jewel`, `not-carried-passive` (`docs/phase-3/calc-scope-poe1.json`).
- **Q8** (P3-3) How to reach 3 in-scope builds per chosen skill: the survey finds 1 (Winter Orb) and no attack. Options in `docs/phase-3/calc-survey.md`: (a) credited public builds of the chosen skills on the 3_29 tree without the excluded jewels (recommended), (b) derived fixtures with a guide's earlier 3_29 spec active, (c) a lower minimum. No default: P3-3b waits for the answer and for Q2.

## plan-reviewer (2026-11-11) summary
Verdict: proceed, with changes. No ADR contradicted; decision 12 (WP1, WP2, WP6 to Phase 4) endorsed; Q1 "yes before P3-6" called sound. All findings applied:
- **B1** in-scope rules contradicted the out-of-scope list (15 of 21 community fixtures carry `timeless-jewel`, Boneshatter's among them), so gate 1 was reachable only through the fallback: decision 3 now has a committed excluded-mechanics list from game data (P3-3, maintainer-approved, Q7) and a minimum of 3 in-scope builds per skill, topped up with credited public builds.
- **B2** `data-version` entries could hide calc bugs: decision 5 adds `calc-run --data-overlay` (PoB's values for the rows `data-parity` found different, tool-side only); a `data-version` entry counts only when the overlay run matches (gate 1).
- **M1** tree versions: in-scope rule "tree version equals the bundle's"; otherwise `Indeterminate`, `CALC_TREE_VERSION`.
- **M2** breakdown precision: terms compared at each line's displayed decimals; grouping written per field in `slice-fields.json`.
- **M3** P3-1a too large: step 1 split by table family (1a skills with the v2 manifest, 1b character and passives, 1c item properties and monsters), each end to end.
- **M4** result document too late: P3-13 moved right after P3-4, before P3-5.
- **M5** `mechanic` tier needs code (`Tier` in `oracle_fixtures.rs`, `deny_unknown_fields`, count tests): owned by P3-5.
- **M6** P3-9 split into 9a (skill model) and 9b (spell hits); P3-10 depends on 9a.
- **N1** the identifier scan already covers `loom-calc` (`CORE_CRATES`); wording fixed to "add calc names to `POE1_ONLY`".
- **N2** `data-parity`'s Lua script lives outside the oracle adapter protocol (`tools/oracle/data/`).
- **N3** `pob-derivation` and `representation` need cited evidence; counts per category reported.
- **N4** gate 4's mutation set adds an unhandled `kind` and a mismatched tree version.
- **N5** `calc-run` records calculation time per build (baseline, not gated).
- **N6** the report states that slice `TotalDPS` is not whole-build DPS.

## Facts found while planning (2026-11-11)
- A community build's oracle output has 706 `main_output` fields; the oracle's ten breakdowns are display strings: `["2074 ^8(base)", "x 2.84 ^8(increased/reduced)", "= 5890"]` for `Life`, `["582276.9 ^8(average damage)", "x 1.82 ^8(attack rate)", "= 1057055.3"]` for `TotalDPS` (`boneshatter-juggernaut`, `local/oracle-run/c3b-community-1`). `breakdown_source` is `calcsTab.calcsEnv.player.breakdown`.
- `boneshatter-juggernaut` in the oracle: `Life` 5890, `Mana` 663, `Armour` 33724, `FireResist` 84, `TotalDPS` 1,057,055.25, `CombinedDPS` 10,548,785.48 (so most of its damage is outside the hit slice), `Speed` 1.815.
- The bundle has no calc tables; the extract has most of them (Current state in the plan). Per-level stat sets (`GrantedEffectStatSetsPerLevel`), `WeaponTypes`, `ArmourTypes`, `ShieldTypes`, and default monster stats are not in `local/determinism/poe1/run1/tables`.
- `oracle-compare` treats a field absent on one side as a difference, so comparing a slice needs a completeness-aware mode (`calc-parity`), not the current tool as is.
- The PR stack: PR #42 (`feat/p2-1-domain-model`) holds all of Phase 2 (#43 to #58 squash-merged into it 2026-11-11); its merge to `main` is blocked by an external DNS problem (maintainer, 2026-11-11). P3-0 branches from it and targets it; CI is not checked until the maintainer says so.
- Merged 2026-11-11: the whole stack, #42 into `main` (`7c122b9`). From then on PRs branch from and target `dev` (permanent; the maintainer merges it into `main`).

## Decisions in P3-1a (bundle format 2, skill tables; evidence `docs/phase-3/bundle-v2-skills.md`)
- **Format 2 = format 1 + whole sections.** `loom_data::bundle::skills`: `data/skill_types.json`, `cost_types.json`, `active_skills.json`, `granted_effects.json` (per-level rows and quality stats inside), `stat_sets.json` (per-level values inside), `gems.json` (keyed by base item), `gem_effects.json`, `gem_tags.json`. All required in format 2; one in a format-1 bundle is `BUNDLE_FORMAT_FILE`. Format 1 still loads (`skills` is `None`). `data_schema` is 2 (each build's data version records it). P3-1b and P3-1c add required files to format 2, so local format-2 bundles are remade after each.
- **One set of types for both games.** Game differences stay in the converter (column names per game; PoE1 cost types per level, PoE2 per effect; PoE1 mana and life reservations, PoE2 `Reservation` as spirit, per PoB's PoE2 exporter) and in optional fields (`damage_effectiveness` PoE1, `damage_incremental_effectiveness` PoE2, `weapon_classes` PoE1).
- **Values as stored.** No PoB scaling in the bundle; the interpolation number is kept raw (1, 2, 3) until P3-9a gives it a meaning with evidence. `f32` fields are written as their shortest text and read back to the same bits (bit-pattern sweep test).
- **Validator:** every skill reference (`BUNDLE_DANGLING_REF`), unique and non-empty IDs per skill file (`BUNDLE_DUPLICATE_ID`, `BUNDLE_EMPTY_ID`), strictly ascending levels (`BUNDLE_SKILL_LEVELS`); a mutation case each.
- **Converter:** columns both games have are required, and one of each game's alternative sets must be complete, so a renamed column fails the run; the summary counts sorted level lists, unequal parallel lists, owner-less rows, dropped stats and gems. Real data: 0 except one sorted level list (PoE1 `IncaBossSummonAtziri`) and 4 PoE1 stat sets without levels.
- **Extractor:** `SKILL_TABLES` roots in both games (new tables `GrantedEffectStatSetsPerLevel`, `GrantedEffectQualityStats`, `EffectivenessCostConstants`); every earlier extract file byte-identical.
- **Not carried:** PoE1 Ruthless gem effects, level requirements, Vaal soul costs, PoE2 quality `AltStats`/`ApplyToStatSets`, PoE2 skill weapon requirements.
- **Open:** `cargo deny check` fails on RUSTSEC-2024-0370 (`proc-macro-error`, via Tauri/GTK) on the base branch too; not this PR's change, for the maintainer.

## code-architecture-reviewer (P3-1a, 2026-11-11) summary
No code blockers; no ADR or crate-boundary issue. Applied: required columns per game instead of silent zeros; dropped stats and gems counted, NaN sentinel removed; `cost_types.json` with a referential check; `ConstantStat` named rows; `BUNDLE_EMPTY_ID`; mutation cases for each support-type list and the cost type; every converter summary counter asserted; missing-column tests. Not applied: an interpolation enum (kept raw until P3-9a decides with evidence); `serde_json`'s `float_roundtrip` (the bit-pattern sweep passes; revisit if a value fails); the commit trailer remark (the trailer is the one the session was given). Format 2 changing in 1b/1c is stated above and in the handoff.

## Decisions in P3-1b (character and passive tables; evidence `docs/phase-3/bundle-v2-passives.md`)
- **Per-level growth is not in a table.** It is in the object template `Metadata/Characters/Character.ot` (`life_per_level` 12, `mana_per_level` 6, resistance caps, ...), the file PoB exports `data.characterConstants` from. The extractor now reads object templates with the item-type grammar (widened: `%`/`+` in keys, `Name { }`, one-line script values), only the character folder (and one level below) and `Metadata/Monsters/Monster.ot` (for P3-1c); manifest format 5 (`object_templates`); `loom-data` reads manifest formats 3 to 5. Item type output unchanged.
- **`loom_data::bundle::passives`**, required in format 2: `characters.json` (class number, base life/mana/attributes, own template), `ascendancies.json`, `object_templates.json` (`Stats` block: integer stats; other entries kept as written in `values`; `extends` unresolved, the calc applies the chain), `passives.json`, `mastery_groups.json`, `mastery_effects.json` (`hash` = `HASH16`), `passive_tree.json` (`Default` tree: groups with x/y, nodes with orbit and connections, roots, orbit slots).
- **Validator:** every reference (tree nodes are passives; connections and roots are tree nodes); `BUNDLE_TEMPLATE_CYCLE`; `BUNDLE_TREE_GEOMETRY` (orbit and position within the orbit slots); unique non-empty IDs; unique node numbers, tree nodes, roots, mastery hashes, class numbers, ascendancy numbers within a class.
- **Converter:** identifying and calc-driving integers (class numbers, base stats, node numbers, hashes) and the graph's lists are required, never 0 or empty.
- **For P3-3:** the tree's identity is the manifest's Steam build; the PoB tree version label is decided there with evidence. Connections are undirected and stored once. Orbit radii are not in the data read (needed only for distance rules). PoE2 attribute and weapon-set passive columns are not carried (PoE2 calc after Phase 4).
- **For P3-6:** PoE1 classes extend `Character.ot` through their empty class templates; PoE2 classes have no own template.

## code-architecture-reviewer (P3-1b, 2026-11-11) summary
No blockers. Applied: template cycle check; tree geometry check and required graph lists; required integers instead of 0; unique class and ascendancy numbers and roots; `StatAmount` rename; docs on tree identity, connection direction, orbit radii; tests for each, the three "kept as written" rules, and missing inputs; exact-code duplicate tests. Not applied: a chain resolver in `loom-data` (the calc owns the override rule, P3-6; the validator guarantees chains end); a check that each root is a class start (PoE1 roots include ascendancy starts; P3-3 decides with the data); `calc_files()` as a const array (cosmetic).

## Decisions in P3-1c (item property and monster tables; evidence `docs/phase-3/bundle-v2-items-monsters.md`)
- **`loom_data::bundle::properties`**, required in format 2: `weapon_types.json`, `armour_types.json` (defences as `ValueRange`; PoE2 `min` = `max`), `shield_types.json`, `monster_levels.json` (`DefaultMonsterStats`, the columns PoB reads), `game_constants.json`. The non-level monster defaults are the monster object template (P3-1b). Format 2 is complete: a later change to its sections is a new format.
- **Extractor:** `ITEM_PROPERTY_TABLES` roots (`WeaponTypes`, `ArmourTypes`, `ShieldTypes`, `DefaultMonsterStats`, `GameConstants`), both games; PoE2 also brings `WeaponClasses` (not carried).
- **Validator:** base item references; one row per base and per constant; `BUNDLE_MONSTER_LEVELS` (1, 2, 3, ... without gaps: the calc reads level n at row n - 1); `BUNDLE_VALUE_RANGE` (ranges of amounts, positive attack times and divisors, no negative chances, ranges, reload times, block); `BUNDLE_GAME_SHAPE` (PoE1: no reload time, `alt_life_1` and the conversion multiplier; PoE2: reload time, single-value defences, no PoE1 columns).
- **Monster armour:** PoB v2.67.2 does not read `DefaultMonsterStats.Armour` and derives monster armour from `MonsterDamageReductionImprovement`; the bundle keeps both; P3-11 decides with evidence (a `pob-derivation` candidate for P3-2's `data-parity`).
- **Against PoB:** every monster table, game constant, and monster constant equal (damage tables at `f32` precision); weapon and armour bases equal. The calc widens `f32` with `f64::from`.

## Decisions in P3-2 (data parity; evidence `docs/phase-3/data-parity.md`)
- **Two steps, not one command.** `tools/oracle/data/pob_data.lua` exports PoB's loaded data on the oracle runtime (maintainer-run, like the share-code script; outside the adapter protocol); `data-parity <bundle-dir> <pob-export> --rules <file> --out <dir>` compares. The plan's `<pob-checkout>` argument became the export, so the Rust side needs no runtime and the tests use a hand-written export. The export's SHA-256 and PoB version are recorded.
- **Compared in PoB's form.** Each bundle value goes through the conversion PoB's exporter made (cited per field in `compare::TRANSFORMS`), numbers at 14 significant digits (Lua's `tostring` wrote PoB's data files), then compared exactly. Absent equals 0 only where PoB writes a field only when not 0 (named at each comparison). Skill levels are found the way PoB pairs rows (by level, or the single row). Passive text: the bundle's stats rendered with `passive_skill_stat_descriptions`, stats without a description render nothing (as in game), "Grants N Passive Skill Points" compared as `skill_points`.
- **Categories:** the plan's `patch-delta`, `pob-derivation`, `representation`, plus **`not-carried`**: the game has the value in a column the bundle does not carry (`PassiveSkills.PassiveSkillBuffs`, `DescendancyKey`, `Character.ot` Pathfinding). Not a game-vs-PoB difference; a calc reaching it must be `Partial`. The overlay holds `patch-delta` and `pob-derivation` rows only. Rules need citations by category (validator): a patch-note thread for `patch-delta`, `v2.67.2:`/`pob@<commit>:`/`game:` otherwise, `game:` for `not-carried`. First matching rule wins; an unused rule fails the run.
- **Result on Steam build 26530219:** 534 differences in about 530,000 compared values, all categorized; the game build is 3.29.3 or later (Conflagration's level-1 fix, 3.29.3, 2026-08-12), PoB's data 3.29.2 or earlier (v2.67.2 is dated 2026-08-07). Most differences are PoB's own: hand-written skills and levels, `#addSkillTypes`, the exporter's stat-order shift (a new stat in a later level lands values under the wrong names), monster armour derived.
- **For the maintainer:** the Vaal transfigured gem tags (8) are classed `patch-delta` without a patch-note line (PoB copies `GemEffects.Tags` verbatim, so only the game can have changed them); confirm or reclassify. The PoB `dev` branch in the read-only legacy repo (`3dbca44`, 2026-10-29) is cited as corroboration (`pob@`).
- **For P3-3/P3-8:** 15 ascendancy nodes grant buffs or skills whose stats the bundle does not carry (`passives-buff-text`); bloodline nodes have no ascendancy link. A build allocating one gets `Partial`, or the bundle gains the buff tables (a format change).
- **For P3-11:** monster armour: PoB derives it (`monster-armour-derived`); the overlay carries PoB's table.

## Decisions in P3-3 (scope survey; evidence `docs/phase-3/calc-survey.md`)
- **A committed scope file** (`docs/phase-3/calc-scope-poe1.json`, `calc_survey::rules::Scope`): Steam build, `status` (`proposed` until the maintainer confirms), tree version `3_29` with evidence, `minimum_per_skill` 3, the chosen skills by granted effect ID (`null` until chosen), and which excluded mechanics apply (a closed list in code: `timeless-jewel`, `cluster-jewel`, `not-carried-passive`). The tool refuses a bundle of another Steam build.
- **Main skill as PoB finds it**, from the bundle: gems by `gemId` + `variantId` (or `skillId`); PoB's `GlobalEffect` mark is not game data, so every combination of `enableGlobal`-off effects is tried and a disagreement is `main-skill-uncertain`. Supports by PoB's type expressions, in group order.
- **Archetype rules** read the types after supports: `not-hit` (PoB's hit flag; auras and summons), `not-manually-cast`, `triggered`, `totem`, `trap`, `mine`, `brand`, `minion`, `mirage` (a support requiring exactly `MirageArcherCanUse`). Channelling is not excluded (the plan names no such rule; Q2 notes Winter Orb is channelled).
- **Tree version strict:** PoB computes each build with its own version's tree; 17 of 18 builds on 3_28 allocate nodes that changed in 3_29 (PoB's own tree files), so a per-node equality rule would not help.
- **Unknown gems:** PoB v2.67.2 skips gems it cannot find (`SkillsTab.lua`); the survey reports them (`indeterminate`) instead.
- **Slice fields** (`docs/phase-3/slice-fields.json`, revision 1, proposed): `AverageDamage` replaces `AverageHit` (no top-level `AverageHit` for attacks; the DPS line prints `AverageDamage`), `HitSpeed` added; all `exact`; the ten breakdowns with their PoB line formats and term groupings (`chain`, `slots`, `min-max-total`).
- **Result:** public 1 of 29 in scope, corpus 0 of 21; the gate's minimum is not met (Q8).
- **Review (code-architecture-reviewer):** no blockers. Applied: `--anonymous` reports keep aggregates only (ADR-007); `tree:unread-jewel` for an allocated socket whose jewel cannot be read (ADR-015); gems-only supports; a `confirmed` scope file names both skills. Not applied: re-checking supports after all types are added (PoB's second loop reconsiders only non-support entries; documented in `main_skill.rs`).

## Decisions in P3-3b (maintainer, 2026-11-11; evidence `docs/phase-3/calc-survey.md`, "P3-3b")
- **Q2:** spell `IceNova`, attack `Smite` (the skills that reached the minimum; preference for no PoB skill parts held for the spell, Smite has two). **Q7:** `timeless-jewel`, `cluster-jewel`, `not-carried-passive` confirmed. **Q8:** derived setups of a guide count as builds; minimum stays 3. Scope file `confirmed`.
- **Derived fixtures:** a guide's own setup made active (spec, skill set, item set, main group of the same name; for Smite Act 5 and 10 also the group's `mainActiveSkill` and `mainActiveSkillCalcs`, since the oracle's breakdowns come from PoB's CALCS view, which reads the latter). Tag `derived` in `fixtures.json`; the exact change and the Maxroll terms note in `community/SOURCES.md`.
- **For P3-5 and P3-10:** Smite's two skill parts; PoB's CALCS view uses `mainActiveSkillCalcs` and `skill_number`, the main output `mainActiveSkill`: the harness must check both select the same skill (they do in the seven fixtures).

## code-architecture-reviewer (P3-2, 2026-11-11) summary
No ADR or crate-boundary issue; deviations 1 (export instead of checkout) and 2 (`not-carried`) accepted. Applied:
- rules pin an exact `count` and fail on a key pattern that matched nothing, so a new difference on a covered row fails the run (a rule matched on table, key, and field only);
- the rules file names its `steam_build` and `pob_version`; a bundle or export of another version is refused;
- bundle rows, levels, and additional stat sets PoB lacks are listed in `report.json` (`bundle_only`, not gating, stated in `NOT_COMPARED`);
- support constraints and additional stat sets (35 effects; PoB exports the main set only) named in `NOT_COMPARED`;
- `named_in_notes` on `patch-delta` rules (`false` for the Vaal tags, shown in the summary); `pob@` citations only corroborate (a `v2.67.2:` or `game:` source is required);
- non-finite numbers never equal "absent"; monster levels found by level through a column table; `skill_points` from `grantedPassivePoints`; a support's PoB cast time compared; non-numeric level keys are differences; the export records `jit.version`;
- tests for the committed rules file, rendering, crit fallback, single-row levels, level not found, absent vs 0, armour ranges, gems, characters, constants, mastery effects, base name matching (19 tests).
Not applied: the overlay naming the bundle column per field (P3-5 defines the inverse with the transforms list); duplicate stat names in a PoB skill (none seen; would show as differences).

## code-architecture-reviewer (P3-1c, 2026-11-11) summary
No code blockers; the docs gap it named is this PR's docs commit. Applied: `movement_speed` required; per-game shape check; contiguous monster levels; value rules (positive attack time and divisor, no negatives); the armour error names both column sets; a game constant without an ID is an error; tests for each, a drop per table and a null key, missing weapon and monster columns; `MonsterLevel` docs on row lookup, `f32` widening, the armour constant; PoE2 weapon classes noted for P3-10. Not applied: a shared helper module for `required`/`float`/`one_of` (cosmetic); the commit trailer remark (the trailer is the one the session was given).

## Decisions in P3-4 (engine core; PR #68)
- **Game-neutral core, PoE1 rules in `loom_calc::poe1`.** `Quantity` and `Field` are closed enums with a `Poe1(..)` variant (as `loom_domain::GameSpecific`); spell suppression and spell block are PoE1-only (`POE1_ONLY` gained `SpellSuppression`, `SpellBlock`, `spell-suppression`, `spell-block`). Our names (`Field`'s `Display`: `life`, `resistance.fire`, `hit-dps`, `poe1.spell-block-chance`) are not PoB's; the slice id mapping (`TotalDPS` is `hit-dps`) belongs to P3-5's `calc-parity`.
- **A `Ruleset` (game, slice, mapping, `collect`, phases) runs in `calculate_with`.** The PoE1 rules have an empty mapping and no phases, so every one of the 24 slice fields is `Unsupported` (`not-computed`): no false `Complete`. A field in the slice that no phase writes is always `not-computed`. A PoE2 build (or rules of another game) gives `CalcResult::game_unsupported`: no fields, state `Unsupported`.
- **Data version:** `Build.data` `None` is `CALC_NO_DATA_VERSION` (import always records it); a different game, schema, or Steam build is `CALC_DATA_VERSION`. Other codes: `CALC_NO_MAIN_SKILL`, `CALC_MISSING_DATA` (defined for P3-6/P3-9a), `CALC_CANCELLED`, `CALC_GRAPH` (an engine bug: a node naming a later child).
- **Store order:** each query sorts the effects it combines by `Source`, then value (`total_cmp`), so insertion order never changes bits. Conditions: `all` flags, `any` flags, named `ConditionKey`s; a failed flag or settled condition rules an effect out before an unknown one is reported; an unknown condition leaves the effect out with `UnmodelledCondition` on the queried field only.
- **Chain:** `base * (1 + increased/100) * product(1 + more/100)`; an applying `Override` replaces it (the product stays in the graph). Overrides of different values: `Indeterminate` `ConflictingOverride`, the first by canonical order is the value.
- **Ingest reasons apply to every computed field** (conservative reading of decision 1, since which fields a stat reaches is unknown until mapped): `UnmappedStat`, `UnresolvedLine`, `PendingLine`, `ExcludedLine` (custom and section), `UnhandledKind` (every typed `kind` until a handler exists), and `Indeterminate` `UnsettledValue` (a stat value that is not one number: only `Exact` with `min == max` and `Hidden` with an assumption settle). `NotSelected` variant lines and `Continues` give no reason. Adapted lines carry their `Adapted` in `Source::Item`. Narrowing reasons to the fields they reach is a later step's choice.
- **States:** `StateKind` orders Complete < Partial < Unsupported < Indeterminate (contradiction is worst); the build state is the worst field. `Indeterminate` keeps the partial reasons too.
- **Graph:** append-only; children always earlier nodes (acyclic by construction); each node records the engine's value; `evaluate_all` recomputes from the leaves and `check` compares bits. Composite nodes carry a `Term` (`Base`, `Increased`, `More`, `Override`, `Result`, `Step(name)`) for `slice-fields.json`'s groupings.
- **Math:** `floor`, `ceil`, `trunc`, `round_half_up` (`floor(x + 0.5)`, PoB's `m_floor(x + 0.5)`; the largest double below 0.5 rounds to 1), `round_half_up_to` (up to 6 decimals), `min`, `max`, `percent_factor`; a bit table test run on Windows and Linux in CI.
- **Property tests without a crate:** a fixed-seed SplitMix64 in `crates/loom-calc/tests/engine.rs`, 300 cases per property, so the core gains no dependency (only `thiserror`, already in the workspace). Checked: the order test fails when the canonical sort is removed.
- **Cancellation:** `&dyn Fn() -> bool`, polled before collecting, after collecting, and after each phase.
- **For P3-13:** the DTO needs `Field`/`Quantity` `Display`, the reason/state `name()`s, and the graph's nodes; `loom-serialization` does not depend on `loom-calc` today (AGENTS.md crate boundaries), so P3-13 decides where the conversion lives.
- **Read side checked:** `from_json` runs `Document::check_shape` (`DTO_CALC_SHAPE`): children earlier, arity and term per kind, field node in range with equal value bits, reasons matching the state. It does not re-evaluate the graph (P3-5 can, with `loom_calc::math`, if it needs to).
- **Adding to v1:** strings for names that grow; a new reason, source, or node kind is added to v1 (schema and TypeScript regenerated) while every reader is in the repo and no document lives outside it, until the Phase 3 gate; after that, a new version with a migration. Written in `dto/calc/v1/mod.rs`.
- **Inputs are not in the document:** `CalcResult` has none; the runner (P3-5 `calc-run` summary) records build hash, bundle, oracle pin, slice revision.
- No ADR changed or extended; no divergence from the oracle (nothing is compared yet).

## code-architecture-reviewer (P3-13, 2026-11-11) summary
No blockers; one-way dependency, LF goldens, and the golden-bytes cross-platform argument accepted (cite the `rust (windows-latest)` and `rust (ubuntu-latest)` jobs). Applied: S1 the additive-variant rule (written down); S2 the read-side shape check with 11 rejecting tests; S3 the separate `reason` key for `unsupported` fields; S4 inputs recorded as the runner's (above); the macOS wording in the golden test; the shadowed `reasons` binding. Not applied: a `NodeIndex::get_u32` accessor (the cast is lossless and annotated; not worth touching `loom-calc` here); the double parse through `serde_json::Value` (as build v1; revisit if large graphs cost); schema `enum`s for `op`/`term` (strings on purpose, decision above).

## code-architecture-reviewer (P3-4, 2026-11-11) summary
No ADR violation, no scope miss; agreed with the ten design choices above. Applied:
- **B1:** an override of `-0.0` recorded `-0.0` while its one-leaf sum evaluates to `+0.0`, failing `Graph::check`; the value is now recorded as the sum gives it (`0.0 + v`); unit test, and the property generator now draws `-0.0` (the test fails without the fix).
- **S1:** `Calc::set_derived(field, query, deps)`: a field computed from other fields carries every reason of each dependency (never a false `Complete`); a dependency not computed makes it `Unsupported` with the new reason `DependsOnNotComputed`. P3-6 on use it for `LifeUnreserved`, `HitDps`, and the like.
- **S2:** `CalcResult.reasons` keeps the reasons from collecting (unmapped stats, quarantined lines), so they show while no field is computed (the gate's mutation set needs them).
- **N1:** the accepted `Hidden` assumption is documented at `ingest::settle`. **N3:** `UnmodelledCondition` names the left-out effect's quantity and source. **N4:** the PoE2 path polls the hook too.
Not applied: N2 (how many real `Exact` values have `min < max`; P3-8 counts them when it maps real stats), N5 (`Graph::evaluate` recomputes all; use `evaluate_all` in loops), N6 (byte comparison of the serialized document is P3-13's).

## Decisions in P3-13 (calc result document v1)
- **Where the conversion lives:** in `loom-serialization` (`dto::calc::v1`), which now depends on `loom-calc` (new edge `loom-calc` <- `loom-serialization`, AGENTS.md). The serialization layer depends on the model it writes, as it does on `loom-domain`; `loom-calc` stays free of serde and DTOs. No cycle: neither `loom-calc` nor `loom-data` depends on serialization. Rejected: DTOs in serialization with the conversion in `loom-calc` (the engine would depend on its persistence form), and the conversion in `tools/parity-cli` (the UI and `loom-app` need the same document).
- **One way:** the engine names rules, labels, terms, and reasons by `&'static str`, so a document does not convert back to a `CalcResult`. Readers (P3-5's `calc-parity`, the UI) read the DTO `Document` through `from_json`, which checks `format` then `version` first, as build v1.
- **Consistent or not written:** `to_json` runs `Graph::check` (`DTO_CALC_GRAPH`) and checks each field's value is its node's recorded value, bit for bit, and that value and node are both set or both absent (`DTO_CALC_FIELD`). So each node has one `value` (a contribution's value equals its recorded value), and a reader can recompute every value from the leaves.
- **Shape:** build v1's conventions. Nodes are `{kind, term?, children?, value}`: `kind` is `{"contribution": {source, quantity, op}}`, `{"constant": {label}}`, or a string (`sum`, `product`, `percent-factor`, `any`, `override`, `floor`, `round-half-up`, `min`, `max`); `children` in the engine's order (an override's `by` then `replaced`; min/max `a` then `b`); `term` absent on leaves and overrides. A field has `reason` (exactly when `unsupported`: why it is not computed) and two lists named by reason type, `unsupported` (what was not modelled: not empty for `partial`, possibly for `indeterminate`) and `indeterminate` (not empty exactly for `indeterminate`). Build-level reasons are the two lists. Fields in `Field`'s order (the map's), not `SLICE` order.
- **Names as strings:** fields and quantities by `Display`, reasons and states by `name()` (a test checks each tag equals it), ops by `Op::name`, terms `base`/`increased`/`more`/`override`/`result`/`step.<name>`. Strings keep game-specific names (`poe1.*`) out of shared DTO enums (ADR-001) and need no DTO change when P3-6 on add fields or quantities; the schema types them as strings.
- **Cross-platform:** no new CI job. The golden-bytes test runs in the required `rust` job on Windows and Linux; each must equal the committed bytes (LF by `.gitattributes`), which is stronger than a hash diff between the two. P3-5 adds the `calc-run` documents' hashes to `dto cross-platform` once real results exist.
- No ADR changed or extended; no divergence from the oracle (nothing is compared yet).
