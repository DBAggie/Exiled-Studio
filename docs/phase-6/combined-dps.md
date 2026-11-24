# P6-12: combined damage per second

P6-12 adds `combined-dps` (PoB's `CombinedDPS`) and ends with Phase 6's report. Part 1 is the field; the report follows once the remaining steps land.

## Part 1: the field

The new field is `combined-dps` (slice revision 10, applies present, group `offence`). It is a `Poe1Field`, as impale DPS is, so `Field::for_minion` stays total. The engine emits it whenever the build has a main skill.

**The calculation** (`loom_calc::poe1::combined`, phase `combined` after `costs`; `CalcOffence.lua` 5982 to 6139):
- **Sum:** the hit's DPS, plus impale's, plus `min(damage over time + poison + ignite + bleeding, 35791394)` (PoB's `DotDpsCap`, `Data.lua` 255).
- **Cull multiplier:** Culling Strike's `100 / (100 - 10)` (3217 to 3221).
  - Culling Strike's stat, `kill_enemy_on_hit_if_under_10%_life`, was a `NoEffect` row. It is now `Poe1Quantity::CullingStrike`, a flag queried in the skill's context, so a supported skill's own Culling Strike counts.
- **Dependencies:** each component the build has. An unsupported one leaves the field unsupported, and a partial one makes it partial.
- **Reservation multiplier:** PoB's enemy life reservation multiplier is 1 here; its stats stay reasons.

**A skill without a hit** (no `Damage`, `Attack`, or `Projectile` type, as the hit phase reads a spell; `combined::has_hit`) adds PoB's 0 for the hit. Examples are Righteous Fire and the summoning skills.

**Not modelled:**
- A hitting skill whose hit is not computed: the field is unsupported.
- Decay and critical culling. Mirage Archer is in since P6-8b (`docs/phase-6/mirages.md`).
- Skills that show their average damage (`base_skill_show_average_damage_instead_of_dps`, a reason).
- Ground damage over time and corrupting blood: the damage over time field is unsupported for them, and so is this one.

**Found while checking: a flag stat at 0.**
- A description of several stats resolves its line with the others at 0. "Attacks have #% chance to cause Bleeding" is the variant of `bleed_on_hit_with_attacks_%`, `global_bleed_on_hit`, and `cannot_cause_bleeding` where the last is 0.
- The engine added `CannotInflict(Bleed)` from that 0, a `Flag` row (P6-2c). So `smite-bleed-templar.xml`'s bleeding was never computed in the evidence runs, though its fixture's description says it matched PoB.
- `Ingest` now adds no flag effect for a settled value of 0 (a unit test). The bleeding is back, equal to PoB but for the attack rate (Q11).

**Register:** DIV-0133 to DIV-0173, 41 Q11 `game-truth` entries.
- 40 are attacks' `CombinedDPS` and one is `smite-bleed-templar.xml`'s `BleedDPS`. Each is the build's registered `TotalDPS` difference from the attack rate, carried through, and the register tool checks that entry exists first.
- In the Inquisitor builds Culling Strike scales it: act 10's −45.59 hit DPS difference is −51.11 combined, which is −45.59 × 10/9 plus ignite's −0.41.

**Evidence** (`local/p6/p6-12a/`):
- **Public `calc-parity`:** 195 builds, 172 expected differences, 0 unexpected; `Complete` fields 5,142.
  - `CombinedDPS` equals PoB's in 81 builds and differs only by the registered attack rate in 40.
  - It is partial in 13, indeterminate in 4, and not computed in 56.
  - Two builds gained a bleeding field (`smite-bleed-templar.xml`, complete; `lacerate-bleed-gladiator.xml`, partial). No other value moved.
- **Corpus:** 0 unexpected, no value moved, no state changed.
- **Tests:** the flag at 0 (`ingest`); the slice's names. Workspace tests, clippy, and the real-bundle tests pass.

**The skill without a hit** (a follow-up commit; `local/p6/p6-12b/`):
- `CombinedDPS` now equals PoB's in 97 builds: both Righteous Fire fixtures and the summoning skills' builds (PoB's 0) joined.
- 0 unexpected; no other value moved; `Complete` fields 5,158. A unit test covers `has_hit`.


## Part 2: full damage per second

**What PoB does** (`Modules/Calcs.lua` 176 to 386, `calcFullDPS`): each active skill of a group marked `includeInFullDPS` is computed as the main skill, with its count (the group's `groupCount`, else the gem's `count`, else 1). `FullDPS` then adds up, in order:
- each skill's `TotalDPS` times its count, and its minion's `TotalDPS` times its count;
- the largest ignite and the largest bleeding;
- poison and impale, each times the count;
- damage over time (times the count when it stacks);
- the result times the largest cull multiplier.

**The engine** (`poe1::full_dps`, phase `full-dps`; slice revision 14):
- **Each included skill** is computed with `calculate` on a copy of the build with that skill as the main one. The copy is marked `CalcOptions::nested`, so it computes no full DPS of its own. Trigger sources and mirage copies are nested too.
- **The field:** `full-dps`. A build without a skill set has none.
- **The cull multiplier** is read from each skill's combined DPS over its parts.
- **Reasons:** each field read carries its own. A hitting skill whose hit is not computed is `full-dps-skill`. A count above 1 with damage over time is `full-dps-dot-stack` (`DotCanStack` is not known). A Vaal skill is `full-dps-vaal`: PoB counts a Vaal gem's two effects by its `enableGlobal` flags, which the engine does not read yet.
- **Not modelled:** mirages' own share (PoB adds the mirage's output separately), ground damage over time, corrupting blood, decay, and the skills PoB counts once by a config condition.
- **Performance:** each included skill is a full calculation. `calc-run`'s total for the 205 public builds went from 0.35 s to 1.26 s, about 6 ms a build (the Phase 5 budgets are re-checked in the report).

### Evidence

- **Public `calc-parity` on `b-ot`:** 205 builds, 271 expected, 0 unexpected.
  - `FullDPS` equals PoB's in 97 builds.
  - 40 carry the registered Q11 attack rate: DIV-0233 to DIV-0272.
    - Each is a build whose `Speed` is registered.
    - `trigger-comk-no-kill-templar.xml` is the exception: its Ice Nova is self-cast and equal, but the group's Smite counts in full DPS with the Q11 rate.
  - 38 are `partial` or `indeterminate`, with their reasons.
- **Corpus:** 0 unexpected.
- **Tests:** the phase order; the workspace's 1,763 tests and clippy pass. The goldens are unchanged, as the synthetic build has no skill set.
