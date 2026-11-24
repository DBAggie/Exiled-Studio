Last Updated: 2026-11-24 (maintainer decision: the rest of Phase 6 in one session)

# Phase 6 close-out (plan)

## The maintainer's decision (2026-11-24)
The maintainer asked for the rest of Phase 6 to fit into a single "continue" session, then chose the scope: **the stat mop-up only**. The remaining engineering items are deferred out of Phase 6 and listed in the report as named reasons (below).

## Scope
After P6-11ik, 450 distinct stats are still unmapped in the public fixtures and the corpus. 425 of them are in a single build. [`p6-close-out-list.md`](p6-close-out-list.md) puts each one in exactly one batch. A draft generator, `local/p6/tools/closeout_list.py`, made the list from PoB v2.67.2's `ModCache.lua`. The generator only drafts: each batch reviews its rows.

| Batch | Stats | What it does |
|---|---|---|
| `M1-conditions` | 38 | Rows under conditions the engine settles; new settled conditions where a config check exists |
| `M2-per-and-thresholds` | 21 | Per-stat, per-multiplier, and threshold lines (`stacks`, `charges`, `PerStat`) |
| `M3-plain` | 83 | Plain rows: PoB's mod maps straight onto an engine quantity |
| `M4-minion` | 12 | `MinionModifier` lines onto the minion actor |
| `M5-enemy` | 13 | `EnemyModifier` lines (`damage_taken`, `exposure`, the enemy's conditions) |
| `M6-buffs` | 19 | Aura, buff, Elusive, warcry, exert, and reservation lines |
| `M7-skill-scoped` | 9 | Lines scoped to a skill name, skill type, or socketed gems |
| `N-unparsed` | 126 | Lines PoB leaves unparsed or with text left over (below) |
| `H-hidden` | 13 | Stats without a description text: each read in PoB's data and code |
| `C-*` | 116 | Closed as named reasons, not mapped (below) |

**Rules, unchanged from the mop-up so far:**
- A stat is `NoEffect` only when PoB reads it into no output at all.
- A line PoB leaves unparsed applies nothing in PoB:
  - when it changes no slice field in the game either, it is `NoEffect`;
  - when it would change one, it stays a reason, listed as a game-truth item for the maintainer.
- A row that needs a system the engine lacks moves to a `C-*` reason. The move is recorded in the batch's section.

**Closed reasons** (`C-*`; the report lists each stat under its reason, with why):
- `C-unmodelled-skill` (91): text only in skill and gem descriptions, or lines owned by skills the engine does not model (Skitterbots, Radiant Sentinel, seals). They stay `unmodelled-gem` or `unmapped-stat` until their skill is modelled.
- `C-item-skill` (10): `ExtraSkill`, skills granted by items.
- `C-keystone` (6): keystones whose rule is not ported.
- `C-gem-property` (7): `GemProperty`, `ExtraSupport`, `LinkedSupport`.
- `C-no-slice` (2): PoB reads them into no slice field.

## Deferred out of Phase 6 (maintainer 2026-11-24)
Each item stays a named reason in the report:
- Spellslinger; Blood and Sand, with PoB's full DPS pass (tasks file);
- mine, trap, and totem costs per second;
- General's Cry's mirage warriors, Sacred Wisps;
- the other trigger configs: Arcanist Brand, Mark on Hit, Hextouch, Cast on Death, item triggers;
- spectres;
- ward's other lines;
- EHP's other pools (P6-10b's further parts);
- Skitterbots (data run);
- the minion's own damage over time.

The maintainer-run items stay the maintainer's: the data runs, the game-truth checks, the gate, and merging.

## How the session runs
1. **One commit per batch**, in the order M1 to M7, then H, then N; an `M3` batch over 30 stats splits in two. Each commit carries its evidence as before:
   - mechanic fixtures, several independent lines per fixture;
   - one public and corpus run per batch, with 0 unexpected and no value moved away from PoB without an explanation;
   - Q11 entries for attack fixtures;
   - the batch's section in `docs/phase-6/stat-mop-up.md`, recording each stat that moved batch or to a `C-*` reason.
2. **No stop between batches.** Stop only for a real blocker: a change that needs a calc result v2 variant (plan decision 3), an ADR conflict, or a failing check that cannot be fixed. The handoff's resume point names the next batch after every commit, so a context reset resumes there.
3. **The last commit** updates `docs/phase-6/phase-6-report.md`: the final counts, the closed reason list with each stat, the deferred items above, and the maintainer's game-truth list. The tasks file then marks P6-11 done, and the gate is the maintainer's.
