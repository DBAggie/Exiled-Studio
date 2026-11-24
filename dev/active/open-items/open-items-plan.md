Last Updated: 2026-11-24

# Open items (plan)

## Summary
The maintainer's request (2026-11-24, after PR #167 merged into `dev` as `6deeb5da`): "Do any open items you can in the next session. Any items that need my intervention store as a follow up. I specifically want to complete any open items that are possible to complete in this next session."

This belongs to `docs/plan/ACTION-PLAN.md`, "Phase 6+: Expansion", the PoE1 part: the engineering the Phase 6 close-out deferred, and the closed reasons it left (`docs/phase-6/phase-6-report.md`, "The close-out (2026-11-24)"). PoE2 calculations and PoE2 `.build` import are not in it; they are not planned yet (follow-ups file).

Everything that needs the maintainer is in [`maintainer-follow-ups.md`](maintainer-follow-ups.md). This plan holds only work an agent can finish without the maintainer.

## Current state (2026-11-24, `dev` at `6deeb5da`)
Evidence `local/p6/n/` (bundle `local/p6/packs/b-ct`):
- **Public:** 531 builds, 38,757 fields `Complete`; calc-parity 682 expected, 0 unexpected.
- **Corpus:** 21 builds, 0 fields `Complete` (804 `Partial`, 111 `Unsupported`, 762 indeterminate); 0 unexpected.
- **Distinct `unmapped-stat`:** 278.

Largest reasons by fields blocked (`python local/p6/tools/rank_reasons.py local/p6/n/public local/p6/n/corpus`):
- `unmodelled-gem` (public 1,419 fields in 19 builds; corpus 1,114 in 15). Public gems: Summon Skitterbots (5 builds), Rain of Spores, Raise Spectre, Increased Duration, Spellslinger, and about 25 gems with one build each (Arcane Cloak, Bane, Cold Snap, Conduit Sigil, Eclipse, Energy Blade, General's Cry, Living Lightning, Petrified Blood, Primal Aegis, Puresteel Banner, Sacred Wisps, Unbound Ailments, Vaal Grace, Vaal Smite, Vaal Ancestral Warchief, ...).
- `unresolved-line` `no-template` (public 1,259 fields in 17 builds, the only reason of 67 fields; corpus 757 in 10).
- `unmodelled-skill` `pob-skill-data` (public 219 fields in 31 builds, the only reason of 89 fields): main skills with no ported PoB skill data. Also `totem-data`, `reservation-becomes-cost`, `full-dps-skill`, `minion-buffs`, `socket-colour-quality`, `sigil-of-power-stages`, `skill-chill-effect`, `wave-of-conviction-exposure`.
- `unmodelled-passive`: `cluster-jewel-node`, `unconnected-node`, `unresolved-tree-line`, `timeless-jewel`.
- `unmodelled-config`: brittle, sapped, scorched, crushed, on brittle ground, `ActiveBrands`, `stormRainBeamOverlap`, `toxicRainPodOverlap`, `buffAlchemistsGenius`, `feedingFrenzyFeedingFrenzyActive`, `minionsConditionFullLife`.
- `unresolved-line` `ambiguous` and `no-variant-fits`, `scaled-line` (catalyst, modifier magnitude), `excluded-line` (section), `unmodelled-item` (`flask-base`, `ambiguous-base`, `slot`).

## Target state
- Every open item an agent can finish is done, each in a commit with its evidence.
- Every open item that needs the maintainer is in `maintainer-follow-ups.md`, with what is needed and why.
- Both calc-parity runs still have 0 unexpected differences.
- More public and corpus fields are `Complete` than at `6deeb5da`.
- The report and handoff name every reason left, each with the class it belongs to.

## Rules (unchanged from Phase 6)
- Port PoB v2.67.2 from `local/oracle/pob`, cited by file and line, with the same order of operations and rounding (`docs/plan`, plan decision 1 of `phase-6-mechanics-plan.md`).
- The game over PoB. Every difference from PoB is fixed or registered with evidence (ADR-001, ADR-006). **An agent registers only two kinds of entry:** Q11 entries under the confirmed rule (`python local/p6/add_q11.py`), and `pob-bug` entries backed by an instrumented PoB run whose diff and output are committed (S4, S10). Any other new difference is reverted: its rows go back to reasons and the line goes on the game-truth list in `maintainer-follow-ups.md`. A `game-truth` entry needs the maintainer's check in the game.
- **Before each step,** list the fields, reasons, and node kinds it will emit, and check each against the existing `FieldResult`, `UnsupportedReason`, and `Node` variants (`crates/loom-serialization/src/dto/calc/v1/types.rs`). A step that needs a new variant goes to the follow-ups before any code is written.
- **Fixture budget:** one fixture adds about the oracle jobs' last run time divided by their fixture count (measure it in S0 from the last green `dev` or #167 run). Keep each step's new fixtures inside its share of the 150-minute limit's headroom, with several independent lines per fixture.
- Stat-ID first. Nothing dropped silently: what stays unmodelled stays a reason (ADR-015).
- Core crates stay wasm32-compatible (ADR-016).
- **No new closed variants in calc result v1** (Phase 6 plan decision 3). An item that truly needs one is not done here: it goes to the follow-ups file as a v2 decision, and the session moves on. It does not stop the session.
- A stat is `NoEffect` only when PoB reads it into no output at all. A line PoB leaves unparsed that would change a slice field in the game stays a reason and goes on the game-truth list (follow-ups).
- Instruction counts (ADR-013): each engine change stays inside the `bench` limits or re-baselines with a stated reason.

## Steps

### S0. Survey and classify (S)
Classify every distinct reason in the public and corpus runs at `dev`'s tip into exactly one class:
- **A, agent:** code that can be written and checked against PoB now, with the bundle and oracle on this workstation.
- **B, data run:** needs game data that no evidence bundle on this workstation has (`local/p6/packs/`: `b-ct` already carries data schema 5 with the cluster, timeless, and tattoo tables of build 26530219; check it, `b-cl`, and `b-tl` before calling anything B). Candidates: `VaalSouls`, Skitterbots' minion data, monster varieties for spectres, newer item templates for `no-template` lines from a later league. Evidence bundles stay on one game build each (26530219); a newer build is the maintainer's data run.
- **C, maintainer:** needs a game check, a decision, an ADR or v2 change, or the maintainer's hardware.

Tools: `rank_reasons.py`, `unmapped.py`, `gems_unmodelled.py` (public), and `corpus_gemdiff.py` for corpus builds (by content hash, never by name). `no-template` lines are checked against the bundle's stat descriptions: a line whose template the bundle has is class A (a resolver gap); one whose template it lacks is class B.

**Acceptance:**
- `open-items-inventory.json` (machine-readable: reason key as `rank_reasons.py` prints it, class, fields blocked, builds, effort S/M/L/XL, step) and `open-items-inventory.md` (the same, readable) list each distinct reason once.
- Class A items carry their step and an order by fields cleared per unit of effort.
- Class B and C items are copied into `maintainer-follow-ups.md`.
- A check script, `local/p6/tools/inventory_check.py <inventory.json> <run-dir>...`, fails when a reason in the runs is missing from the inventory. It is the gate's item 3.
- The fixture budget is measured and written in the inventory.

### S1. Main skills without PoB skill data (L)
The `pob-skill-data` main skills (31 public builds), and the related `unmodelled-skill` kinds: `totem-data`, `reservation-becomes-cost`, `full-dps-skill`, `minion-buffs`, `socket-colour-quality`, `sigil-of-power-stages`, `skill-chill-effect`, `wave-of-conviction-exposure`. Port each skill's `parts`, `baseFlags`, `baseMods`, `statMap`, and skill functions through the existing generated table (Phase 6 plan decision 5), most builds first.
**Acceptance:** each ported skill has a mechanic fixture matching PoB, or is moved to the follow-ups with why.

### S2. Unmodelled support and active gems (L)
The class A gems from S0 (public list above, plus the corpus's), each through `poe1::gems` with its `gem_effects` row checked by `data-parity` (`uncategorized` 0).
**Acceptance:** each gem modelled with a fixture, or moved to B or C with why.

### S3. Triggers and Spellslinger (XL)
The deferred trigger configs: Spellslinger, Arcanist Brand, Mark on Hit, Hextouch, Cast on Death, Cast when Damage Taken, Mirage Archer, Manaforged Arrows, item triggers (`trigger_socketed_spell_on_skill_use_%`). Port PoB's trigger rate rules (`CalcTriggers.lua`) on the P6-8a model.
**Acceptance:** each trigger kind has a fixture matching PoB; the stats it owns leave `unmapped-stat`.

### S4. Blood and Sand, and PoB's full DPS pass (M)
Name the modifier that carries over between skills in `calcs.calcFullDPS` (tasks file of Phase 6, the Blood and Sand item). Use an instrumented copy of the pinned PoB in the scratchpad, never `local/oracle/pob` itself and never `D:\POE Dev\PathOfBuilding`. Then model Blood and Sand. If PoB's carry-over is a PoB bug, register it (`pob-bug`).
**Evidence kept:** the instrumentation diff and the output excerpt are committed into the step's doc section (`docs/phase-6/`), since the scratchpad and `local/` are not kept.
**Acceptance:** a fixture with both stances matches PoB on its main skill, or the difference is registered with the committed instrumented run as evidence.

### S5. Costs per second, warcries, mirages, wisps (L)
Mine, trap, and totem costs per second; warcry power and the cries that scale with it (Battlemage's, Ancestral, Infernal, Seismic, Intimidating); General's Cry's mirage warriors; Sacred Wisps.
**Acceptance:** each with a fixture matching PoB, its stats mapped.

### S6. Minions: damage over time and spectres (L)
The minion's own damage over time through the P6 DoT phases under the minion actor. Spectres only if S0 finds their monster data in the bundle; otherwise they go to the follow-ups (B).
**Acceptance:** minion DoT fields match PoB on a fixture.

### S7. Defences: ward, EHP's other pools, maximum hit (L)
P6-10b's further parts: ward's other lines (Wicked Ward, ward not breaking, bypass, restore on block, Ghostwrithe, armour and evasion converted to ward), EHP's other pools, damage over time EHP, the reduction against a hit, conversion smoothing in maximum hit, degeneration (D7), a dual wielder's ailments and leech. Soul costs only with `VaalSouls` (B).
**Acceptance:** each field or line matches PoB on a fixture; new slice fields are additive (slice revision 11, Phase 6 plan decision 2).

### S8. Config inputs and the closed reasons (M)
- **Config inputs** from S0 that are class A: brittle, sapped, scorched, crushed, on brittle ground, `ActiveBrands`, the overlap inputs, Alchemist's Genius, Feeding Frenzy, minions at full life.
- **The closed reasons** of the close-out, revisited now that the scope allows them: `C-engine-system` (70), `C-keystone` (6), `C-gem-property` (7), `C-item-skill` (10), `C-no-slice` (2), and the stats `C-unmodelled-skill` ties to skills S1 and S2 model.
- **Repeats' other modes**, `FINAL` and `FINAL_DPS` (P6-11aj modelled `AVERAGE` and `NONE`; `docs/phase-6/phase-6-report.md`, "Still open").
- **The two stats** kept only by guard tests: `chance_to_gain_adrenaline_for_2_seconds_on_leech_removed_by_filling_unreserved_life_%` and `travel_skills_cannot_be_exerted`. Mapped if PoB reads them into an output; otherwise `NoEffect`, with the guard tests in `poe1::buffs` and `poe1::gems` updated to allow them by name.
**Acceptance:** each stat mapped with a test or a fixture, or listed with its class.

### S9. Resolver and item gaps (M)
The class A share of `unresolved-line` (`no-template` with a template in the bundle, `ambiguous`, `no-variant-fits`), `scaled-line` (catalyst, modifier magnitude), `excluded-line` (section), and `unmodelled-item` (`flask-base`, `ambiguous-base`, `slot`, `item-property`). Port PoB's reading where it has one.
**Acceptance:** each fixed with a test, or classed B or C.

### S10. PoB's mana `PercentStat` (S)
The open item in the report: PoB's 5% of "Regenerate 10% of Mana over 2 seconds" reads more than the shown maximum mana. Find the cause with an instrumented scratchpad copy of PoB. Then map `regenerate_%_maximum_mana_over_2_seconds_on_consuming_corpse` as PoB does (if PoB is right) or register it (`pob-bug`).
**Acceptance:** the stat mapped or registered, with the instrumentation diff and output committed into `docs/phase-6/stat-mop-up.md`'s section for it.

### S11. Report and close (S)
A new section in `docs/phase-6/phase-6-report.md`, "Open items (2026-11-25)": the counts, what each step did, every reason left with its class. Then update the handoff, CLAUDE.md, this tasks file, and `maintainer-follow-ups.md`.

## Order
S0 first. Then the class A items by **fields cleared per unit of effort**, not by fields blocked alone, so the small wins land before the large steps. The order S0 is expected to give: S8 (config inputs, closed reasons), S10, the class A part of S9, S2's single-build gems, S1, S5, S6. **Stretch:** S3 (triggers), S4 (Blood and Sand), S7 (defences) are the largest and were deferred by the maintainer on 2026-11-24. Their items are written into `maintainer-follow-ups.md` up front as "planned, not reached", and removed from it as they are done, so a stop is a planned stop. S11 last.

No stop between steps; stop only for a failing check that cannot be fixed. A step that cannot finish moves its rest to the follow-ups, and the session continues.

## Gate (checked by scripts)
1. `calc-parity` on public and corpus: 0 unexpected (`sh local/p6/evidence.sh <step>`).
2. **Every new divergence-register entry** is a Q11 entry or a `pob-bug` entry whose evidence is committed (rules above); no new `game-truth` entry.
3. Public `Complete` count ≥ 38,757 at every commit and higher at the end. Corpus: the fields blocked per build (`rank_reasons.py`) fall from the S0 baseline; the corpus `Complete` count may stay 0, since every corpus build carries data or closed reasons.
4. Every distinct reason in the final run is in `open-items-inventory.json` with a class (`inventory_check.py` passes), and every class B or C item is in `maintainer-follow-ups.md`.
5. `cargo fmt --all -- --check`, clippy with `-D warnings`, the workspace tests, loom-calc's real-bundle tests (`LOOM_BUNDLE_DIR=$PWD/local/p6/packs/b-ct cargo test -p loom-calc -- --ignored`), `data-parity` `uncategorized` 0 where gem tables change, and CI's checks green.

## Fallback
- An item that fails its fixture after a real attempt, or differs from PoB without Q11 or `pob-bug` evidence: revert its rows. It stays a reason and goes to the follow-ups with the difference found (the game-truth list when the game may differ).
- An item that needs a calc result v2 variant, an ADR change, or game data: to the follow-ups, class C or B.
- If the fixture budget runs out: stop adding fixtures, test the rest by unit tests only, and list the untested rows in the follow-ups. Raising `timeout-minutes` again or splitting the oracle jobs is the maintainer's decision (asked in the follow-ups).

## Risks and mitigations
- **The scope is larger than one session.** The steps run in impact order and each commit is complete, so a stop leaves a consistent branch. The handoff's resume point names the next step after every commit.
- **Corpus builds are from a later league than the bundle.** Many `no-template` and tree reasons may be data (class B). S0 separates them first, so no time goes into lines the bundle cannot hold.
- **The oracle CI jobs grow with each fixture** (80 to 113 minutes on the last green runs; the limit is 150, so about 37 minutes of headroom). Mitigation: the fixture budget, several independent lines per fixture, and the maintainer asked up front about splitting the jobs.
- **A step needs a v2 variant only once it is half-built.** Mitigation: the variant check before each step (rules).
- **Instrumenting PoB.** Only a scratchpad copy is changed. The pinned oracle and the legacy repository stay read only.

## Planned PR breakdown
One PR for the session, from `dev` (branch `feat/open-items`), one commit per step with its evidence, as the maintainer asked for Phase 6's open tasks and close-out. Each commit carries:
- tests and mechanic fixtures for what it models;
- the public and corpus calc-run and calc-parity summaries (`local/p6/<step>/`), with value moves explained (`move_cmp.py`);
- divergence-register entries for each difference from PoB;
- `data-parity` when a gem table changes;
- its section in `docs/phase-6/stat-mop-up.md` (or the doc of its area), and the tasks file ticked.

ADR-012 asks for small PRs; the maintainer has accepted one PR with a commit per step for this kind of work (#167). To keep the review tractable, the PR body keeps a per-step index of commits and evidence (`local/p6/tools/open-items-pr.md`).
