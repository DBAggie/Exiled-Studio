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

**Not modelled:**
- A skill whose hit is not computed: the field is unsupported. Damage over time skills such as Righteous Fire are among these, where PoB adds 0 for the hit.
- Mirages (P6-8b), decay, and critical culling.
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
