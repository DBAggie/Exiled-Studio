Last Updated: 2026-11-11 (PR #59, P3-0: plan written, plan-reviewer findings applied)

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
| `local/pob-v2.67.2` | `git archive` of PoB's `src/Data` at the oracle tag (Spike B), for `data-parity` |
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
- **Q7** The excluded-mechanics list (plan decision 3), written in P3-3 from game data. Default: what P3-3 proposes, at least timeless and cluster jewels in allocated sockets if the survey shows they reach slice fields; the maintainer confirms before P3-6.

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
