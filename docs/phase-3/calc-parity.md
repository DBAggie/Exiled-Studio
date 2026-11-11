# Phase 3: the parity harness (P3-5)

Plan step 5 (`dev/active/phase-3-calc-slice/phase-3-calc-slice-plan.md`, decisions 5, 9, 10): `calc-run` writes the successor's results, `calc-parity` compares them with the oracle's, and the fixture manifest gets the `mechanic` tier. The oracle adapter moves to protocol 4, which sets PoB's CALCS view (the breakdowns) to its main view (`main_output`), the maintainer's answer to Q9. Commands and exit codes: `AGENTS.md`; register rules: `docs/parity/README.md`.

## `calc-run`

`loom-parity-cli calc-run <builds>... --bundle <dir> --out <dir> [--anonymous] [--data-overlay <overlay.json>]`

- Each build: import (PoB XML or a v1 build document), resolution with the bundle (validated first), `loom_calc::calculate`, the calc result document v1 (`documents/<id>.json`), read back with `from_json`. A build stops at a named step (`read`, `resolve`, `calculate`, `document`) with its stable code.
- `report.json` (deterministic for one tool commit; `calc-parity` reads it): the inputs (tool commit, bundle game, Steam build, data schema, `bundle.json` SHA-256, overlay) and per build its id, SHA-256, outcome, document SHA-256, and field counts by state. `summary.json`: counts only (build and field states, reason kinds, failure codes) and calculation time. `timing.json`: time per build, the baseline for ADR-013's budgets in Phase 4 (not gated, not deterministic).
- Ids as `oracle-run` gives them: the file name, or the SHA-256 with `--anonymous`, so the two runs pair. Two builds with one id stop the run.
- **Data overlay (decision 5).** `--data-overlay` reads `data-parity`'s `overlay.json` and refuses one made for another bundle (game, Steam build, data schema, `bundle.json` SHA-256) or with a row twice. `OverlayData` is a `CalcData` over the bundle with PoB's value per overlay row (`pob_value(table, key, field)`), in the tool only: the core keeps the game's data. `CalcData` has only `data_version` until P3-6 adds the first table reads, so today an overlay run gives the same documents as a plain run (tested); each read P3-6 on adds must consult the overlay.

## `calc-parity`

`loom-parity-cli calc-parity <oracle-run-dir> <calc-run-dir> --slice docs/phase-3/slice-fields.json --out <dir> [--register <file>] [--overlay-run <dir>] [--manifest <file>]`

Builds are paired by id. Per slice field (our names from `slice::OUR_NAMES`, tested equal to `loom_calc::poe1::SLICE`; PoB's names stay in this tool, decision 2):

| Our field | Outcome | Class |
|---|---|---|
| `complete`, equal to `main_output` | `match` | none |
| `complete`, within its `abs`/`rel` rule | `within-tolerance` | none |
| `complete`, different, or the oracle does not write the field | `difference` | 19 with an accepted entry that covers it, else 20 |
| `partial`, `indeterminate` | listed with its reason kinds, not compared | none |
| `unsupported` (not computed), or not in the result (PoE2) | `not-computed` | none |

**Breakdowns** (only for a `complete` field). The oracle's ten breakdowns are read by shape (`calc_parity::breakdown`): `chain` lines (`<n> ^8(base)`, `(<a> + <b>) ^8(base)`, `x <n> ^8(<label>)`, `= <n>`; labels base, increased/reduced, more/less, average damage or hit, attack/cast/trigger/hit rate, DPS and quantity multipliers), `min-max-total` (`Min: <n>%`, `Max: <n>%`, `Total: <n>%`), and `slots` (rows of `base`, `inc`, `more`, `source`, `sourceName`, `total`, an `item` marker; `{}` for none). A line or row the reader does not know, or a term the slice file does not list for the field, fails the build with `ORACLE_NORMALIZATION` (17), never a pass. Each term is compared with the node of our graph that its `graph` key names (`slice-fields.json` revision 2: `base`, `increased`, `more` (the nearest node with that term below the field's node; the search does not enter another field's `result` node), `result`, `step.<name>`, `field:<id>`; a slot row is the node `step.slot.<source>[.<sourceName>]` and its parts below it), **printed the way PoB prints the line** (`%g`, `%.Nf`, or Lua's `tostring`, emulated in `breakdown::Format`, tested against C's output), and the two texts must be equal. So `x 1.35` matches our 1.3549 and not our 1.3551. A term PoB prints that our graph lacks, and a required term neither side has, is a difference; an optional term PoB leaves out must print as 1 on our side. Where the oracle wrote no breakdown, the slice file's `absent_when` must apply (then only the optional terms are checked), else the breakdown is not compared.

**PoB's two views.** PoB computes `main_output` in its main view (the build's `mainSocketGroup`, the group's `mainActiveSkill`, the gems' `skillPart`, `skillStageCount`, `skillMineCount`, `skillMinion`, `skillMinionItemSet`, `skillMinionSkill`) and the breakdowns in its CALCS view, which reads the Calcs tab's own selection: the `skill_number` input and each value's `...Calcs` twin (`Modules/CalcSetup.lua`, `Modules/CalcActiveSkill.lua`, `Classes/SkillsTab.lua`). Since adapter protocol 4 (below) the oracle sets the CALCS selection to the main view's before PoB loads the build. As a tripwire, `calc-parity` still checks each chain breakdown's total against the main output printed the same way; when they differ, that build's breakdowns are not compared (`views_differ`, counted, and the build is `incomplete`).

**Register.** Differences are JSON pointers into the oracle's document (`/main_output/<key>`, `/breakdowns/<key>/<line>`, `/breakdowns/<key>/slots/<row>/<part>`, `/breakdowns/<key>/<term>` for a term only our graph has) and are classified by `oracle_compare::classify`. An entry of kind `data-version` covers a difference only when the `--overlay-run` compared that pointer and found it equal; without one, or when it differs, the difference stays unexpected with a note. Accepted entries for a compared build that covered nothing are listed (`unused_entries`).

**Sides.** A build without output on a side is not compared and keeps that side's class (the successor's: `ORACLE_IMPORT` at `read`, `ORACLE_CALCULATION` at `resolve` or `calculate`, `ORACLE_PROTOCOL` at `document`); both sides failing to import a fixture whose manifest expects it is an `expected_failure`. A build on one side only is unexpected; a build whose bytes differ between the two runs is not compared (`ORACLE_PROTOCOL`). A plain run passed as `--overlay-run`, an overlay run passed as the run compared, an overlay run of another bundle or of other builds (ids, SHA-256) or another tool commit, and a document whose bytes differ from the report's SHA-256 are refused.

**Nothing compared is not a pass.** A build with output on both sides but no `complete` field, or with a `complete` field whose breakdown was not compared (PoB's views differ, or the oracle wrote none where `absent_when` does not apply), is listed `incomplete` with what was not compared; the report is then `incomplete` and exits **5** unless a failure class or an unregistered difference (20) takes precedence. Until P3-6 computes fields, every run is `incomplete`.

**Mechanic fixtures** (decision 10, gate item 2). `tools/oracle/fixtures/fixtures.json` takes `tier: mechanic` (files in `mechanic/`, `targets`: the slice field ids the fixture isolates, sorted). With `--manifest`, each target must `match`, be `within-tolerance`, or be a registered difference; a target not computed fails the build (20), and a mechanic fixture missing from the comparison fails the report (`missing_mechanic`, 20). CI's `oracle` job runs every tier of the manifest, so mechanic fixtures run there as soon as P3-6 adds them.

## Adapter protocol 4: PoB's CALCS view set to the main view (Q9, maintainer 2026-11-11: option a)

**Found:** the oracle's breakdowns come from PoB's CALCS view, and builds keep the Calcs tab's selection apart from the main skill (the Ice Nova builds' main group is 32, their `skill_number` 2; Smite's main group 6 or 2, its `skill_number` 1). Under protocol 3 the `TotalDPS` breakdown described another skill in **all 7 in-scope builds** and 7 other public fixtures (for example `ice-nova-elementalist-campaign-lab1`: breakdown `21163.3 x 3.97 = 84002.1`, main output 115856.6), and in 12 of 21 corpus builds; `calc-parity` saw it as `views_differ` in 12 public and 11 corpus builds.

**Fix (`tools/oracle/adapter/oracle.lua`, `alignCalcsView`):** before PoB reads the build, the adapter parses it with PoB's XML parser, sets the Calcs section's `skill_number` input to `mainSocketGroup` (creating the input or section when absent), copies `mainActiveSkill` and each gem's six main-view values to their `...Calcs` twins in **every group of the active skill set** (both views calculate all of them: other groups' parts, stages, and minions give buffs), and composes the XML again with PoB's own `xml.ComposeXML`, the function PoB saves builds with. PoB then loads and calculates once, in its usual order, as if the user had selected the main skill in the Calcs tab. When no value differs (PoB reads an unset group, active skill, or part as 1), the original text is loaded unchanged. Every difference is recorded in the output's new `probe.calcs_view` (`"skill_number 2 -> 32"`, `"group 32 mainActiveSkillCalcs 1 -> 2"`). Protocol 4 in the manifest, the schema, `oracle_output.rs`, and the fake adapters.

**Rejected on the way, with measurements:** recalculating after loading. PoB's calculation passes are not repeatable within one process: running `CalcsTab:BuildOutput` again changed `main_output` in 11 of 36 public fixtures (Smite act 1 `AverageDamage` 919.6 to 703.2), and recomputing only the CALCS env halved the in-scope builds' Life breakdown (1546 against 2583). Copying the main group only left one corpus build's breakdown wrong (other groups' `...Calcs` values feed the main skill).

**Effect** (protocol 4 against the same adapter with the alignment switched off, same commit): `main_output` byte-identical in all 36 public and 21 corpus outputs; breakdowns changed only in the misaligned builds (14 public, 12 corpus; every change in `TotalDPS` only); `calc-parity` finds the views agreeing in every build; every fixture identical over 3 runs. The CI `oracle` job's runtime cache key includes `manifest.json`, so the first run rebuilds the runtime.

**Side observation, not this PR's change:** against Spike C's C5 corpus outputs (commit `aa940fd`), today's outputs with the alignment off differ in `main_output` for all 21 corpus builds, in enemy-related keys (`ChaosEnemyDamage`, `enemySkillTime`, ...). The bytes, PoB pin, and runtime are the same; the cause is not investigated here.

## Evidence (2026-11-11, Windows, clean)

Inputs: bundle `local/p3/p3-1c/evidence2/bundle-poe1-a` (Steam build 26530219, format 2); overlay `local/p3/p3-2/run4/overlay.json` (314 rows, P3-2). Outputs in `local/p3/p3-5/evidence/` (`ff8c539`) and `local/p3/p3-5/evidence-p4/` (`0f1ab6b`, protocol 4), local; scrubbed summaries in `docs/phase-3/calc-parity/`.

| Run | Builds | Result |
|---|---|---|
| `oracle-run`, fixture manifest, protocol 4 (`0f1ab6b`) | 37 | Exit 0: every expectation met, 3/3 each; views aligned in 29 (`probe.calcs_view`) |
| `oracle-run`, corpus, `--anonymous`, protocol 4 | 21 | Exit 0: 21 ok, 3/3 each |
| `calc-run`, public (fixtures, community, PoB's 3.13 builds), twice (`ff8c539`) | 37 | 36 documents, `malformed.xml` `IMPORT_XML` at `read` (exit 1, as expected); 864 fields, all `unsupported` (`not-computed`): P3-4's engine computes nothing yet. The two runs' documents and `report.json` byte-identical; the run at `0f1ab6b` gives the same documents |
| `calc-run`, corpus, `--anonymous`, twice | 21 | 21 documents, 504 fields `not-computed`; byte-identical, and equal at `0f1ab6b` |
| `calc-run --data-overlay`, public and corpus | 37, 21 | Same documents as the plain runs (the engine reads no table yet) |
| `calc-parity`, public, with register, overlay run, manifest (`0f1ab6b`) | 37 | Exit 5 (`incomplete`): no difference, but 36 builds have no complete field (every field listed `not-computed`), so nothing passed; 1 `expected_failure` (`malformed.xml`); breakdowns 360 `not-compared`; PoB's views agree in every build (12 differed under protocol 3) |
| `calc-parity`, corpus | 21 | Exit 5 (`incomplete`, 21 builds): 504 fields `not-computed`, 210 breakdowns not compared; views agree in every build (11 differed under protocol 3) |

Calculation time today (the empty PoE1 rules): median 1 µs, maximum 14 µs per build (release).

**Every breakdown line the public fixtures produce reads.** `tools/parity-cli/testdata/calc-parity/breakdown-lines.json` holds every distinct breakdown the protocol-4 oracle wrote for the 36 fixtures with output (399 distinct lines, every slots object, the `null` breakdowns), and `every_public_fixture_breakdown_should_read` reads each with its shape's reader and checks each term is one the slice file lists (Life: base, increased, more, total; resistances: min, max, total; TotalDPS: average, rate, DPS and quantity multipliers, total). Regenerate it from an `oracle-run` of the manifest when fixtures or the adapter change.


## Not in this PR

- No mechanic fixture yet: P3-6 adds the first ones (a bare character per class), as the plan says for each engine PR.
- No CI `calc-parity` job: the real bundle is not in CI (Q1, before P3-6). CI runs `calc-run` on the synthetic PoE1 and PoE2 builds with their synthetic bundles on Windows and Linux, and `dto cross-platform` compares their documents' SHA-256.
