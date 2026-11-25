Last Updated: 2026-11-24

# Open items (context)

## Where things are
- **Branch:** `feat/open-items` from `dev` at `6deeb5da` (PR #167's squash). **PR #168** (base `dev`), opened 2026-11-24. PR body draft: `local/p6/tools/open-items-pr.md` (`gh pr edit 168 --body-file`).
- **Evidence bundle:** `local/p6/packs/b-ct`. Oracle: `local/p6/oracle` (public) and `local/p3/p3-5/evidence-p4/oracle-corpus` (corpus). The latest run is `local/p6/n/`; it is the baseline for S0.
- **Local tools:** `local/p6/tools/` (its `README.md` gives the per-step routine) and `local/p6/*.py`, `local/p6/evidence.sh`. New: `local/p6/tools/rank_reasons.py <run-dir>...` ranks reasons by fields blocked, builds, and fields where it is the only reason.
- **Per step:** edit, `cargo fmt --all`, `cargo test -p loom-calc`; build `loom-parity-cli` to `local/p6/lpc-<step>.exe`; fixtures in `local/p6/<step>-fx`; `oracle-run tools/oracle/manifest.json local/oracle --out <x>-o <x>-fx`; `python local/p6/add_fixtures.py spec.json`; Q11 entries `python local/p6/add_q11.py`; `BUNDLE=local/p6/packs/b-ct sh local/p6/evidence.sh <step> <lpc>`; `python local/p6/move_cmp.py local/p6/<prev>/parity local/p6/<step>/parity --list` (and `parity-corpus`); fmt check, clippy, workspace tests, real-bundle tests; docs; commit; push.
- **The PR body:** keep a draft at `local/p6/tools/open-items-pr.md` with the template's evidence headings (memory: the PR evidence check fails without them), and push it with `gh pr edit <N> --body-file`.

## Key code
- `crates/loom-calc/src/poe1/mapping.rs`: `HIT_ROWS`, `STATE_ROWS`, `STATE_DIVIDED_ROWS`, `ROWS`, `NO_EFFECT`, `NO_EFFECT_DEPENDS_ON`, `HIT_REASONS`.
- `poe1::gems` (gem effects, `gem_effects` data-parity table), `poe1::skill`, `poe1::hit`, `poe1::dot`, `poe1::full_dps`, `poe1::costs`, `poe1::minions`, `poe1::conditions`, `poe1::stacks`, `poe1::buffs`, `poe1::damage_taken`, `poe1::flasks`, `poe1::regen`.
- The guard tests that list `NoEffect` rows by their words (`damage_taken`, `charges`, `config`, `gems`, `buffs`, and the real-bundle tests in `crates/loom-calc/tests/`). A new `NoEffect` row may need adding to one of them.
- PoB v2.67.2 at `local/oracle/pob`: `Modules/CalcTriggers.lua`, `Modules/Calcs.lua` (`calcFullDPS`), `Modules/CalcDefence.lua`, `Modules/CalcOffence.lua`, `Data/Skills/*.lua`, `Data/Minions.lua`, `Data/Spectres.lua`.

## ADRs and plan decisions
- ADR-001, ADR-006: the game over PoB; each difference registered.
- ADR-005: game data from our own extraction; the extraction is maintainer-run.
- ADR-013: instruction-count limits.
- ADR-015 and its amendments: stat-ID first, nothing dropped silently.
- ADR-016: wasm32-compatible core.
- `dev/active/phase-6-mechanics/phase-6-mechanics-plan.md`, design decisions 1 to 13. Decision 3 (no new calc result v1 variants) holds; an item that needs one goes to the follow-ups.

## Decisions
- **2026-11-24 (maintainer):** "Do any open items you can in the next session. Any items that need my intervention store as a follow up." So the session does not stop for a blocked item. It records it in `maintainer-follow-ups.md` and moves on.
- **One PR, a commit per step,** as in Phase 6's open tasks (#167).
- **PoE2 is out of scope.** It is not planned, and planning it is the maintainer's call.
- **Instrumenting PoB** (S4, S10) uses a scratchpad copy of `local/oracle/pob`. The pinned oracle and `D:\POE Dev\PathOfBuilding` are never changed. The instrumentation diff and its output are committed into the step's doc, since the scratchpad and `local/` are not kept.
- **Plan review (2026-11-24, `plan-reviewer`):** no ADR conflict. Applied:
  - an agent registers only Q11 and `pob-bug` entries; other differences revert and go to the game-truth list;
  - the gate is checkable by script (`open-items-inventory.json`, `inventory_check.py`); the corpus target is fewer blocked fields, not a `Complete` count;
  - `b-ct` already has the cluster, timeless, and tattoo tables, so those reasons are checked as agent work first;
  - the follow-ups list was completed against the reports and tasks files;
  - the order is by effort, with S3, S4, and S7 as stretch;
  - each step has a fixture budget, and checks for calc result v2 variants before it starts.

## Open questions
- None yet. S0 may raise some; each goes to the follow-ups file.
