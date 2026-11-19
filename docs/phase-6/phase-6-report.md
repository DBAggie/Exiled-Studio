# Phase 6 report: PoE1 mechanics expansion

Phase 6 is the PoE1 mechanics expansion with the stat mop-up (`dev/active/phase-6-mechanics/`). The maintainer asked for it on 2026-11-17: "do phase 6 in full without stopping. We can do gate testing after."

This report is P6-12's: the counts, the register, the gate checklist, and the performance budgets re-checked. It was first written at PR #167's commit `f366c65` (P6-11n). The counts and the reasons below are updated to P6-11ac (`a1676a8`); the Phase 3 and Phase 6 set figures and the performance record are still P6-11n's.

**Verdict: the gate is not met yet.**
- Gate item 1 holds: every `Complete` field equals PoB's or is registered.
- Item 3 holds for the public builds but not for the corpus, where no field is `Complete` yet.
- Item 2's closed list is below. The steps still open are listed under "Still open".

## The counts

Every count is from `local/p6/p6-11ac/`, made by `sh local/p6/evidence.sh p6-11ac` on `local/p6/packs/b-ot` (the evidence bundle with the tree packs 3_25 to 3_28).

| Set | Builds | Fields `Complete` | Partial | Unsupported | Indeterminate | calc-parity unexpected |
|---|---|---|---|---|---|---|
| Public, start (P6-0b, 2026-11-17) | 136 | 2,270 | 386 | 482 | 102 | 0 |
| Public, at P6-11n | 210 (209 calculate; one is the expected failure) | 13,547 | 1,925 | 812 | 443 | 0 |
| Public, now (P6-11ac) | 212 (211 calculate; one is the expected failure) | 13,953 | 1,669 | 816 | 447 | 0 |
| Corpus, start | 21 | 0 | 193 | 141 | 170 | 0 |
| Corpus, now | 21 | 0 | 791 | 124 | 749 | 0 |

- **The Phase 3 set** (the 24 fields every document carries; at P6-11n):
  - Public: 3,090 `Complete` of 4,180, against 2,270 of 3,240 at the start. The new fixtures are mostly complete builds.
  - Corpus: 0 of 420.
- **The Phase 6 set** (slice revisions 3 to 22, each field present when it applies; at P6-11n):
  - Public: 10,457 `Complete`.
  - Every computed field that is complete equals PoB's or is registered.
- **Public calc-parity:** 280 expected differences, 0 unexpected. Fields matching PoB: 13,535 at P6-11w, 13,651 at P6-11ac.

## The register

`docs/parity/divergence-register.json` holds 293 entries: 292 `game-truth` and 1 `pob-bug` (DIV-0282 to DIV-0293, P6-11aj's Multistrike fixtures, are Q11 entries too).
- **Q11 entries (Phase 6 adds DIV-0102 to DIV-0293, except DIV-0111 to DIV-0113 and DIV-0174):** the game floors a weapon's attack time to whole milliseconds (Q11, answered in game 2026-11-12). Each attack fixture's `Speed` difference is carried into the fields that read it:
  - the hit's DPS;
  - combined and full DPS;
  - triggers;
  - the mirage;
  - the costs per second;
  - leech.

  A field is registered only where the build's `Speed` entry is.
- **DIV-0111 to DIV-0113 and DIV-0174:** Vaal Summon Skeletons' minion level. The game's `base_display_minion_actor_level` is 68 at gem level 20; PoB uses the level requirement, 70. Confirmation in game is a maintainer item.

## The gate checklist

1. **Every `Complete` field matches PoB or has a registered, evidenced divergence.** Met: 0 unexpected differences, public and corpus.
2. **No build leaves a field `Unsupported` for a mechanic marked done; the remaining reasons form a closed list.** The list is below. Every `Unsupported` field is either:
   - a mechanic still open (a dual wielder's ailments and leech, traps', mines', and totems' own rates, conversion smoothing in maximum hit, the config inputs below); or
   - not computed for the build's main skill (no hit, no skill).
3. **The `Complete` counts rise.**
   - Public: met, 2,270 to 13,953.
   - Corpus: not met, still 0. Every corpus build carries unmapped stats and unmodelled gems: 21 of 21 builds for each.

### The remaining reasons (the closed list)

Counted once per build, on the `life` field's reasons in all three of its lists (unsupported, indeterminate, partial); a build-wide reason reaches every field. Public 211 builds, corpus 21 (P6-11ac):

| Reason | Public builds | Corpus builds | What it is |
|---|---|---|---|
| `unmapped-stat` | 26 | 21 | 929 distinct stats across both sets, on any field. These include the field markers (`EhpReason`, `PoolReason`, ...), which are reasons by design. The rest is the P6-11 tail: heralds, marks, brands, sigils, resistance conversions, Eldritch Battery, Eternal Youth, granted notables, guards and Molten Shell, and many uniques' own lines. |
| `unmodelled-gem` | 26 | 21 | A gem of the active set the engine does not model: Frostblink, Second Wind, Toxic Rain, Reflection, Sniper's Mark, Immortal Call, Flame Wall, Spellslinger, Eternal Blessing, Raise Spectre, and others (the rest of P6-9), and the supports linked to them. |
| `unmodelled-config` | 14 (28 inputs) | 12 (16 inputs) | Config inputs no step models yet: the minion's own charges and conditions, brands attached, Feeding Frenzy, Alchemist's Genius, ground effects on the enemy, Toxic Rain and Storm Rain overlaps. |
| `unresolved-line` | 22 | 12 | Item or tree lines the resolver cannot settle: no template, ambiguous, or no variant fits (ADR-015). |
| `unmodelled-passive` | 21 | 12 | Cluster jewel nodes: the evidence bundle has no cluster tables, which wait for the maintainer's extraction. Also unconnected nodes and unresolved tree lines. |
| `tree-version` | 5 | 10 | A build saved on a tree the bundle does not carry (3_13, 3_15, 3_22, 3_23, 3_26 alternate). |
| `scaled-line`, `excluded-line`, `unread-line`, `unsettled-value` | 12, 10, 1, 1 | 4, 5, 0, 0 | Item lines PoB itself scales or skips (P3-8b). |
| `unmodelled-item` | 9 | 6 | Abyssal sockets, flask bases, ambiguous bases, flask merges, a jewel limit, a slot. |
| `unmodelled-skill` | 5 | 2 | Socket colour quality, a reservation, Rallying Cry's allies, a shield requirement. |
| `unmodelled-condition` | 4 | 3 | A passive's condition not settled. |
| `unhandled-kind` | 1 | 1 | An item property. |
| `missing-data` | 0 | 1 | A passive the bundle lacks. |

The field-specific reasons (each on its own fields only) are listed in each step's document:
- `docs/phase-6/recovery.md`: costs, regeneration, leech, the defence estimate's markers `EnemyHitReason`, `PoolReason`, `DotTakenReason`, `EhpReason`;
- `docs/phase-6/triggers.md`, `mirages.md`, `minions.md`, `gems.md`.

## Performance (P6-12: the budgets re-checked)

`loom-bench wall --runs 20` on `local/p6/packs/b-ot` with `forged-frostbearer-spectre-necromancer.xml` (release build, 2026-11-18):

| Scenario | Median | Slowest | Phase 5 record |
|---|---|---|---|
| `validate` | 607.1 ms | 625.2 ms | 585.0 ms |
| `load` | 651.0 ms | 677.2 ms | 726.2 ms |
| `start` | 780.7 ms | 800.3 ms | 994.4 ms |
| `import` | 18.0 ms | 20.7 ms | 28.8 ms |
| `calculate` | 64.9 ms | 69.3 ms | 6.7 ms |
| `allocate` | 0.6 ms | 0.9 ms | 1.0 ms |

**Every budget is still met** (ADR-013, reference tier):
- import and first calculation, 83 ms of 1.5 s;
- an item-edit preview, 65 ms of 300 ms;
- an allocation preview, 66 ms of 150 ms.

**`calculate` is ten times slower.** Full DPS (P6-12 part 2) computes each skill included in it as the main skill: this build has several. The defence estimate's hits-to-die adds a little.
- **The CI instruction-count check** (`tools/bench/baseline.json`, measured on CI's runner) will report the rise. The baseline needs a new record from that job, as PR #135 did for the tree's bench bundle: a maintainer item.
- **Caching** a skill's sub-calculation across previews is the obvious later step, if the preview budgets tighten.

## Done in Phase 6

Each step's record is in the tasks file (`dev/active/phase-6-mechanics/phase-6-mechanics-tasks.md`) and its document in `docs/phase-6/`. In brief:
- **P6-1 to P6-6:** damage over time, ailments, dual wielding, flasks, channelling, Vaal skills, totems, traps, mines, brands.
- **P6-7:** minions: data, the actor, hits, ailments, combined DPS.
- **P6-8:** triggers and the mirage.
- **P6-9:** gems: auras, curses, marks, guards, warcries, heralds, summons.
- **P6-10a:** the timeless extraction roots.
- **P6-10b:**
  - regeneration, recharge, movement, costs (mana, life, energy shield, per second);
  - base damage reduction;
  - the leech caps and rates;
  - damage taken over time;
  - the evade chances;
  - the enemy's hit;
  - maximum hit;
  - the effective health pool.
- **P6-11:** the stat mop-up batches a to ac (`docs/phase-6/stat-mop-up.md`). Since P6-11n:
  - P6-11o to P6-11s: lines while an aura affects you, Onslaught on kill, socketed gem levels, supports an item grants, and damage per aura or herald;
  - P6-11t: regeneration per charge and per stat, and channelling mana costs;
  - P6-11u: durations, projectile counts, and chains as `NoEffect` with guards;
  - P6-11v: nearby enemies;
  - P6-11w: burning ground;
  - P6-11x: Flesh and Stone's Sand Stance, and fortification rounded with the other hit MOREs;
  - P6-11y: Momentum's Swiftness, and the main skill's support buffs left to `poe1::gems`;
  - P6-11z: Arctic Armour;
  - P6-11aa: Infused Channelling;
  - P6-11ab: Rapid Decay and Swift Affliction;
  - P6-11ac: the eldritch block implicit.
- **P6-12:** combined and full DPS, and this report.

## Still open

**On this branch** (no maintainer needed):
- **The defence estimate:**
  - ward, aegis, a type's own guard, allies' pools, life loss prevention, and recoup in the pools (the shared guard since P6-11ae, the low life and low energy shield checks since P6-11ad, gains on block since P6-11ak);
  - damage over time EHP;
  - conversion smoothing in maximum hit.
- **A dual wielder's** ailments and leech.
- **The remaining gems** (P6-9) and config inputs (P6-8a's Spellslinger, Arcanist Brand, Mark on Hit, Hextouch, Cast on Death, item triggers; General's Cry's mirages; Sacred Wisps).
- **The minion's** damage over time and spectres (P6-7c3).
- **The P6-11 tail:** 929 distinct stats, including the intended field markers.
- **Repeats' other modes:** `FINAL` and `FINAL_DPS` (P6-11aj modelled the default `AVERAGE` and `NONE`).

**Maintainer items:**
- **Data runs:**
  - the timeless jewel data run, and Q-P6-1 (the seed algorithm's source: the known reference is GPL);
  - the cluster jewel tables in a bundle;
  - `VaalSouls` for soul costs;
  - the relic's Nova skill level requirements.
- **Confirmation in game:** Vaal Summon Skeletons' minion level.
- **The bench baseline** after this PR's `calculate` rise.
- **Shipping** `poe1/minions.json` and `poe1/flasks.json` in the signed bundle.
- **The gate run itself**, as the maintainer asked: "We can do gate testing after."
