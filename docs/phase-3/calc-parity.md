# Phase 3: the parity harness (P3-5)

Plan step 5 (`dev/active/phase-3-calc-slice/phase-3-calc-slice-plan.md`, decisions 5, 9, 10): `calc-run` writes the successor's results, `calc-parity` compares them with the oracle's, and the fixture manifest gets the `mechanic` tier. Commands and exit codes: `AGENTS.md`; register rules: `docs/parity/README.md`.

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

**PoB's two views.** PoB computes `main_output` in its main view (the build's `mainSocketGroup`) and the breakdowns in its CALCS view, which selects the socket group from the Calcs tab's `skill_number` (`Modules/CalcSetup.lua`: `env.calcsInput.skill_number` in CALCS mode, `build.mainSocketGroup` otherwise), and the active skill and part from `mainActiveSkillCalcs`/`skillPartCalcs`. When a chain breakdown's total is not the main output's value printed the same way, the two views computed different things, and that build's breakdowns are not compared (`views_differ`, counted in the report).

**Register.** Differences are JSON pointers into the oracle's document (`/main_output/<key>`, `/breakdowns/<key>/<line>`, `/breakdowns/<key>/slots/<row>/<part>`, `/breakdowns/<key>/<term>` for a term only our graph has) and are classified by `oracle_compare::classify`. An entry of kind `data-version` covers a difference only when the `--overlay-run` compared that pointer and found it equal; without one, or when it differs, the difference stays unexpected with a note. Accepted entries for a compared build that covered nothing are listed (`unused_entries`).

**Sides.** A build without output on a side is not compared and keeps that side's class (the successor's: `ORACLE_IMPORT` at `read`, `ORACLE_CALCULATION` at `resolve` or `calculate`, `ORACLE_PROTOCOL` at `document`); both sides failing to import a fixture whose manifest expects it is an `expected_failure`. A build on one side only is unexpected; a build whose bytes differ between the two runs is not compared (`ORACLE_PROTOCOL`). A plain run passed as `--overlay-run`, an overlay run passed as the run compared, an overlay run of another bundle or of other builds (ids, SHA-256) or another tool commit, and a document whose bytes differ from the report's SHA-256 are refused.

**Nothing compared is not a pass.** A build with output on both sides but no `complete` field, or with a `complete` field whose breakdown was not compared (PoB's views differ, or the oracle wrote none where `absent_when` does not apply), is listed `incomplete` with what was not compared; the report is then `incomplete` and exits **5** unless a failure class or an unregistered difference (20) takes precedence. Until P3-6 computes fields, every run is `incomplete`.

**Mechanic fixtures** (decision 10, gate item 2). `tools/oracle/fixtures/fixtures.json` takes `tier: mechanic` (files in `mechanic/`, `targets`: the slice field ids the fixture isolates, sorted). With `--manifest`, each target must `match`, be `within-tolerance`, or be a registered difference; a target not computed fails the build (20), and a mechanic fixture missing from the comparison fails the report (`missing_mechanic`, 20). CI's `oracle` job runs every tier of the manifest, so mechanic fixtures run there as soon as P3-6 adds them.

## Evidence (2026-11-11, Windows, tool commit `ff8c539`, clean)

Inputs: bundle `local/p3/p3-1c/evidence2/bundle-poe1-a` (Steam build 26530219, format 2); overlay `local/p3/p3-2/run4/overlay.json` (314 rows, P3-2); oracle runs: the fixture manifest (37 fixtures, 3 runs each, every expectation met, exit 0; `local/p3/p3-5/oracle-run`) and the corpus run of Spike C's C5 (`local/oracle-run/c5-corpus-windows`, `--anonymous`, 21 builds). Outputs in `local/p3/p3-5/evidence/` (local; scrubbed summaries in `docs/phase-3/calc-parity/`).

| Run | Builds | Result |
|---|---|---|
| `calc-run`, public (fixtures, community, PoB's 3.13 builds), twice | 37 | 36 documents, `malformed.xml` `IMPORT_XML` at `read` (exit 1, as expected); 864 fields, all `unsupported` (`not-computed`): P3-4's engine computes nothing yet. The two runs' documents and `report.json` byte-identical (`report.json` `6e525761…`) |
| `calc-run`, corpus, `--anonymous`, twice | 21 | 21 documents, 504 fields `not-computed`; byte-identical (`64859536…`) |
| `calc-run --data-overlay`, public and corpus | 37, 21 | Same documents as the plain runs (the engine reads no table yet) |
| `calc-parity`, public, with register, overlay run, manifest | 37 | Exit 5 (`incomplete`): no difference, but 36 builds have no complete field (every field listed `not-computed`), so nothing passed, 1 `expected_failure` (`malformed.xml`); breakdowns 360 `not-compared`; **12 builds with PoB's views differing**. A second comparison (the second run) gives the same report but the successor record |
| `calc-parity`, corpus | 21 | Exit 5 (`incomplete`, 21 builds): 504 fields `not-computed`, 210 breakdowns not compared; **11 builds with PoB's views differing** |

Calculation time today (the empty PoE1 rules): median 1 µs, maximum 10 µs per build (release).

**Every breakdown line the public fixtures produce reads.** `tools/parity-cli/testdata/calc-parity/breakdown-lines.json` holds every distinct breakdown the oracle wrote for the 36 fixtures with output (397 distinct lines, every slots object, the `null` breakdowns), and `every_public_fixture_breakdown_should_read` reads each with its shape's reader and checks each term is one the slice file lists (Life: base, increased, more, total; resistances: min, max, total; TotalDPS: average, rate, DPS multiplier, total). Regenerate it from an `oracle-run` of the manifest when fixtures change.

## For the maintainer: PoB's CALCS view computes another skill in every in-scope build

In the oracle's outputs, the `TotalDPS` breakdown of **all 7 in-scope builds** (Ice Nova 3, Smite 4) and of 5 other public fixtures is not the main output's `TotalDPS` (for example `ice-nova-elementalist-campaign-lab1`: breakdown `21163.3 x 3.97 = 84002.1`, main output `AverageDamage` 31433.97, `Speed` 3.686, `TotalDPS` 115856.6). Life and Mana always agree. The fixtures do not set the Calcs tab's socket group (`<Calcs><Input name="skill_number">` is absent or 1; the Ice Nova builds' main group is 32, Smite's 6), so PoB's CALCS view calculates socket group 1. P3-3b's note that the views select the same skill held for `mainActiveSkill`/`mainActiveSkillCalcs` within the main group, not for the group itself. Until this changes, `calc-parity` compares no breakdown of those builds (it says so per build), so **gate item 3 cannot be met for the in-scope builds**. Two ways to fix it, for the maintainer to choose (Q9):

- **(a) The adapter sets the CALCS view to the main view** before reading the breakdowns: `calcsTab.input.skill_number = build.mainSocketGroup`, and the main group's `mainActiveSkillCalcs` and its gems' `skillPartCalcs` to their main-view values. Recommended: one rule for every fixture and corpus build, written in the adapter (a protocol bump, so every oracle output and the CI `oracle` job's expected hashes change once).
- **(b) The fixtures set it:** add `<Input name="skill_number" number="<mainSocketGroup>"/>` to the derived fixtures (new pins; the community builds and the corpus stay unfixed).

## Not in this PR

- No mechanic fixture yet: P3-6 adds the first ones (a bare character per class), as the plan says for each engine PR.
- No CI `calc-parity` job: the real bundle is not in CI (Q1, before P3-6). CI runs `calc-run` on the synthetic PoE1 and PoE2 builds with their synthetic bundles on Windows and Linux, and `dto cross-platform` compares their documents' SHA-256.
